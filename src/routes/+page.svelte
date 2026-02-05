<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import Setup from "$lib/components/Setup.svelte";
  import Settings from "$lib/components/Settings.svelte";

  interface ModelStatus {
    whisper_downloaded: boolean;
    whisper_model: string;
    whisper_size_mb: number;
    models_dir: string;
  }

  interface TranscriptSegment {
    id: string;
    time: string;
    text: string;
    timestamp_ms: number;
  }

  interface TranscriptionEvent {
    id: string;
    text: string;
    start_ms: number;
    end_ms: number;
    is_partial: boolean;
  }

  // App state
  let needsSetup = $state(true);
  let isLoading = $state(true);
  let isRecording = $state(false);
  let recordingDuration = $state(0);
  let transcript = $state<TranscriptSegment[]>([]);

  // Q&A state
  let question = $state("");
  let isAsking = $state(false);
  let answer = $state("");

  // Settings modal
  let showSettings = $state(false);

  // Language indicator
  let currentLanguage = $state("en");
  const languageNames: Record<string, string> = {
    auto: "Auto-detect",
    en: "English",
    fr: "French",
    es: "Spanish",
    de: "German",
    it: "Italian",
    pt: "Portuguese",
    nl: "Dutch",
    pl: "Polish",
    ru: "Russian",
    ja: "Japanese",
    ko: "Korean",
    zh: "Chinese",
    ar: "Arabic",
  };

  // Timer and event listener
  let timerInterval: ReturnType<typeof setInterval> | null = null;
  let unlistenTranscription: UnlistenFn | null = null;
  let transcriptContainer: HTMLDivElement | null = null;

  interface Settings {
    llm_provider: string;
    openai_api_key: string | null;
    ollama_url: string | null;
    ollama_model: string | null;
    auto_detect_meetings: boolean;
    whisper_model: string;
    language: string;
  }

  onMount(async () => {
    try {
      const status = await invoke<ModelStatus>("check_model_status");
      needsSetup = !status.whisper_downloaded;

      // Load current settings to get language
      try {
        const settings = await invoke<Settings>("get_settings");
        currentLanguage = settings.language;
      } catch (e) {
        console.error("Failed to load settings:", e);
      }

      // If model is downloaded, load it into memory
      if (status.whisper_downloaded) {
        try {
          await invoke("load_model", { modelName: status.whisper_model });
          console.log("Model loaded:", status.whisper_model);
        } catch (e) {
          console.error("Failed to load model:", e);
        }
      }
    } catch (e) {
      console.error("Failed to check model status:", e);
      needsSetup = true;
    }
    isLoading = false;
  });

  onDestroy(() => {
    if (timerInterval) {
      clearInterval(timerInterval);
    }
    if (unlistenTranscription) {
      unlistenTranscription();
    }
  });

  // Format milliseconds to MM:SS
  function formatTimeMs(ms: number): string {
    const totalSecs = Math.floor(ms / 1000);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }

  // Subscribe to transcription events
  async function startTranscriptionListener() {
    unlistenTranscription = await listen<TranscriptionEvent>("transcription", (event) => {
      const data = event.payload;

      // Convert TranscriptionEvent to TranscriptSegment
      const segment: TranscriptSegment = {
        id: data.id,
        time: formatTimeMs(data.start_ms),
        text: data.text,
        timestamp_ms: data.start_ms,
      };

      // Add to transcript
      transcript = [...transcript, segment];

      // Auto-scroll to bottom
      requestAnimationFrame(() => {
        if (transcriptContainer) {
          transcriptContainer.scrollTop = transcriptContainer.scrollHeight;
        }
      });
    });
  }

  // Stop transcription listener
  function stopTranscriptionListener() {
    if (unlistenTranscription) {
      unlistenTranscription();
      unlistenTranscription = null;
    }
  }

  function handleSetupComplete() {
    needsSetup = false;
  }

  function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }

  async function toggleRecording() {
    if (isRecording) {
      // Stop recording
      isRecording = false;

      // Stop the timer
      if (timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
      }

      // Stop transcription listener
      stopTranscriptionListener();

      try {
        const result = await invoke<TranscriptSegment[]>("stop_recording");
        // Update with final transcript from backend (includes any remaining segments)
        if (result.length > 0) {
          transcript = result;
        }
      } catch (e) {
        console.error("Failed to stop recording:", e);
      }
    } else {
      // Start recording
      try {
        // Clear previous state
        transcript = [];
        answer = "";
        recordingDuration = 0;

        // Start listening for transcription events BEFORE starting recording
        await startTranscriptionListener();

        await invoke("start_recording");
        isRecording = true;

        timerInterval = setInterval(() => {
          recordingDuration++;
        }, 1000);
      } catch (e) {
        console.error("Failed to start recording:", e);
        stopTranscriptionListener();
      }
    }
  }

  async function askQuestion() {
    if (!question.trim() || isAsking) return;
    isAsking = true;
    answer = "";

    try {
      answer = await invoke<string>("ask_question", { question });
    } catch (e) {
      answer = `Error: ${e}`;
    }

    isAsking = false;
    question = "";
  }

  function openSettings() {
    showSettings = true;
  }

  async function closeSettings() {
    showSettings = false;
    // Refresh language after settings might have changed
    try {
      const settings = await invoke<Settings>("get_settings");
      currentLanguage = settings.language;
    } catch (e) {
      console.error("Failed to refresh settings:", e);
    }
  }
</script>

{#if isLoading}
  <div class="flex items-center justify-center min-h-screen bg-sidecar-bg">
    <div class="w-8 h-8 border-2 border-sidecar-accent border-t-transparent rounded-full animate-spin"></div>
  </div>
{:else if needsSetup}
  <Setup onComplete={handleSetupComplete} />
{:else}
  <main class="flex flex-col h-screen p-6 no-select bg-sidecar-bg">
    <!-- Header -->
    <header class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-lg bg-sidecar-accent flex items-center justify-center">
          <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
          </svg>
        </div>
        <h1 class="text-xl font-semibold text-sidecar-text">Sidecar</h1>
      </div>

      <div class="flex items-center gap-2">
        <!-- Language Indicator -->
        <button
          onclick={openSettings}
          class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-sidecar-surface border border-sidecar-border hover:border-sidecar-text-muted transition-colors"
          title="Change language"
        >
          <svg class="w-4 h-4 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
          </svg>
          <span class="text-xs font-medium text-sidecar-text">{languageNames[currentLanguage] || currentLanguage}</span>
          <svg class="w-3 h-3 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </button>

        <!-- Settings Button -->
        <button
          onclick={openSettings}
          class="p-2 rounded-lg hover:bg-sidecar-surface-hover transition-colors"
          title="Settings"
        >
          <svg class="w-5 h-5 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      </div>
    </header>

    <!-- Recording Control -->
    <div class="flex flex-col items-center justify-center py-8">
      <div class="relative">
        <!-- Outer ring pulse when recording -->
        {#if isRecording}
          <div class="absolute inset-0 rounded-full bg-sidecar-danger/30 animate-ring-pulse"></div>
          <div class="absolute inset-0 rounded-full bg-sidecar-danger/20 animate-ring-pulse" style="animation-delay: 0.5s"></div>
        {/if}

        <button
          onclick={toggleRecording}
          class="relative w-24 h-24 rounded-full transition-all duration-300 btn-shine {isRecording
            ? 'bg-gradient-danger animate-recording-glow'
            : 'bg-gradient-accent animate-idle-glow hover:scale-105'}"
        >
          {#if isRecording}
            <svg class="w-8 h-8 mx-auto text-white relative z-10" fill="currentColor" viewBox="0 0 24 24">
              <rect x="6" y="6" width="12" height="12" rx="2" />
            </svg>
          {:else}
            <svg class="w-8 h-8 mx-auto text-white" fill="currentColor" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="6" />
            </svg>
          {/if}
        </button>
      </div>

      <p class="mt-4 text-sm text-sidecar-text-muted">
        {#if isRecording}
          Recording <span class="font-mono text-sidecar-danger font-semibold">{formatDuration(recordingDuration)}</span>
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

      <div class="flex-1 glass rounded-xl border border-sidecar-border overflow-hidden shadow-glow-surface">
        {#if transcript.length === 0}
          <div class="flex flex-col items-center justify-center h-full text-sidecar-text-muted">
            <div class="w-16 h-16 mb-4 rounded-2xl bg-sidecar-surface/50 flex items-center justify-center">
              <svg class="w-8 h-8 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            </div>
            <p class="text-sm font-medium">
              {isRecording ? "Listening..." : "No transcript yet"}
            </p>
            <p class="text-xs mt-1 opacity-70">
              {isRecording ? "Speech will appear here in real-time" : "Start recording to capture audio"}
            </p>
          </div>
        {:else}
          <div bind:this={transcriptContainer} class="p-4 space-y-3 overflow-y-auto h-full scroll-smooth">
            {#each transcript as segment (segment.id)}
              <div class="flex gap-3 animate-fade-in p-2 rounded-lg hover:bg-sidecar-surface/50 transition-colors">
                <span class="text-xs text-sidecar-accent font-mono shrink-0 pt-0.5">{segment.time}</span>
                <p class="text-sm leading-relaxed text-sidecar-text">{segment.text}</p>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Answer Display -->
    {#if answer}
      <div class="mt-4 p-4 glass rounded-xl border border-sidecar-border shadow-glow-accent">
        <div class="flex items-center gap-2 mb-2">
          <div class="w-5 h-5 rounded-full bg-gradient-accent flex items-center justify-center">
            <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          <h3 class="text-xs font-medium text-sidecar-text-muted uppercase tracking-wide">AI Answer</h3>
        </div>
        <p class="text-sm text-sidecar-text whitespace-pre-wrap leading-relaxed">{answer}</p>
      </div>
    {/if}

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
          disabled={transcript.length === 0}
          class="flex-1 px-4 py-3 glass border border-sidecar-border rounded-xl text-sm text-sidecar-text placeholder:text-sidecar-text-muted focus:outline-none focus:border-sidecar-accent focus:shadow-glow-accent transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        />
        <button
          type="submit"
          disabled={!question.trim() || isAsking || transcript.length === 0}
          class="px-5 py-3 bg-gradient-accent hover:bg-gradient-accent-hover rounded-xl text-sm font-medium text-white transition-all hover-lift disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:transform-none btn-shine"
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

  <!-- Settings Modal -->
  {#if showSettings}
    <Settings onClose={closeSettings} />
  {/if}
{/if}
