<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let status: 'idle' | 'recording' | 'processing' | 'speaking' = 'idle';

  const dispatch = createEventDispatcher<{ start: void; stop: void }>();

  let pressed = false;
  let pressedAt = 0;

  function handlePointerDown(e: PointerEvent) {
    if (status !== 'idle') return;
    e.preventDefault();
    pressed = true;
    pressedAt = Date.now();
    dispatch('start');
  }

  function handlePointerUp(e: PointerEvent) {
    if (!pressed) return;
    e.preventDefault();
    pressed = false;
    const held = Date.now() - pressedAt;
    if (held < 400) {
      // held too briefly — ffmpeg hasn't captured anything yet
      setTimeout(() => dispatch('stop'), 400 - held);
    } else {
      dispatch('stop');
    }
  }

  function handlePointerLeave() {
    if (pressed && status === 'recording') {
      pressed = false;
      dispatch('stop');
    }
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="record-btn"
  class:recording={status === 'recording'}
  class:processing={status === 'processing' || status === 'speaking'}
  class:pressed
  on:pointerdown={handlePointerDown}
  on:pointerup={handlePointerUp}
  on:pointerleave={handlePointerLeave}
  role="button"
  tabindex="0"
  aria-label="按住说话"
>
  <div class="btn-ring">
    <div class="btn-inner">
      {#if status === 'recording'}
        <div class="mic-icon recording">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2a3 3 0 0 1 3 3v7a3 3 0 0 1-6 0V5a3 3 0 0 1 3-3z"/>
            <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
            <line x1="12" y1="19" x2="12" y2="22"/>
          </svg>
        </div>
      {:else if status === 'processing'}
        <div class="spinner"></div>
      {:else}
        <div class="mic-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2a3 3 0 0 1 3 3v7a3 3 0 0 1-6 0V5a3 3 0 0 1 3-3z"/>
            <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
            <line x1="12" y1="19" x2="12" y2="22"/>
          </svg>
        </div>
      {/if}
    </div>
  </div>
  <span class="btn-label">
    {#if status === 'recording'}松开发送
    {:else if status === 'processing'}处理中...
    {:else if status === 'speaking'}播报中...
    {:else}按住说话
    {/if}
  </span>
</div>

<style>
  .record-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
    outline: none;
  }

  .btn-ring {
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: rgba(99, 102, 241, 0.15);
    border: 2px solid rgba(99, 102, 241, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
    position: relative;
  }

  .record-btn.recording .btn-ring {
    border-color: var(--accent);
    background: rgba(34, 211, 238, 0.15);
    box-shadow: 0 0 20px rgba(34, 211, 238, 0.3);
    animation: ring-pulse 1.5s ease-in-out infinite;
  }

  .record-btn.processing .btn-ring {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .record-btn.pressed .btn-ring {
    transform: scale(0.92);
    border-color: var(--accent);
    background: rgba(34, 211, 238, 0.2);
  }

  @keyframes ring-pulse {
    0%, 100% { box-shadow: 0 0 20px rgba(34, 211, 238, 0.3); }
    50% { box-shadow: 0 0 35px rgba(34, 211, 238, 0.5); }
  }

  .btn-inner {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--primary);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .record-btn.recording .btn-inner {
    background: var(--accent);
  }

  .mic-icon {
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .mic-icon.recording {
    animation: mic-bounce 0.6s ease-in-out infinite;
  }

  @keyframes mic-bounce {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.1); }
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .btn-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: color 0.2s;
  }

  .record-btn.recording .btn-label {
    color: var(--accent);
  }
</style>