<script lang="ts">
  let {
    isRecording = false,
    recordingDuration = 0,
    onClick,
  }: {
    isRecording?: boolean;
    recordingDuration?: number;
    onClick?: () => void;
  } = $props();

  // Format duration
  function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
</script>

{#if isRecording}
  <button
    onclick={onClick}
    class="recording-indicator"
    title="Return to live recording"
  >
    <span class="recording-dot"></span>
    <span class="recording-text">
      Recording in progress
      <span class="recording-time">{formatDuration(recordingDuration)}</span>
    </span>
    <span class="return-hint">Click to return</span>
  </button>
{/if}

<style>
  .recording-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 9999px;
    cursor: pointer;
    transition: all 0.15s ease;
    font-size: 0.75rem;
    color: var(--phantom-ear-danger);
  }

  .recording-indicator:hover {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.5);
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

  .recording-text {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
  }

  .recording-time {
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 0.625rem;
    background: rgba(239, 68, 68, 0.2);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }

  .return-hint {
    font-size: 0.625rem;
    color: var(--phantom-ear-text-muted);
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .recording-indicator:hover .return-hint {
    opacity: 1;
  }
</style>
