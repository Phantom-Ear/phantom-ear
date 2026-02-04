<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  // Recording state
  let isRecording = $state(false);
  let recordingDuration = $state(0);
  let transcript = $state<Array<{ time: string; text: string }>>([]);

  // Q&A state
  let question = $state("");
  let isAsking = $state(false);

  // Interval for timer
  let timerInterval: ReturnType<typeof setInterval> | null = null;

  function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }

  async function toggleRecording() {
    if (isRecording) {
      // Stop recording
      isRecording = false;
      if (timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
      }
      // TODO: Call Rust to stop recording
      // await invoke("stop_recording");
    } else {
      // Start recording
      isRecording = true;
      recordingDuration = 0;
      transcript = [];
      timerInterval = setInterval(() => {
        recordingDuration++;
      }, 1000);
      // TODO: Call Rust to start recording
      // await invoke("start_recording");
    }
  }

  async function askQuestion() {
    if (!question.trim() || isAsking) return;
    isAsking = true;
    // TODO: Call Rust to process question
    // const answer = await invoke("ask_question", { question });
    setTimeout(() => {
      isAsking = false;
      question = "";
    }, 1000);
  }
</script>

<main class="flex flex-col h-screen p-6 no-select">
  <!-- Header -->
  <header class="flex items-center justify-between mb-6">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-sidecar-accent flex items-center justify-center">
        <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
        </svg>
      </div>
      <h1 class="text-xl font-semibold">Sidecar</h1>
    </div>

    <button
      class="p-2 rounded-lg hover:bg-sidecar-surface-hover transition-colors"
      title="Settings"
    >
      <svg class="w-5 h-5 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    </button>
  </header>

  <!-- Recording Control -->
  <div class="flex flex-col items-center justify-center py-8">
    <button
      onclick={toggleRecording}
      class="relative w-24 h-24 rounded-full transition-all duration-300 {isRecording
        ? 'bg-sidecar-danger hover:bg-red-600'
        : 'bg-sidecar-surface hover:bg-sidecar-surface-hover border-2 border-sidecar-border'}"
    >
      {#if isRecording}
        <div class="absolute inset-0 rounded-full bg-sidecar-danger animate-pulse-recording opacity-50"></div>
        <svg class="w-8 h-8 mx-auto text-white relative z-10" fill="currentColor" viewBox="0 0 24 24">
          <rect x="6" y="6" width="12" height="12" rx="2" />
        </svg>
      {:else}
        <svg class="w-8 h-8 mx-auto text-sidecar-text-muted" fill="currentColor" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="6" />
        </svg>
      {/if}
    </button>

    <p class="mt-4 text-sm text-sidecar-text-muted">
      {#if isRecording}
        Recording <span class="font-mono text-sidecar-danger">{formatDuration(recordingDuration)}</span>
      {:else}
        Click to start recording
      {/if}
    </p>
  </div>

  <!-- Transcript Area -->
  <div class="flex-1 flex flex-col min-h-0">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-sm font-medium text-sidecar-text-muted uppercase tracking-wide">Live Transcript</h2>
      {#if transcript.length > 0}
        <span class="text-xs text-sidecar-text-muted">{transcript.length} segments</span>
      {/if}
    </div>

    <div class="flex-1 bg-sidecar-surface rounded-xl border border-sidecar-border overflow-hidden">
      {#if transcript.length === 0}
        <div class="flex flex-col items-center justify-center h-full text-sidecar-text-muted">
          <svg class="w-12 h-12 mb-3 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          <p class="text-sm">
            {isRecording ? "Listening..." : "No transcript yet"}
          </p>
          <p class="text-xs mt-1 opacity-70">
            {isRecording ? "Speech will appear here in real-time" : "Start recording to capture audio"}
          </p>
        </div>
      {:else}
        <div class="p-4 space-y-3 overflow-y-auto h-full">
          {#each transcript as segment}
            <div class="flex gap-3">
              <span class="text-xs text-sidecar-text-muted font-mono shrink-0">{segment.time}</span>
              <p class="text-sm leading-relaxed">{segment.text}</p>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Q&A Input -->
  <div class="mt-4">
    <form
      onsubmit={(e) => {
        e.preventDefault();
        askQuestion();
      }}
      class="flex gap-2"
    >
      <input
        type="text"
        bind:value={question}
        placeholder="Ask a question about the meeting..."
        disabled={!isRecording && transcript.length === 0}
        class="flex-1 px-4 py-3 bg-sidecar-surface border border-sidecar-border rounded-xl text-sm placeholder:text-sidecar-text-muted focus:outline-none focus:border-sidecar-accent transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      />
      <button
        type="submit"
        disabled={!question.trim() || isAsking || (!isRecording && transcript.length === 0)}
        class="px-4 py-3 bg-sidecar-accent hover:bg-sidecar-accent-hover rounded-xl text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if isAsking}
          <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
          </svg>
        {:else}
          Ask
        {/if}
      </button>
    </form>
  </div>
</main>
