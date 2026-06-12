<script lang="ts">
  export let status: string = 'idle';
  export let text: string = '';
</script>

<div class="indicator {status}">
  <div class="dot"></div>
  <span class="text">{text}</span>
</div>

<style>
  .indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 20px;
    background: rgba(255,255,255,0.06);
    border: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition: background 0.3s;
    flex-shrink: 0;
  }

  .indicator.recording .dot {
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }

  .indicator.processing .dot {
    background: var(--primary);
    animation: spin 1s linear infinite;
  }

  .indicator.speaking .dot {
    background: var(--success);
    animation: pulse 0.8s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .text {
    font-weight: 500;
    white-space: nowrap;
  }
</style>