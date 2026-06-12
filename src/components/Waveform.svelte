<script lang="ts">
  export let level: number = 0; // 0.0 - 1.0

  const BAR_COUNT = 28;
  $: bars = Array.from({ length: BAR_COUNT }, (_, i) => {
    const position = i / BAR_COUNT; // 0 to 1
    // Create a wave pattern that shifts based on level
    const wave = Math.sin((position * Math.PI) + (Date.now() / 200));
    const base = 0.1;
    return Math.min(1, base + wave * level + level * 0.5);
  });
</script>

<div class="waveform">
  {#each bars as bar, i}
    <div
      class="bar"
      style="height: {Math.max(4, bar * 100)}%; animation-delay: {i * 20}ms;"
    ></div>
  {/each}
</div>

<style>
  .waveform {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 8px 0;
  }

  .bar {
    flex: 1;
    min-width: 4px;
    border-radius: 2px;
    background: linear-gradient(to top, var(--accent), var(--primary));
    opacity: 0.8;
    animation: wave-flow 0.8s ease-in-out infinite alternate;
    transition: height 0.1s ease;
  }

  @keyframes wave-flow {
    from { transform: scaleY(0.4); opacity: 0.5; }
    to { transform: scaleY(1); opacity: 1; }
  }
</style>