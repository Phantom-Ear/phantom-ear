<script lang="ts">
  import type { TranscriptSegment } from "$lib/types";

  // Props
  let {
    segments = [],
    duration = 0,
    currentPosition = 0,
    isRecording = false,
    onSeek,
  }: {
    segments?: TranscriptSegment[];
    duration?: number;
    currentPosition?: number;
    isRecording?: boolean;
    onSeek?: (timestampMs: number) => void;
  } = $props();

  // Refs
  let timelineEl: HTMLDivElement | null = $state(null);

  // Generate pseudo-waveform bars based on segment activity
  const waveformBars = $derived.by(() => {
    const barCount = 60; // Number of bars in the waveform
    const bars: { height: number; hasSegment: boolean }[] = [];
    
    if (duration <= 0) {
      // Return flat bars when no duration
      for (let i = 0; i < barCount; i++) {
        bars.push({ height: 20, hasSegment: false });
      }
      return bars;
    }

    const barDuration = duration / barCount;
    
    for (let i = 0; i < barCount; i++) {
      const barStart = i * barDuration;
      const barEnd = (i + 1) * barDuration;
      
      // Check if any segment falls within this bar's time range
      const segmentsInBar = segments.filter(s => {
        const segStart = (s.timestamp_ms || 0) / 1000;
        return segStart >= barStart && segStart < barEnd;
      });
      
      // Generate height based on activity (segments) with some randomness for natural look
      let height: number;
      if (segmentsInBar.length > 0) {
        // More segments = higher bars, with some variation
        height = 40 + Math.min(segmentsInBar.length * 15, 40) + Math.random() * 20;
      } else {
        // Low ambient activity
        height = 15 + Math.random() * 15;
      }
      
      bars.push({ 
        height: Math.min(height, 100), 
        hasSegment: segmentsInBar.length > 0 
      });
    }
    
    return bars;
  });

  // Handle click to seek
  function handleClick(e: MouseEvent) {
    if (!timelineEl || !onSeek || duration <= 0) return;

    const rect = timelineEl.getBoundingClientRect();
    const clickPercent = (e.clientX - rect.left) / rect.width;
    const targetTime = clickPercent * duration * 1000; // in ms

    onSeek(targetTime);
  }

  // Format time for display
  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }
</script>

<div class="timeline-container">
  <!-- Time markers: only start and end -->
  <div class="time-markers">
    <span class="time-label">0:00</span>
    <span class="time-label">{formatTime(duration)}</span>
  </div>

  <!-- Waveform timeline -->
  <div 
    bind:this={timelineEl}
    class="waveform-track"
    onclick={handleClick}
    role="slider"
    aria-label="Transcript timeline"
    aria-valuemin="0"
    aria-valuemax={duration}
    aria-valuenow={currentPosition}
  >
    <!-- Waveform bars -->
    <div class="waveform-bars">
      {#each waveformBars as bar, i}
        <div 
          class="waveform-bar"
          class:has-segment={bar.hasSegment}
          style="height: {bar.height}%"
        ></div>
      {/each}
    </div>

    <!-- Playhead / Current position -->
    {#if duration > 0}
      <div 
        class="playhead"
        style="left: {(currentPosition / duration) * 100}%"
      ></div>
    {/if}

    <!-- Recording indicator -->
    {#if isRecording}
      <div class="recording-badge">
        <span class="recording-dot"></span>
        <span>REC</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .timeline-container {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    padding: 0.75rem;
    background: var(--phantom-ear-surface);
    border-radius: 0.5rem;
    border: 1px solid var(--phantom-ear-border);
  }

  .time-markers {
    display: flex;
    justify-content: space-between;
    padding: 0 0.125rem;
  }

  .time-label {
    font-size: 0.625rem;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    color: var(--phantom-ear-text-muted);
    letter-spacing: 0.025em;
  }

  .waveform-track {
    position: relative;
    height: 2.5rem;
    background: var(--phantom-ear-bg);
    border-radius: 0.25rem;
    cursor: pointer;
    overflow: hidden;
  }

  .waveform-bars {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    height: 100%;
    padding: 0.25rem 0.125rem;
    gap: 1px;
  }

  .waveform-bar {
    flex: 1;
    background: var(--phantom-ear-text-muted);
    opacity: 0.25;
    border-radius: 1px;
    transition: opacity 0.15s ease, background-color 0.15s ease;
    min-height: 2px;
  }

  .waveform-bar.has-segment {
    background: var(--phantom-ear-accent);
    opacity: 0.5;
  }

  .waveform-track:hover .waveform-bar {
    opacity: 0.35;
  }

  .waveform-track:hover .waveform-bar.has-segment {
    opacity: 0.7;
  }

  .playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--phantom-ear-danger);
    transform: translateX(-50%);
    pointer-events: none;
    z-index: 10;
    box-shadow: 0 0 4px var(--phantom-ear-danger);
  }

  .playhead::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 6px;
    height: 6px;
    background: var(--phantom-ear-danger);
    border-radius: 50%;
  }

  .recording-badge {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    background: rgba(0, 0, 0, 0.6);
    border-radius: 0.25rem;
    pointer-events: none;
    font-size: 0.625rem;
    font-weight: 600;
    color: var(--phantom-ear-danger);
    letter-spacing: 0.05em;
  }

  .recording-dot {
    width: 6px;
    height: 6px;
    background: var(--phantom-ear-danger);
    border-radius: 50%;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
</style>
