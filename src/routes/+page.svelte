<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import Setup from "$lib/components/Setup.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { meetingsStore } from "$lib/stores/meetings.svelte";
  import type { ModelStatus, TranscriptSegment, TranscriptionEvent, Settings as SettingsType, ModelInfo, View, Summary } from "$lib/types";

  interface DownloadProgress {
    model_name: string;
    downloaded_bytes: number;
    total_bytes: number;
    percentage: number;
    status: "Starting" | "Downloading" | "Completed" | "Failed" | "Cancelled";
  }

  // App state
  let needsSetup = $state(true);
  let isLoading = $state(true);
  let isRecording = $state(false);
  let recordingDuration = $state(0);
  let transcript = $state<TranscriptSegment[]>([]);

  // Pause state
  let isPaused = $state(false);

  // Q&A state
  let question = $state("");
  let isAsking = $state(false);
  let answer = $state("");

  // Summary state
  let isGeneratingSummary = $state(false);
  let summary = $state<Summary | null>(null);

  // UI state
  let currentView = $state<View>('home');
  let sidebarCollapsed = $state(false);
  let transcriptCollapsed = $state(false);
  let downloadingModel = $state<string | null>(null);
  let downloadProgress = $state<DownloadProgress | null>(null);
  let unlistenDownload: UnlistenFn | null = null;

  // Settings/model state
  let currentLanguage = $state("en");
  let currentModel = $state("base");
  let models = $state<ModelInfo[]>([]);
  let llmProvider = $state("ollama");
  let llmModelName = $state("");

  // Export state
  let exportCopied = $state(false);

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

  onMount(async () => {
    try {
      const status = await invoke<ModelStatus>("check_model_status");
      needsSetup = !status.whisper_downloaded;

      // Load current settings, models, and meetings
      try {
        const [settings, loadedModels] = await Promise.all([
          invoke<SettingsType>("get_settings"),
          invoke<ModelInfo[]>("get_models_info"),
        ]);
        currentLanguage = settings.language;
        currentModel = settings.whisper_model;
        models = loadedModels;
        llmProvider = settings.llm_provider;
        llmModelName = settings.ollama_model || "";
      } catch (e) {
        console.error("Failed to load settings:", e);
      }

      // Load meetings from DB
      await meetingsStore.loadMeetings();

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
    if (unlistenDownload) {
      unlistenDownload();
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

      // Add to local transcript
      transcript = [...transcript, segment];

      // Also add to active meeting (optimistic UI)
      meetingsStore.addLocalSegment(segment);

      // Auto-scroll to bottom
      scrollTranscriptToBottom();
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
      isPaused = false;

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
          meetingsStore.setActiveTranscript(result);
        }
      } catch (e) {
        console.error("Failed to stop recording:", e);
      }

      // Refresh meetings list to show updated segment counts
      await meetingsStore.loadMeetings();
    } else {
      // Start recording
      try {
        // Clear previous state
        transcript = [];
        answer = "";
        summary = null;
        recordingDuration = 0;

        // Start listening for transcription events BEFORE starting recording
        await startTranscriptionListener();

        // start_recording now returns meeting ID
        const meetingId = await invoke<string>("start_recording");
        isRecording = true;

        // Set active meeting and refresh list
        meetingsStore.setActive(meetingId);
        meetingsStore.setActiveTranscript([]);
        await meetingsStore.loadMeetings();

        timerInterval = setInterval(() => {
          recordingDuration++;
        }, 1000);
      } catch (e) {
        console.error("Failed to start recording:", e);
        stopTranscriptionListener();
      }
    }
  }

  async function togglePause() {
    if (!isRecording) return;
    try {
      if (isPaused) {
        await invoke("resume_recording");
        isPaused = false;
      } else {
        await invoke("pause_recording");
        isPaused = true;
      }
    } catch (e) {
      console.error("Failed to toggle pause:", e);
    }
  }

  async function askQuestion() {
    if (!question.trim() || isAsking) return;
    isAsking = true;
    answer = "";

    try {
      answer = await invoke<string>("ask_question", { question, meetingId: isRecording ? null : meetingsStore.activeMeetingId });
    } catch (e) {
      answer = `Error: ${e}`;
    }

    isAsking = false;
    question = "";
    scrollTranscriptToBottom();
  }

  async function generateSummary() {
    if (isGeneratingSummary || transcript.length === 0) return;
    isGeneratingSummary = true;
    summary = null;

    try {
      summary = await invoke<Summary>("generate_summary", { meetingId: isRecording ? null : meetingsStore.activeMeetingId });
    } catch (e) {
      // Show error in answer field as fallback
      answer = `Summary Error: ${e}`;
    }

    isGeneratingSummary = false;
    scrollTranscriptToBottom();
  }

  function handleNavigate(view: View) {
    currentView = view;
  }

  function scrollTranscriptToBottom() {
    requestAnimationFrame(() => {
      if (transcriptContainer) {
        transcriptContainer.scrollTop = transcriptContainer.scrollHeight;
      }
    });
  }

  async function handleSelectMeeting(id: string) {
    await meetingsStore.selectMeeting(id);
    transcript = meetingsStore.activeTranscript;
    currentView = 'home';
    scrollTranscriptToBottom();
  }

  async function handleLanguageChange(lang: string) {
    currentLanguage = lang;
    try {
      const settings = await invoke<SettingsType>("get_settings");
      settings.language = lang;
      await invoke("save_settings", { settings });
      await invoke("load_model", { modelName: currentModel });
    } catch (e) {
      console.error("Failed to save language:", e);
    }
  }

  async function handleLlmChange(provider: string) {
    llmProvider = provider;
    try {
      const settings = await invoke<SettingsType>("get_settings");
      settings.llm_provider = provider;
      await invoke("save_settings", { settings });
    } catch (e) {
      console.error("Failed to save LLM provider:", e);
    }
  }

  async function handleModelChange(model: string) {
    currentModel = model;
    try {
      const settings = await invoke<SettingsType>("get_settings");
      settings.whisper_model = model;
      await invoke("save_settings", { settings });
      await invoke("load_model", { modelName: model });
      // Refresh models list
      models = await invoke<ModelInfo[]>("get_models_info");
    } catch (e) {
      console.error("Failed to change model:", e);
    }
  }

  async function handleDownloadModel(modelName: string) {
    downloadingModel = modelName;
    downloadProgress = null;

    try {
      // Listen for download progress events
      unlistenDownload = await listen<DownloadProgress>("model-download-progress", (event) => {
        downloadProgress = event.payload;
        if (event.payload.status === "Completed") {
          // Download complete, update state
          setTimeout(async () => {
            currentModel = modelName;
            models = await invoke<ModelInfo[]>("get_models_info");
            downloadingModel = null;
            downloadProgress = null;
            if (unlistenDownload) {
              unlistenDownload();
              unlistenDownload = null;
            }
          }, 500);
        } else if (event.payload.status === "Failed") {
          downloadingModel = null;
          downloadProgress = null;
          if (unlistenDownload) {
            unlistenDownload();
            unlistenDownload = null;
          }
        }
      });

      // Download and load the model
      await invoke("download_model", { modelName });
      // Update settings
      const settings = await invoke<SettingsType>("get_settings");
      settings.whisper_model = modelName;
      await invoke("save_settings", { settings });
    } catch (e) {
      console.error("Failed to download model:", e);
      downloadingModel = null;
      downloadProgress = null;
      if (unlistenDownload) {
        unlistenDownload();
        unlistenDownload = null;
      }
    }
  }

  async function handleSettingsSaved() {
    // Refresh settings after save
    try {
      const [settings, loadedModels] = await Promise.all([
        invoke<SettingsType>("get_settings"),
        invoke<ModelInfo[]>("get_models_info"),
      ]);
      currentLanguage = settings.language;
      currentModel = settings.whisper_model;
      models = loadedModels;
      llmProvider = settings.llm_provider;
      llmModelName = settings.ollama_model || "";
    } catch (e) {
      console.error("Failed to refresh settings:", e);
    }
    currentView = 'home';
  }

  async function handleExportMeeting() {
    const meetingId = meetingsStore.activeMeetingId;
    if (!meetingId) return;
    try {
      const md = await meetingsStore.exportMeeting(meetingId, 'markdown');
      await navigator.clipboard.writeText(md);
      exportCopied = true;
      setTimeout(() => { exportCopied = false; }, 2000);
    } catch (e) {
      console.error("Failed to export meeting:", e);
    }
  }

  function handleSearch(query: string) {
    meetingsStore.searchMeetings(query);
  }

  // Derived values for sidebar
  let pinnedMeetings = $derived(meetingsStore.getPinnedMeetings());
  let recentMeetings = $derived(meetingsStore.getRecentMeetings());
</script>

{#if isLoading}
  <div class="flex items-center justify-center min-h-screen bg-sidecar-bg">
    <div class="w-8 h-8 border-2 border-sidecar-accent border-t-transparent rounded-full animate-spin"></div>
  </div>
{:else if needsSetup}
  <Setup onComplete={handleSetupComplete} />
{:else}
  <div class="flex h-screen bg-sidecar-bg no-select">
    <!-- Sidebar -->
    <Sidebar
      collapsed={sidebarCollapsed}
      {currentView}
      {pinnedMeetings}
      {recentMeetings}
      activeMeetingId={meetingsStore.activeMeetingId}
      onToggle={() => sidebarCollapsed = !sidebarCollapsed}
      onNavigate={handleNavigate}
      onSelectMeeting={handleSelectMeeting}
      onRenameMeeting={(id, title) => meetingsStore.renameMeeting(id, title)}
      onTogglePinMeeting={(id) => meetingsStore.togglePin(id)}
      onDeleteMeeting={(id) => meetingsStore.deleteMeeting(id)}
      onSearch={handleSearch}
    />

    <!-- Main Content -->
    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <!-- Top Bar -->
      <TopBar
        language={currentLanguage}
        currentModel={currentModel}
        {models}
        {llmProvider}
        {llmModelName}
        onLanguageChange={handleLanguageChange}
        onModelChange={handleModelChange}
        onDownloadModel={handleDownloadModel}
        onLlmChange={handleLlmChange}
        {isRecording}
        {recordingDuration}
        {isPaused}
        onToggleRecording={toggleRecording}
        onTogglePause={togglePause}
      />

      <!-- Content Area -->
      <div class="flex-1 flex flex-col overflow-hidden">
        {#if currentView === 'home'}
          <div class="flex-1 flex flex-col p-6 overflow-hidden">
            <!-- Recording Control (only show big button when NOT recording) -->
            {#if !isRecording}
              <div class="flex flex-col items-center justify-center py-6">
                <div class="relative">
                  <button
                    onclick={toggleRecording}
                    class="relative w-20 h-20 rounded-full transition-all duration-300 btn-shine bg-gradient-accent animate-idle-glow hover:scale-105"
                  >
                    <svg class="w-7 h-7 mx-auto text-white" fill="currentColor" viewBox="0 0 24 24">
                      <circle cx="12" cy="12" r="6" />
                    </svg>
                  </button>
                </div>

                <p class="mt-3 text-sm text-sidecar-text-muted">
                  Click to start recording
                </p>
              </div>
            {/if}

            <!-- Transcript + AI Results Area -->
            <div class="flex-1 flex flex-col min-h-0">
              <!-- Collapsible Header -->
              <div class="flex items-center justify-between mb-3">
                <button
                  onclick={() => transcriptCollapsed = !transcriptCollapsed}
                  class="flex items-center gap-2 group"
                >
                  <svg
                    class="w-4 h-4 text-sidecar-text-muted transition-transform {transcriptCollapsed ? '-rotate-90' : ''}"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  </svg>
                  <h2 class="text-sm font-medium text-sidecar-text-muted uppercase tracking-wide">Live Transcript</h2>
                </button>
                <div class="flex items-center gap-2">
                  {#if transcript.length > 0}
                    <span class="text-xs text-sidecar-text-muted">{transcript.length} segments</span>
                  {/if}
                  {#if transcript.length > 0 && !isRecording}
                    <button
                      onclick={handleExportMeeting}
                      class="px-2 py-1 text-xs rounded-md bg-sidecar-surface border border-sidecar-border text-sidecar-text-muted hover:text-sidecar-text hover:border-sidecar-accent transition-colors"
                      title="Export transcript to clipboard"
                    >
                      {exportCopied ? 'Copied!' : 'Export'}
                    </button>
                  {/if}
                </div>
              </div>

              {#if !transcriptCollapsed}
                <div class="flex-1 glass rounded-xl border border-sidecar-border overflow-hidden shadow-glow-surface transition-all duration-200">
                  {#if transcript.length === 0 && !summary && !answer}
                    <div class="flex flex-col items-center justify-center h-full text-sidecar-text-muted">
                      <div class="w-14 h-14 mb-4 rounded-2xl bg-sidecar-surface/50 flex items-center justify-center">
                        <svg class="w-7 h-7 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                    <div bind:this={transcriptContainer} class="p-4 space-y-2 overflow-y-auto h-full scroll-smooth">
                      {#each transcript as segment (segment.id)}
                        <div class="flex gap-3 animate-fade-in p-2 rounded-lg hover:bg-sidecar-surface/50 transition-colors">
                          <span class="text-xs text-sidecar-accent font-mono shrink-0 pt-0.5">{segment.time}</span>
                          <p class="text-sm leading-relaxed text-sidecar-text">{segment.text}</p>
                        </div>
                      {/each}

                      <!-- Summary Display (inside scroll) -->
                      {#if summary}
                        <div class="mt-4 p-4 rounded-xl bg-sidecar-purple/5 border border-sidecar-purple/20">
                          <div class="flex items-center gap-2 mb-3">
                            <div class="w-5 h-5 rounded-full bg-sidecar-purple flex items-center justify-center">
                              <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                              </svg>
                            </div>
                            <h3 class="text-xs font-medium text-sidecar-text-muted uppercase tracking-wide">Meeting Summary</h3>
                          </div>

                          {#if summary.overview}
                            <p class="text-sm text-sidecar-text leading-relaxed mb-4">{summary.overview}</p>
                          {/if}

                          {#if summary.key_points.length > 0}
                            <div class="mb-3">
                              <h4 class="text-xs font-semibold text-sidecar-text-muted uppercase tracking-wide mb-2">Key Points</h4>
                              <ul class="space-y-1">
                                {#each summary.key_points as point}
                                  <li class="flex items-start gap-2 text-sm text-sidecar-text">
                                    <span class="text-sidecar-accent mt-1">&#8226;</span>
                                    <span>{point}</span>
                                  </li>
                                {/each}
                              </ul>
                            </div>
                          {/if}

                          {#if summary.action_items.length > 0}
                            <div>
                              <h4 class="text-xs font-semibold text-sidecar-text-muted uppercase tracking-wide mb-2">Action Items</h4>
                              <ul class="space-y-1">
                                {#each summary.action_items as item}
                                  <li class="flex items-start gap-2 text-sm text-sidecar-text">
                                    <span class="text-sidecar-success mt-1">&#10003;</span>
                                    <span>{item}</span>
                                  </li>
                                {/each}
                              </ul>
                            </div>
                          {/if}
                        </div>
                      {/if}

                      <!-- Answer Display (inside scroll) -->
                      {#if answer}
                        <div class="mt-4 p-4 rounded-xl bg-sidecar-accent/5 border border-sidecar-accent/20">
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
                    </div>
                  {/if}
                </div>
              {:else}
                <div class="py-2 text-sm text-sidecar-text-muted">
                  {transcript.length > 0 ? `${transcript.length} segments captured` : 'No transcript yet'}
                </div>
              {/if}
            </div>

            <!-- ChatGPT-style Input Bar -->
            <div class="mt-4">
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  askQuestion();
                }}
                class="relative flex items-center"
              >
                <input
                  type="text"
                  bind:value={question}
                  placeholder="Ask a question about the meeting..."
                  disabled={transcript.length === 0}
                  class="w-full pl-4 pr-28 py-3.5 glass border border-sidecar-border rounded-2xl text-sm text-sidecar-text placeholder:text-sidecar-text-muted focus:outline-none focus:border-sidecar-accent/50 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                />
                <!-- Icons inside the input, right side -->
                <div class="absolute right-2 flex items-center gap-1">
                  <!-- Summary button -->
                  <button
                    type="button"
                    onclick={generateSummary}
                    disabled={isGeneratingSummary || transcript.length === 0}
                    class="p-2 rounded-lg text-sidecar-text-muted hover:text-sidecar-purple hover:bg-sidecar-surface transition-colors disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:text-sidecar-text-muted disabled:hover:bg-transparent"
                    title="Generate summary"
                  >
                    {#if isGeneratingSummary}
                      <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                      </svg>
                    {:else}
                      <!-- Document/summary icon -->
                      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                      </svg>
                    {/if}
                  </button>

                  <!-- Send button -->
                  <button
                    type="submit"
                    disabled={!question.trim() || isAsking || transcript.length === 0}
                    class="p-2 rounded-xl bg-sidecar-text text-sidecar-bg hover:opacity-80 transition-all disabled:opacity-20 disabled:cursor-not-allowed"
                    title="Send question"
                  >
                    {#if isAsking}
                      <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                      </svg>
                    {:else}
                      <!-- Arrow up icon -->
                      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18" />
                      </svg>
                    {/if}
                  </button>
                </div>
              </form>
            </div>
          </div>

        {:else if currentView === 'genie'}
          <div class="flex-1 flex flex-col items-center justify-center p-6 text-center">
            <div class="w-16 h-16 mb-4 rounded-2xl bg-sidecar-purple/20 flex items-center justify-center">
              <svg class="w-8 h-8 text-sidecar-purple" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z" />
              </svg>
            </div>
            <h2 class="text-xl font-semibold text-sidecar-text mb-2">Genie AI Assistant</h2>
            <p class="text-sm text-sidecar-text-muted max-w-md">
              Coming soon! Ask Genie to summarize meetings, extract action items, and answer questions about your recorded sessions.
            </p>
          </div>

        {:else if currentView === 'settings'}
          <div class="flex-1 overflow-y-auto">
            <Settings onClose={handleSettingsSaved} inline={true} />
          </div>
        {/if}
      </div>
    </main>
  </div>

  <!-- Download Progress Overlay -->
  {#if downloadingModel}
    <div class="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center">
      <div class="bg-sidecar-surface rounded-2xl border border-sidecar-border p-6 w-80 shadow-glow-surface">
        <div class="flex items-center gap-3 mb-4">
          <div class="w-10 h-10 rounded-xl bg-sidecar-accent/20 flex items-center justify-center">
            <svg class="w-5 h-5 text-sidecar-accent animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-semibold text-sidecar-text">Downloading Model</h3>
            <p class="text-xs text-sidecar-text-muted capitalize">{downloadingModel}</p>
          </div>
        </div>

        {#if downloadProgress}
          <div class="space-y-2">
            <div class="h-2 bg-sidecar-border rounded-full overflow-hidden">
              <div
                class="h-full bg-gradient-accent transition-all duration-300"
                style="width: {downloadProgress.percentage}%"
              ></div>
            </div>
            <div class="flex justify-between text-xs text-sidecar-text-muted">
              <span>{downloadProgress.status}</span>
              <span>{downloadProgress.percentage.toFixed(0)}%</span>
            </div>
          </div>
        {:else}
          <div class="flex items-center justify-center py-2">
            <div class="w-5 h-5 border-2 border-sidecar-accent border-t-transparent rounded-full animate-spin"></div>
            <span class="ml-2 text-sm text-sidecar-text-muted">Preparing...</span>
          </div>
        {/if}
      </div>
    </div>
  {/if}
{/if}
