<script lang="ts">
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  // Props
  let { isRecording = false, isPaused = false }: { isRecording?: boolean; isPaused?: boolean } = $props();

  // State
  let audioLevel = $state(0); // 0-1 range
  let unlistenAudioLevel: UnlistenFn | null = null;
  let pollInterval: number | null = null;

  // Number of bars to display
  const barCount = 5;

  onMount(async () => {
    // Subscribe to audio level events
    unlistenAudioLevel = await listen<{ level: number }>("audio-level", (event) => {
      if (isRecording && !isPaused) {
        audioLevel = event.payload.level;
      }
    });

    // Also poll for audio level as fallback
    pollInterval = window.setInterval(async () => {
      if (isRecording && !isPaused) {
        try {
          const level = await invoke<number>("get_audio_level");
          audioLevel = level;
        } catch (e) {
          console.warn("Failed to get audio level:", e);
        }
      }
    }, 100);
  });

  onDestroy(() => {
    if (unlistenAudioLevel) {
      unlistenAudioLevel();
    }
    if (pollInterval) {
      clearInterval(pollInterval);
    }
  });

  // Reset level when not recording
  $effect(() => {
    if (!isRecording || isPaused) {
      audioLevel = 0;
    }
  });

  // Generate bar heights based on audio level (0-100%)
  function getBarHeight(index: number): number {
    if (!isRecording || isPaused) return 20;
    
    // Create a more natural look with varying heights
    const baseHeight = 20;
    const maxExtra = 80;
    
    // Each bar has a slightly different response
    const factor = 1 - (Math.abs(index - Math.floor(barCount / 2)) * 0.15);
    const level = audioLevel * factor;
    
    return Math.min(100, baseHeight + (level * maxExtra));
  }
</script>

<div 
  class="audio-level-container {isPaused ? 'opacity-40' : ''}"
  aria-label="Audio level indicator"
>
  {#each Array(barCount) as _, i}
    <div
      class="audio-bar"
      style="height: {getBarHeight(i)}%"
    ></div>
  {/each}
</div>

<style>
  .audio-level-container {
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 2px;
    height: 20px;
    min-width: 24px;
  }

  .audio-bar {
    width: 3px;
    min-height: 4px;
    background-color: var(--phantom-ear-danger);
    border-radius: 2px;
    transition: height 0.05s ease-out;
  }
</style>
