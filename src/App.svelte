<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import RecordButton from './components/RecordButton.svelte';
  import ConversationBubble from './components/ConversationBubble.svelte';
  import Waveform from './components/Waveform.svelte';
  import StatusIndicator from './components/StatusIndicator.svelte';

  // ── Types ─────────────────────────────────────────────────────────────────
  interface LlmConfig {
    provider: 'local' | 'openai' | 'custom';
    base_url: string;
    model: string;
    api_key: string;
  }

  interface TtsConfig {
    api_key: string;
    voice_id: string;
  }

  interface AppConfig {
    llm: LlmConfig;
    tts: TtsConfig;
  }

  const PROVIDER_URLS: Record<string, string> = {
    local:  'http://127.0.0.1:1234/v1',
    openai: 'https://api.openai.com/v1',
    custom: '',
  };

  const PROVIDER_MODELS: Record<string, string> = {
    local:  'google/gemma-4-e2b',
    openai: 'gpt-4o',
    custom: '',
  };

  // ── State ──────────────────────────────────────────────────────────────────
  let status: 'idle' | 'recording' | 'processing' | 'speaking' = 'idle';
  let audioLevel = 0;
  let messages: Array<{ role: string; content: string; time: string }> = [];
  let statusText = '就绪';
  let recordingTimer: ReturnType<typeof setInterval> | null = null;

  let config: AppConfig | null = null;
  let showSettings = false;
  let draft: AppConfig | null = null;
  let configPath = '';
  let saveError = '';

  // ── Lifecycle ─────────────────────────────────────────────────────────────
  onMount(async () => {
    config = await invoke<AppConfig>('get_config');
    configPath = await invoke<string>('get_config_path');

    const unlisten = await listen<string>('status-changed', (e) => {
      status = e.payload as typeof status;
      statusText = {
        idle:       '就绪',
        recording:  '录音中...',
        processing: '处理中...',
        speaking:   '播报中...',
      }[status] ?? status;
    });

    function onKeyDown(e: KeyboardEvent) {
      if (e.code !== 'Space' || e.repeat) return;
      if (status !== 'idle' || showSettings) return;
      e.preventDefault();
      handleRecordStart();
    }
    function onKeyUp(e: KeyboardEvent) {
      if (e.code !== 'Space') return;
      if (status !== 'recording') return;
      e.preventDefault();
      handleRecordStop();
    }
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);

    return () => {
      unlisten();
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
    };
  });

  // ── Settings ──────────────────────────────────────────────────────────────
  function openSettings() {
    draft = JSON.parse(JSON.stringify(config));
    saveError = '';
    showSettings = true;
  }

  function closeSettings() {
    showSettings = false;
    draft = null;
  }

  function onProviderChange() {
    if (!draft) return;
    const p = draft.llm.provider;
    if (p !== 'custom') {
      draft.llm.base_url = PROVIDER_URLS[p];
      if (!draft.llm.model || draft.llm.model === PROVIDER_MODELS[p === 'local' ? 'openai' : 'local']) {
        draft.llm.model = PROVIDER_MODELS[p];
      }
    }
  }

  async function saveSettings() {
    if (!draft) return;
    saveError = '';
    try {
      await invoke('save_config', { config: draft });
      config = JSON.parse(JSON.stringify(draft));
      closeSettings();
    } catch (e: any) {
      saveError = String(e);
    }
  }

  // ── Actions ───────────────────────────────────────────────────────────────
  async function handleRecordStart() {
    status = 'recording';
    statusText = '录音中...';
    audioLevel = 0;
    await invoke('start_recording');
    recordingTimer = setInterval(() => {
      audioLevel = 0.2 + Math.random() * 0.7;
    }, 150);
  }

  async function handleRecordStop() {
    if (recordingTimer) clearInterval(recordingTimer);
    recordingTimer = null;
    audioLevel = 0;

    status = 'processing';
    statusText = '转写中...';

    await invoke('stop_recording');

    try {
      const transcript = await invoke<string>('transcribe');
      if (!transcript.trim()) {
        status = 'idle';
        statusText = '未检测到语音';
        return;
      }

      addMessage('user', transcript);
      statusText = '思考中...';

      const reply = await invoke<string>('chat', { text: transcript });

      addMessage('assistant', reply);
      status = 'speaking';
      statusText = '播报中...';

      await invoke('speak', { text: reply });

      status = 'idle';
      statusText = '就绪';
    } catch (err: any) {
      console.error('[app] error:', err);
      status = 'idle';
      statusText = `错误: ${err}`;
    }
  }

  function addMessage(role: string, content: string) {
    const now = new Date();
    const time = `${now.getHours().toString().padStart(2,'0')}:${now.getMinutes().toString().padStart(2,'0')}`;
    messages = [...messages, { role, content, time }];
  }
</script>

<!-- ── App Shell ─────────────────────────────────────────────────────────── -->
<div class="app">
  <!-- Title Bar -->
  <div class="titlebar" data-tauri-drag-region>
    <div class="titlebar-left">
      <span class="status-dot {status}"></span>
      <span class="titlebar-title">VoiceAssistant</span>
    </div>
    <div class="titlebar-right">
      <StatusIndicator {status} text={statusText} />
      <button class="settings-btn" on:click={openSettings} title="设置" aria-label="打开设置">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
      </button>
    </div>
  </div>

  <!-- Conversation -->
  <div class="conversation">
    {#if messages.length === 0}
      <div class="empty-state">
        <div class="empty-icon">🎙️</div>
        <p>按住下方按钮开始说话</p>
      </div>
    {/if}
    {#each messages as msg}
      <ConversationBubble role={msg.role} content={msg.content} time={msg.time} />
    {/each}
  </div>

  {#if status === 'recording'}
    <div class="waveform-container">
      <Waveform level={audioLevel} />
    </div>
  {/if}

  <div class="bottom-bar">
    <RecordButton
      {status}
      on:start={handleRecordStart}
      on:stop={handleRecordStop}
    />
  </div>

  <div class="status-bar">
    <span class="status-text">{statusText}</span>
  </div>
</div>

<!-- ── Settings Modal ────────────────────────────────────────────────────── -->
{#if showSettings && draft}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click|self={closeSettings}>
    <div class="modal" role="dialog" aria-modal="true" aria-label="设置">
      <div class="modal-header">
        <span class="modal-title">设置</span>
        <button class="close-btn" on:click={closeSettings} aria-label="关闭">✕</button>
      </div>

      <div class="modal-body">
        <!-- LLM -->
        <div class="section-label">大语言模型</div>

        <label class="field">
          <span>服务商</span>
          <select bind:value={draft.llm.provider} on:change={onProviderChange}>
            <option value="local">本地 (LM Studio / Ollama)</option>
            <option value="openai">OpenAI</option>
            <option value="custom">自定义</option>
          </select>
        </label>

        <label class="field">
          <span>Base URL</span>
          <input
            type="text"
            bind:value={draft.llm.base_url}
            placeholder="http://127.0.0.1:1234/v1"
            readonly={draft.llm.provider !== 'custom'}
            class:readonly={draft.llm.provider !== 'custom'}
          />
        </label>

        <label class="field">
          <span>模型</span>
          <input type="text" bind:value={draft.llm.model} placeholder="model name" />
        </label>

        <label class="field">
          <span>API Key</span>
          <input type="password" bind:value={draft.llm.api_key} placeholder="sk-..." />
        </label>

        <!-- TTS -->
        <div class="section-label" style="margin-top:16px;">语音合成 (Noiz TTS)</div>

        <label class="field">
          <span>API Key</span>
          <input type="password" bind:value={draft.tts.api_key} placeholder="留空则使用系统 say 命令" />
        </label>

        <label class="field">
          <span>Voice ID</span>
          <input type="text" bind:value={draft.tts.voice_id} placeholder="ad703a88" />
        </label>

        {#if configPath}
          <p class="config-path">配置文件：{configPath}</p>
        {/if}

        {#if saveError}
          <p class="save-error">{saveError}</p>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" on:click={closeSettings}>取消</button>
        <button class="btn-save" on:click={saveSettings}>保存</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .app {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-radius: 14px;
    overflow: hidden;
    border: 1px solid var(--border);
  }

  /* Title Bar */
  .titlebar {
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    background: rgba(20, 20, 40, 0.6);
    backdrop-filter: blur(20px);
    border-bottom: 1px solid var(--border);
    -webkit-app-region: drag;
    flex-shrink: 0;
  }

  .titlebar-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .titlebar-right {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition: background 0.3s;
  }
  .status-dot.recording  { background: var(--accent);   animation: pulse 1s infinite; }
  .status-dot.processing { background: var(--primary);  animation: spin 1s linear infinite; }
  .status-dot.speaking   { background: var(--success);  animation: pulse 0.8s infinite; }

  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
  @keyframes spin  { to { transform: rotate(360deg); } }

  .titlebar-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .settings-btn:hover {
    background: rgba(255,255,255,0.08);
    color: var(--text-primary);
  }

  /* Conversation */
  .conversation {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--text-secondary);
    font-size: 13px;
    opacity: 0.6;
  }

  .empty-icon { font-size: 36px; opacity: 0.4; }

  .waveform-container { height: 64px; padding: 0 16px; flex-shrink: 0; }

  .bottom-bar {
    padding: 12px 16px;
    flex-shrink: 0;
    display: flex;
    justify-content: center;
  }

  .status-bar {
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-top: 1px solid var(--border);
    background: rgba(15, 15, 30, 0.4);
    flex-shrink: 0;
  }

  .status-text { font-size: 11px; color: var(--text-secondary); opacity: 0.7; }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    border-radius: 14px;
  }

  .modal {
    width: 340px;
    background: #15152a;
    border: 1px solid var(--border);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px 12px;
    border-bottom: 1px solid var(--border);
  }

  .modal-title { font-size: 13px; font-weight: 600; color: var(--text-primary); }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 13px;
    padding: 2px 4px;
    border-radius: 4px;
    line-height: 1;
  }
  .close-btn:hover { color: var(--text-primary); background: rgba(255,255,255,0.06); }

  .modal-body { padding: 14px 16px; display: flex; flex-direction: column; gap: 10px; }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    margin-bottom: 2px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field span {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .field input,
  .field select {
    background: rgba(255,255,255,0.05);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 12px;
    padding: 6px 8px;
    outline: none;
    font-family: inherit;
    width: 100%;
    transition: border-color 0.15s;
  }
  .field input:focus,
  .field select:focus { border-color: var(--border-active); }

  .field input.readonly {
    color: var(--text-secondary);
    cursor: default;
  }

  .field select option { background: #15152a; }

  .config-path {
    font-size: 10px;
    color: var(--text-secondary);
    opacity: 0.5;
    word-break: break-all;
    margin-top: 4px;
  }

  .save-error {
    font-size: 11px;
    color: var(--error);
    margin-top: 4px;
  }

  .modal-footer {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }

  .btn-cancel, .btn-save {
    font-size: 12px;
    font-family: inherit;
    padding: 6px 14px;
    border-radius: 6px;
    border: none;
    cursor: pointer;
    font-weight: 500;
    transition: opacity 0.15s;
  }
  .btn-cancel {
    background: rgba(255,255,255,0.07);
    color: var(--text-secondary);
  }
  .btn-cancel:hover { opacity: 0.75; }
  .btn-save {
    background: var(--primary);
    color: #fff;
  }
  .btn-save:hover { opacity: 0.85; }
</style>
