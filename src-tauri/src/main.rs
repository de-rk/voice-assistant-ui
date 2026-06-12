// VoiceAssistant — macOS MenuBar App
// Pipeline: cpal PCM → whisper-rs STT → LM Studio/OpenAI → Noiz TTS
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::info;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use tauri::{AppHandle, Emitter, Manager, State};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String, // "local" | "openai" | "custom"
    pub base_url: String,
    pub model:    String,
    pub api_key:  String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub api_key:  String,
    pub voice_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm: LlmConfig,
    pub tts: TtsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            llm: LlmConfig {
                provider: "local".to_string(),
                base_url: "http://127.0.0.1:1234/v1".to_string(),
                model:    "google/gemma-4-e2b".to_string(),
                api_key:  "lm-studio".to_string(),
            },
            tts: TtsConfig {
                api_key:  String::new(),
                voice_id: "ad703a88".to_string(),
            },
        }
    }
}

fn load_config(path: &PathBuf) -> AppConfig {
    if path.exists() {
        if let Ok(json) = fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&json) {
                return cfg;
            }
        }
    }
    let default = AppConfig::default();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&default) {
        let _ = fs::write(path, json);
    }
    default
}

// ─── Whisper Model ────────────────────────────────────────────────────────────

static WHISPER_CTX: OnceLock<WhisperContext> = OnceLock::new();

const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin";

async fn ensure_model(model_path: &PathBuf, app: &AppHandle) -> Result<(), String> {
    if model_path.exists() {
        return Ok(());
    }

    info!("[whisper] downloading model to {:?}", model_path);
    let _ = app.emit("status-changed", "downloading-model");

    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let bytes = client
        .get(MODEL_URL)
        .send()
        .await
        .map_err(|e| format!("model download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("model read failed: {}", e))?;

    fs::write(model_path, &bytes).map_err(|e| format!("model write failed: {}", e))?;
    info!("[whisper] model downloaded ({} bytes)", bytes.len());
    Ok(())
}

// Resample mono f32 PCM from src_rate to 16000 Hz (linear interpolation)
fn resample_to_16k(samples: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == 16000 {
        return samples.to_vec();
    }
    let ratio = src_rate as f64 / 16000.0;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = pos - idx as f64;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push((a + (b - a) * frac as f32) as f32);
    }
    out
}

// ─── macOS Microphone Permission ──────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn mic_auth_status() -> i64 {
    use objc::{class, msg_send, sel, sel_impl};
    use objc::runtime::Object;
    unsafe {
        let av_cls = class!(AVCaptureDevice);
        let ns_cls = class!(NSString);
        let media_type: *mut Object = msg_send![ns_cls,
            stringWithUTF8String: b"soun\0".as_ptr() as *const std::os::raw::c_char];
        msg_send![av_cls, authorizationStatusForMediaType: media_type]
    }
}

#[cfg(target_os = "macos")]
fn ensure_mic_permission() {
    use std::sync::{Arc, Condvar, Mutex};
    use objc::{class, msg_send, sel, sel_impl};
    use objc::runtime::Object;

    // 0=notDetermined 1=restricted 2=denied 3=authorized
    let status = mic_auth_status();
    info!("[mic] auth status: {}", status);

    match status {
        3 => return,
        2 => {
            info!("[mic] permission denied — user must enable in System Settings > Privacy > Microphone");
            return;
        }
        _ => {}
    }

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();

    unsafe {
        let av_cls = class!(AVCaptureDevice);
        let ns_cls = class!(NSString);
        let media_type: *mut Object = msg_send![ns_cls,
            stringWithUTF8String: b"soun\0".as_ptr() as *const std::os::raw::c_char];

        let block = block::ConcreteBlock::new(move |granted: bool| {
            info!("[mic] permission granted: {}", granted);
            let (lock, cvar) = &*pair2;
            *lock.lock().unwrap() = true;
            cvar.notify_one();
        }).copy();

        let _: () = msg_send![av_cls,
            requestAccessForMediaType: media_type
            completionHandler: &*block];

        let (lock, cvar) = &*pair;
        let mut done = lock.lock().unwrap();
        while !*done {
            done = cvar.wait(done).unwrap();
        }
    }
    info!("[mic] permission request complete, status now: {}", mic_auth_status());
}

// ─── App State ────────────────────────────────────────────────────────────────

pub struct AppState {
    pub recording:    Mutex<bool>,
    pub status:       Mutex<String>,
    pub messages:     Mutex<Vec<ChatMessage>>,
    pub audio_chunks: Arc<Mutex<Vec<u8>>>,
    pub stop_flag:    Mutex<Option<Arc<AtomicBool>>>,
    pub sample_rate:  Arc<Mutex<u32>>,
    pub config:       Mutex<AppConfig>,
    pub config_path:  Mutex<PathBuf>,
    pub model_path:   PathBuf,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role:      String,
    pub content:   String,
    pub timestamp: i64,
}

// ─── Config Commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config.lock().clone()
}

#[tauri::command]
fn save_config(config: AppConfig, state: State<'_, AppState>) -> Result<(), String> {
    let path = state.config_path.lock().clone();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    *state.config.lock() = config;
    info!("[config] saved to {:?}", path);
    Ok(())
}

#[tauri::command]
fn get_config_path(state: State<'_, AppState>) -> String {
    state.config_path.lock().to_string_lossy().to_string()
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn start_recording(state: State<'_, AppState>, app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    if mic_auth_status() != 3 {
        return Err("麦克风权限未授权，请在系统设置 → 隐私与安全性 → 麦克风中允许本应用".to_string());
    }

    {
        let mut rec = state.recording.lock();
        if *rec { return Ok(true); }
        *rec = true;
    }
    *state.status.lock() = "recording".to_string();
    let _ = app.emit("status-changed", "recording");
    state.audio_chunks.lock().clear();

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_child = stop_flag.clone();
    let chunks = state.audio_chunks.clone();
    let sample_rate_arc = state.sample_rate.clone();
    *state.stop_flag.lock() = Some(stop_flag);

    thread::spawn(move || {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => { info!("[audio] no input device"); return; }
        };
        info!("[audio] device: {}", device.name().unwrap_or_default());

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => { info!("[audio] default_input_config failed: {}", e); return; }
        };
        let sr = config.sample_rate().0;
        let ch = config.channels();
        info!("[audio] config: {}Hz {}ch {:?}", sr, ch, config.sample_format());
        *sample_rate_arc.lock() = sr;

        let chunks_cb = chunks.clone();
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                let step = ch as usize;
                let bytes: Vec<u8> = data.chunks(step)
                    .flat_map(|frame| {
                        let mono = frame.iter().map(|&s| s as f64).sum::<f64>() / step as f64;
                        let s = (mono.clamp(-1.0, 1.0) * 32767.0) as i16;
                        s.to_le_bytes()
                    })
                    .collect();
                chunks_cb.lock().extend_from_slice(&bytes);
            },
            |e| info!("[audio] stream error: {}", e),
            None,
        );

        let stream = match stream {
            Ok(s) => s,
            Err(e) => { info!("[audio] build_input_stream failed: {}", e); return; }
        };

        if let Err(e) = stream.play() {
            info!("[audio] stream.play failed: {}", e);
            return;
        }

        info!("[audio] cpal stream started");
        while !stop_flag_child.load(Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_millis(50));
        }
        drop(stream);
        info!("[audio] cpal stream stopped");
    });

    info!("[audio] Recording started");
    Ok(true)
}

#[tauri::command]
async fn stop_recording(state: State<'_, AppState>, app: AppHandle) -> Result<String, String> {
    {
        let mut rec = state.recording.lock();
        if !*rec { return Err("not recording".to_string()); }
        *rec = false;
    }
    *state.status.lock() = "processing".to_string();
    let _ = app.emit("status-changed", "processing");

    if let Some(flag) = state.stop_flag.lock().take() {
        flag.store(true, Ordering::Relaxed);
    }
    thread::sleep(std::time::Duration::from_millis(400));

    let bytes = state.audio_chunks.lock().len();
    info!("[audio] Stopped — {bytes} bytes");
    Ok("ok".to_string())
}

#[tauri::command]
async fn transcribe(state: State<'_, AppState>, app: AppHandle) -> Result<String, String> {
    let chunks = state.audio_chunks.lock().clone();
    if chunks.is_empty() {
        return Err("no audio data".to_string());
    }

    let sr = {
        let r = *state.sample_rate.lock();
        if r == 0 { 44100 } else { r }
    };
    state.audio_chunks.lock().clear();

    // i16 bytes → f32 samples
    let samples: Vec<f32> = chunks
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32767.0)
        .collect();

    // Resample to 16kHz
    let samples_16k = resample_to_16k(&samples, sr);

    // Download model on first use
    let model_path = state.model_path.clone();
    ensure_model(&model_path, &app).await?;

    let model_path_str = model_path
        .to_str()
        .ok_or("invalid model path")?
        .to_string();

    let text = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let ctx = WHISPER_CTX.get_or_init(|| {
            info!("[whisper] loading model from {}", model_path_str);
            WhisperContext::new_with_params(
                &model_path_str,
                WhisperContextParameters::default(),
            )
            .expect("failed to load whisper model")
        });

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("auto"));
        params.set_n_threads(4);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = ctx.create_state().map_err(|e| format!("whisper state: {}", e))?;
        state.full(params, &samples_16k).map_err(|e| format!("whisper full: {}", e))?;

        let n = state.full_n_segments().map_err(|e| format!("whisper segments: {}", e))?;
        let mut text = String::new();
        for i in 0..n {
            if let Ok(seg) = state.full_get_segment_text(i) {
                text.push_str(&seg);
            }
        }
        Ok(text.trim().to_string())
    })
    .await
    .map_err(|e| format!("spawn_blocking error: {}", e))??;

    info!("[stt] -> {:?}", text);
    Ok(text)
}

#[tauri::command]
async fn chat(text: String, state: State<'_, AppState>) -> Result<String, String> {
    let (base_url, model, api_key) = {
        let cfg = state.config.lock();
        (
            cfg.llm.base_url.trim_end_matches('/').to_string(),
            cfg.llm.model.clone(),
            cfg.llm.api_key.clone(),
        )
    };
    info!("[ai] base_url={}", base_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let body_str = serde_json::json!({
        "model":      model,
        "messages":   [{"role": "user", "content": text}],
        "max_tokens": 2048,
        "stream":     false
    }).to_string();

    let mut attempts = 0;
    let (_status, body) = loop {
        attempts += 1;
        let resp = client
            .post(format!("{base_url}/chat/completions"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body_str.clone())
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;
        let s = resp.status();
        let b = resp.text().await.map_err(|e| format!("read body error: {}", e))?;
        info!("[ai] HTTP {} (attempt {})", s, attempts);
        if s.as_u16() == 502 && attempts < 3 {
            thread::sleep(std::time::Duration::from_secs(2));
            continue;
        }
        break (s, b);
    };
    info!("[ai] body: {}", &body[..body.len().min(200)]);

    let data: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("JSON parse error: {} — body: {}", e, &body[..body.len().min(300)]))?;

    let choices = data["choices"].as_array().ok_or("no choices")?;
    let msg = &choices[0]["message"];

    let reply = msg["content"]
        .as_str()
        .or_else(|| msg["reasoning_content"].as_str())
        .unwrap_or("")
        .to_string();

    let reply = reply
        .split("Thinking Process:")
        .last()
        .unwrap_or(&reply)
        .trim()
        .to_string();

    info!("[ai] {} chars", reply.len());
    Ok(reply)
}

#[tauri::command]
async fn speak(text: String, app: AppHandle) -> Result<(), String> {
    *app.state::<AppState>().status.lock() = "speaking".to_string();
    let _ = app.emit("status-changed", "speaking");

    let (api_key, voice_id) = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock();
        (cfg.tts.api_key.clone(), cfg.tts.voice_id.clone())
    };

    let cache_dir = "/tmp/va_tts_cache";
    let _ = fs::create_dir_all(cache_dir);
    let cache_key = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        voice_id.hash(&mut h);
        text.hash(&mut h);
        format!("{:016x}", h.finish())
    };
    let cache_path = format!("{}/{}.wav", cache_dir, cache_key);

    if fs::metadata(&cache_path).map(|m| m.len() > 44).unwrap_or(false) {
        info!("[tts] cache hit: {}", cache_path);
        let _ = Command::new("afplay").arg(&cache_path).output();
        *app.state::<AppState>().status.lock() = "idle".to_string();
        let _ = app.emit("status-changed", "idle");
        return Ok(());
    }

    if api_key.is_empty() {
        info!("[tts] no api_key, using say");
        Command::new("say").args(["-v", "Tingting", "-r", "180", &text]).output().ok();
        *app.state::<AppState>().status.lock() = "idle".to_string();
        let _ = app.emit("status-changed", "idle");
        return Ok(());
    }

    info!("[tts] calling Noiz");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .text("text", text.clone())
        .text("voice_id", voice_id)
        .text("output_format", "wav")
        .text("speed", "1")
        .text("trim_silence", "true");

    let resp = client
        .post("https://noiz.ai/v1/text-to-speech")
        .header("Authorization", api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Noiz TTS HTTP error: {}", e))?;

    let http_status = resp.status();
    info!("[tts] HTTP {}", http_status);

    if !http_status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        info!("[tts] Noiz failed — falling back to say. body: {}", &body[..body.len().min(300)]);
        Command::new("say").args(["-v", "Tingting", "-r", "180", &text]).output().ok();
    } else {
        let audio_bytes = resp.bytes().await.map_err(|e| format!("read audio error: {}", e))?;
        info!("[tts] received {} bytes", audio_bytes.len());

        if audio_bytes.len() < 44 || !audio_bytes.starts_with(b"RIFF") {
            info!("[tts] not valid WAV — falling back to say");
            Command::new("say").args(["-v", "Tingting", "-r", "180", &text]).output().ok();
        } else {
            fs::write(&cache_path, &audio_bytes).map_err(|e| format!("write audio error: {}", e))?;
            Command::new("afplay").arg(&cache_path).output().map_err(|e| e.to_string())?;
        }
    }

    *app.state::<AppState>().status.lock() = "idle".to_string();
    let _ = app.emit("status-changed", "idle");
    info!("[tts] done");
    Ok(())
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> String {
    state.status.lock().clone()
}

#[tauri::command]
fn add_message(msg: ChatMessage, state: State<'_, AppState>) {
    state.messages.lock().push(msg);
}

#[tauri::command]
fn get_messages(state: State<'_, AppState>) -> Vec<ChatMessage> {
    state.messages.lock().clone()
}

#[tauri::command]
fn clear_messages(state: State<'_, AppState>) {
    state.messages.lock().clear();
}

// ─── Main Entry ───────────────────────────────────────────────────────────────

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let base_dir = PathBuf::from(&home).join(".voice-assistant");
    let config_path = base_dir.join("config.json");
    let model_path  = base_dir.join("models").join("ggml-small.bin");

    let config = load_config(&config_path);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            recording:    Mutex::new(false),
            status:       Mutex::new("idle".to_string()),
            messages:     Mutex::new(Vec::new()),
            audio_chunks: Arc::new(Mutex::new(Vec::new())),
            stop_flag:    Mutex::new(None),
            sample_rate:  Arc::new(Mutex::new(0u32)),
            config:       Mutex::new(config),
            config_path:  Mutex::new(config_path),
            model_path,
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                ensure_mic_permission();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.set_always_on_top(true);
                    let _ = win.set_skip_taskbar(true);
                    let _ = win.show();
                }
            }
            info!("VoiceAssistant ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            transcribe,
            chat,
            speak,
            get_status,
            add_message,
            get_messages,
            clear_messages,
            get_config,
            save_config,
            get_config_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
