<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  export let role: string = 'user';
  export let content: string = '';
  export let time: string = '';

  let replaying = false;

  async function handleReplay() {
    if (replaying) return;
    replaying = true;
    try {
      await invoke('speak', { text: content });
    } catch (e) {
      console.error('[replay] failed:', e);
    } finally {
      replaying = false;
    }
  }
</script>

<div class="bubble-wrap {role}" class:user={role === 'user'} class:assistant={role === 'assistant'}>
  <div class="bubble">
    <p>{content}</p>
    {#if role === 'assistant'}
      <button
        class="replay"
        class:active={replaying}
        on:click={handleReplay}
        title="重新播放"
        aria-label="重新播放"
      >
        {#if replaying}
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 1 0 9-9" />
          </svg>
        {:else}
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
        {/if}
      </button>
    {/if}
  </div>
  <span class="time">{time}</span>
</div>

<style>
  .bubble-wrap {
    display: flex;
    flex-direction: column;
    gap: 4px;
    animation: bubble-in 0.25s ease-out;
    max-width: 85%;
  }

  .bubble-wrap.user {
    align-self: flex-end;
    align-items: flex-end;
  }

  .bubble-wrap.assistant {
    align-self: flex-start;
    align-items: flex-start;
  }

  @keyframes bubble-in {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .bubble {
    padding: 10px 14px;
    border-radius: 16px;
    font-size: 14px;
    line-height: 1.5;
    word-break: break-word;
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }

  .user .bubble {
    background: var(--primary);
    color: white;
    border-bottom-right-radius: 4px;
  }

  .assistant .bubble {
    background: rgba(99, 102, 241, 0.15);
    border: 1px solid rgba(99, 102, 241, 0.2);
    border-bottom-left-radius: 4px;
    color: var(--text-primary);
  }

  p {
    margin: 0;
    flex: 1;
  }

  .time {
    font-size: 10px;
    color: var(--text-secondary);
    opacity: 0.5;
    padding: 0 4px;
  }

  .replay {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: none;
    background: rgba(99, 102, 241, 0.2);
    color: var(--primary);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: background 0.15s, transform 0.15s;
  }

  .replay:hover {
    background: rgba(99, 102, 241, 0.35);
    transform: scale(1.08);
  }

  .replay:active {
    transform: scale(0.95);
  }

  .replay.active svg {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
