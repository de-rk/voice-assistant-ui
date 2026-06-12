# VoiceAssistant

macOS 菜单栏语音助手，按住按钮说话，自动转写、AI 回答、语音播报。

![platform](https://img.shields.io/badge/platform-macOS-lightgrey)
![rust](https://img.shields.io/badge/rust-stable-orange)
![tauri](https://img.shields.io/badge/tauri-2.x-blue)

---

## 功能

- **语音转文字** — 内置 Whisper (small 模型)，离线运行，支持中英日混合
- **AI 对话** — 支持任意 OpenAI 兼容接口（本地 LM Studio、OpenAI、NVIDIA NIM、DeepSeek 等）
- **语音合成** — 支持 Noiz TTS，无 API Key 时自动降级为系统 `say` 命令
- **全局快捷键** — 空格键按住录音，松开自动处理
- **配置持久化** — 所有设置保存在 `~/.voice-assistant/config.json`

---

## 下载

在 [GitHub Actions](https://github.com/de-rk/voice-assistant-ui/actions) 的最新构建中下载 Artifacts：

- `VoiceAssistant-macOS-dmg` — 安装包
- `VoiceAssistant-macOS-app` — 直接运行的 .app

> 由于使用 ad-hoc 签名，首次打开时 macOS 会提示"无法验证开发者"，在**系统设置 → 隐私与安全性**中点击"仍要打开"即可。

---

## 配置

点击界面右上角齿轮图标打开设置，或直接编辑 `~/.voice-assistant/config.json`：

```json
{
  "llm": {
    "provider": "custom",
    "base_url": "https://integrate.api.nvidia.com/v1",
    "model": "deepseek-ai/deepseek-v4-flash",
    "api_key": "your-api-key"
  },
  "tts": {
    "api_key": "",
    "voice_id": ""
  }
}
```

### 大语言模型

| 服务商 | base_url | 示例模型 |
|--------|----------|----------|
| 本地 LM Studio | `http://127.0.0.1:1234/v1` | `google/gemma-4-e2b` |
| OpenAI | `https://api.openai.com/v1` | `gpt-4o` |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat` |
| NVIDIA NIM | `https://integrate.api.nvidia.com/v1` | `deepseek-ai/deepseek-v4-flash` |
| 硅基流动 | `https://api.siliconflow.cn/v1` | `deepseek-ai/DeepSeek-V3` |

### 语音合成

留空 API Key 则使用系统 `say` 命令（Tingting 语音）。使用 [Noiz TTS](https://noiz.ai) 需填入 API Key 和 Voice ID。

---

## 本地开发

**依赖：**

- Rust (stable)
- Node.js 20+
- cmake（编译 whisper.cpp 需要）
- Xcode Command Line Tools

```bash
# 安装依赖
brew install cmake
npm install --legacy-peer-deps

# 开发模式
npm run tauri dev

# 构建
npm run tauri build
```

首次运行 `transcribe` 时会自动下载 Whisper small 模型（~244MB）到 `~/.voice-assistant/models/`。

---

## 技术栈

- [Tauri 2](https://tauri.app) — 桌面应用框架
- [Svelte 5](https://svelte.dev) — 前端 UI
- [whisper-rs](https://github.com/tazz4843/whisper-rs) — Rust 原生语音识别
- [cpal](https://github.com/RustAudio/cpal) — 跨平台音频采集
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP 客户端
