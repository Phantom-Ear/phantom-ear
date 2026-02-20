<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import Setup from "$lib/components/Setup.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import ReferenceCard from "$lib/components/ReferenceCard.svelte";
  import SearchOverlay from "$lib/components/SearchOverlay.svelte";
  import TranscriptTimeline from "$lib/components/TranscriptTimeline.svelte";
  import HomeMetrics from "$lib/components/HomeMetrics.svelte";
  import EditableSegment from "$lib/components/EditableSegment.svelte";
  import Onboarding from "$lib/components/Onboarding.svelte";
  import MeetingNotification from "$lib/components/MeetingNotification.svelte";
  import { meetingsStore } from "$lib/stores/meetings.svelte";
  import { createShortcutHandler, isMacOS } from "$lib/utils/keyboard";
  import type { ModelStatus, TranscriptSegment, TranscriptionEvent, Settings as SettingsType, ModelInfo, View, Summary, SemanticSearchResult, Speaker } from "$lib/types";

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

  // Track the meeting ID that is currently being recorded (separate from active/viewed meeting)
  let liveRecordingMeetingId = $state<string | null>(null);

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

  // Phomy state
  let phomyQuestion = $state("");
  let phomyAnswer = $state("");
  let phomyIsAsking = $state(false);
  let phomyReferences = $state<SemanticSearchResult[]>([]);
  let phomyContextLimit = $state(10);
  let phomyHistory = $state<Array<{ role: 'user' | 'assistant'; text: string; refs?: SemanticSearchResult[] }>>([]);

  // Embedding state
  let embeddingModelLoaded = $state(false);
  let embeddingDownloading = $state(false);
  let embeddingDownloadFailed = $state(false);
  let showEmbeddingManualDownload = $state(false);
  let embeddingImporting = $state(false);

  // Export state
  let exportCopied = $state(false);
  let exportDropdownOpen = $state(false);
  let selectedExportFormat = $state("markdown");

  // Speakers state
  let speakers = $state<Speaker[]>([]);

  // Splash screen state
  let showSplash = $state(true);
  let splashFadingOut = $state(false);
  let splashAnimationDone = $state(false);

  // Text scramble state for welcome headline
  let scrambleOutput = $state<Array<{ char: string; scrambled: boolean }>>([]);
  let scrambleComplete = $state(false);
  let scrambleStarted = false;

  // Search overlay state
  let showSearchOverlay = $state(false);

  // Onboarding state
  let showOnboarding = $state(false);

  // Keyboard shortcut state
  let sidebarFocused = $state(false);

  // Meeting detection state
  let autoDetectMeetings = $state(false);

  // Derived: Are we viewing a past meeting (not the live recording)?
  const isViewingPastMeeting = $derived(
    meetingsStore.activeMeetingId !== null &&
    meetingsStore.activeMeetingId !== liveRecordingMeetingId &&
    currentView === 'home'
  );

  // Derived: Should show recording indicator (recording AND navigated away from live view)?
  const showRecordingIndicator = $derived(
    isRecording && (currentView !== 'home' || isViewingPastMeeting)
  );

  // Return to live recording
  function returnToLiveRecording() {
    // First switch view to home
    currentView = 'home';

    // Set active meeting to the live recording meeting
    // This ensures isViewingPastMeeting becomes false
    if (liveRecordingMeetingId) {
      meetingsStore.setActive(liveRecordingMeetingId);
      meetingsStore.setActiveTranscript(transcript);
    } else {
      meetingsStore.clearActive();
    }

    // Scroll transcript to bottom after DOM updates
    // Use requestAnimationFrame to ensure render is complete
    requestAnimationFrame(() => {
      scrollTranscriptToBottom();
      // Additional scroll attempt for reliability
      setTimeout(() => scrollTranscriptToBottom(), 150);
    });
  }

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
  let unlistenTray: UnlistenFn | null = null;
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
        autoDetectMeetings = settings.auto_detect_meetings;

        // Check if onboarding should be shown
        if (!settings.onboarding_completed) {
          showOnboarding = true;
        }
      } catch (e) {
        console.error("Failed to load settings:", e);
      }

      // Load meetings and speakers from DB
      await meetingsStore.loadMeetings();
      await loadSpeakers();

      // If model is downloaded, load it into memory
      if (status.whisper_downloaded) {
        try {
          await invoke("load_model", { modelName: status.whisper_model });
          console.log("Model loaded:", status.whisper_model);
        } catch (e) {
          console.error("Failed to load model:", e);
        }
      }

      // Auto-load or download embedding model
      initEmbeddingModel();

      // Start meeting detection if enabled
      if (autoDetectMeetings) {
        try {
          await invoke("start_meeting_detection");
          console.log("Meeting detection started");
        } catch (e) {
          console.error("Failed to start meeting detection:", e);
        }
      }
    } catch (e) {
      console.error("Failed to check model status:", e);
      needsSetup = true;
    }
    isLoading = false;
    // Wait for both loading AND animation to complete before hiding splash
    waitForSplashEnd();

    // Register global keyboard shortcuts
    window.addEventListener('keydown', handleGlobalKeydown);

    // Listen for system tray toggle recording event
    unlistenTray = await listen<void>("tray-toggle-recording", () => {
      toggleRecording();
    });
  });

  // Splash: logo fly-in (0.6s) + hold (0.6s) = ~1.2s minimum
  const SPLASH_MIN_DURATION = 1200;

  // Trigger text scramble when welcome screen is visible
  $effect(() => {
    if (!showSplash && !isRecording && transcript.length === 0 && !scrambleStarted) {
      runTextScramble();
    }
  });

  // Start splash timer on mount
  $effect(() => {
    if (showSplash) {
      const timeout = setTimeout(() => {
        splashAnimationDone = true;
      }, SPLASH_MIN_DURATION);
      return () => clearTimeout(timeout);
    }
  });

  function waitForSplashEnd() {
    const check = setInterval(() => {
      if (!isLoading && splashAnimationDone) {
        clearInterval(check);
        splashFadingOut = true;
        setTimeout(() => {
          showSplash = false;
        }, 500);
      }
    }, 50);
  }

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
    if (unlistenTray) {
      unlistenTray();
    }
    // Remove keyboard event listener
    window.removeEventListener('keydown', handleGlobalKeydown);
    // Stop meeting detection
    invoke("stop_meeting_detection").catch(() => {});
  });

  // Global keyboard shortcut handler
  function handleGlobalKeydown(e: KeyboardEvent) {
    // Skip if we're in an input field or textarea
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
      // Except for Escape key
      if (e.key === 'Escape') {
        (target as HTMLInputElement | HTMLTextAreaElement).blur();
      }
      return;
    }

    // Cmd/Ctrl + Shift + R: Toggle Recording
    const cmdKey = isMacOS() ? e.metaKey : e.ctrlKey;
    if (cmdKey && e.shiftKey && e.key.toLowerCase() === 'r') {
      e.preventDefault();
      if (!needsSetup && !isLoading) {
        toggleRecording();
      }
      return;
    }

    // Cmd/Ctrl + K: Quick Search
    if (cmdKey && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      showSearchOverlay = true;
      return;
    }

    // Cmd/Ctrl + B: Toggle Sidebar
    if (cmdKey && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      sidebarCollapsed = !sidebarCollapsed;
      return;
    }

    // Escape: Close search overlay
    if (e.key === 'Escape') {
      if (showSearchOverlay) {
        showSearchOverlay = false;
      }
      return;
    }
  }

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

      // Only add to meeting store if viewing the live recording (not a past meeting)
      // This prevents segments from appearing in old meetings when navigating during recording
      if (meetingsStore.activeMeetingId === liveRecordingMeetingId) {
        meetingsStore.addLocalSegment(segment);
      }

      // Auto-scroll to bottom (only if viewing the live recording)
      if (meetingsStore.activeMeetingId === liveRecordingMeetingId) {
        scrollTranscriptToBottom();
      }
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
      liveRecordingMeetingId = null;

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
        liveRecordingMeetingId = meetingId;

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

  async function initEmbeddingModel() {
    try {
      const downloaded = await invoke<boolean>("is_embedding_model_downloaded");
      if (downloaded) {
        await invoke("load_embedding_model");
        embeddingModelLoaded = true;
        console.log("Embedding model loaded");
      } else {
        // Auto-download in background
        embeddingDownloading = true;
        embeddingDownloadFailed = false;
        await invoke("download_embedding_model_cmd");
        embeddingModelLoaded = true;
        embeddingDownloading = false;
        console.log("Embedding model downloaded and loaded");
      }
    } catch (e) {
      console.error("Embedding model init failed:", e);
      embeddingDownloading = false;
      const errMsg = String(e);
      if (errMsg.includes("too small") || errMsg.includes("firewall") || errMsg.includes("proxy") || errMsg.includes("blocked")) {
        embeddingDownloadFailed = true;
      }
    }
  }

  async function openEmbeddingManualDownload() {
    showEmbeddingManualDownload = true;
    embeddingDownloadFailed = false;

    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      const urls = await invoke<{ model_url: string; tokenizer_url: string }>("get_embedding_model_download_urls");
      // Open both URLs in browser - user needs to download both files
      await openUrl(urls.model_url);
      // Small delay before opening second URL
      setTimeout(async () => {
        await openUrl(urls.tokenizer_url);
      }, 500);
    } catch (e) {
      console.error("Failed to open download links:", e);
    }
  }

  async function importEmbeddingModel() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    
    embeddingImporting = true;
    try {
      const selected = await open({
        title: "Select embedding model file (model.onnx, tokenizer.json, or .zip)",
        filters: [
          { name: "Model files", extensions: ["onnx", "json", "zip"] },
          { name: "All files", extensions: ["*"] },
        ],
        multiple: true,
      });

      if (!selected) {
        embeddingImporting = false;
        return;
      }

      const files = Array.isArray(selected) ? selected : [selected];

      // Import each selected file
      for (const filePath of files) {
        await invoke("import_embedding_model", { filePath: filePath });
      }

      // Load the model after import
      await invoke("load_embedding_model");
      embeddingModelLoaded = true;
      showEmbeddingManualDownload = false;
      console.log("Embedding model imported and loaded");
    } catch (e) {
      console.error("Embedding model import failed:", e);
    }
    embeddingImporting = false;
  }

  async function askPhomy() {
    if (!phomyQuestion.trim() || phomyIsAsking) return;
    const q = phomyQuestion.trim();
    phomyQuestion = "";
    phomyIsAsking = true;
    phomyAnswer = "";
    phomyReferences = [];
    phomyContextLimit = 10;

    phomyHistory = [...phomyHistory, { role: 'user', text: q }];

    try {
      // Semantic search for references (display only)
      const refs = await meetingsStore.semanticSearch(q, undefined, 10);
      phomyReferences = refs;

      // Use Phomy intelligent routing
      const ans = await invoke<string>("phomy_ask", { question: q });
      phomyAnswer = ans;
      phomyHistory = [...phomyHistory, { role: 'assistant', text: ans, refs }];
    } catch (e) {
      const errMsg = `Error: ${e}`;
      phomyAnswer = errMsg;
      phomyHistory = [...phomyHistory, { role: 'assistant', text: errMsg }];
    }
    phomyIsAsking = false;
  }

  async function expandPhomyContext() {
    if (phomyIsAsking || phomyHistory.length < 2) return;
    const lastUserMsg = [...phomyHistory].reverse().find(h => h.role === 'user');
    if (!lastUserMsg) return;

    const newLimit = Math.min(phomyContextLimit + 10, 30);
    phomyContextLimit = newLimit;
    phomyIsAsking = true;

    try {
      const refs = await meetingsStore.semanticSearch(lastUserMsg.text, undefined, newLimit);
      phomyReferences = refs;

      const ans = await invoke<string>("phomy_ask", { question: lastUserMsg.text });
      phomyAnswer = ans;
      phomyHistory = [
        ...phomyHistory.slice(0, -1),
        { role: 'assistant', text: ans, refs },
      ];
    } catch (e) {
      console.error("Expand context failed:", e);
    }
    phomyIsAsking = false;
  }

  // TextScramble - runs once on welcome screen mount
  const SCRAMBLE_CHARS = "!<>-_\\/[]{}—=+*^?#";
  const HEADLINE = "Always Listening. Never Seen.";

  function runTextScramble() {
    if (scrambleStarted) return;
    scrambleStarted = true;

    const finalChars = HEADLINE.split("");
    const totalFrames = 30;
    let frame = 0;

    // Initialize with all scrambled
    scrambleOutput = finalChars.map(c => c === " " ? { char: " ", scrambled: false } : { char: SCRAMBLE_CHARS[Math.floor(Math.random() * SCRAMBLE_CHARS.length)], scrambled: true });

    const interval = setInterval(() => {
      frame++;
      const progress = frame / totalFrames;

      scrambleOutput = finalChars.map((c, i) => {
        if (c === " ") return { char: " ", scrambled: false };
        // Characters resolve left to right with some randomness
        const charThreshold = (i / finalChars.length) * 0.8;
        if (progress > charThreshold + 0.2) {
          return { char: c, scrambled: false };
        }
        // Still scrambling
        return { char: SCRAMBLE_CHARS[Math.floor(Math.random() * SCRAMBLE_CHARS.length)], scrambled: true };
      });

      if (frame >= totalFrames) {
        clearInterval(interval);
        scrambleOutput = finalChars.map(c => ({ char: c, scrambled: false }));
        scrambleComplete = true;
      }
    }, 50);
  }

  function handleNavigate(view: View) {
    currentView = view;
    // When navigating to Home, clear past meeting state (unless actively recording)
    if (view === 'home') {
      if (isRecording && liveRecordingMeetingId) {
        // If recording, switch back to live view
        meetingsStore.setActive(liveRecordingMeetingId);
        meetingsStore.setActiveTranscript(transcript);
      } else if (!isRecording) {
        // If not recording, clear active meeting to show the start recording button
        meetingsStore.clearActive();
        transcript = [];
      }
    }
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
    answer = "";
    summary = null;
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

  async function handleExportMeeting(action: 'copy' | 'save' = 'copy') {
    const meetingId = meetingsStore.activeMeetingId;
    if (!meetingId) return;
    try {
      if (action === 'copy') {
        // Copy to clipboard (legacy behavior)
        const md = await meetingsStore.exportMeeting(meetingId, selectedExportFormat);
        await navigator.clipboard.writeText(md);
        exportCopied = true;
        setTimeout(() => { exportCopied = false; }, 2000);
      } else {
        // Save to file with dialog
        await invoke("export_meeting_to_file", {
          id: meetingId,
          format: selectedExportFormat
        });
        exportDropdownOpen = false;
      }
    } catch (e) {
      console.error("Failed to export meeting:", e);
    }
  }

  function toggleExportDropdown() {
    exportDropdownOpen = !exportDropdownOpen;
  }

  function selectExportFormat(format: string) {
    selectedExportFormat = format;
  }

  function handleSegmentUpdate(updatedSegment: TranscriptSegment) {
    // Update local transcript array
    transcript = transcript.map(seg =>
      seg.id === updatedSegment.id ? updatedSegment : seg
    );
    // Also update the store
    meetingsStore.setActiveTranscript(transcript);
  }

  function handleSegmentDelete(segmentId: string) {
    // Remove from local transcript array
    transcript = transcript.filter(seg => seg.id !== segmentId);
    // Also update the store
    meetingsStore.setActiveTranscript(transcript);
  }

  async function loadSpeakers() {
    try {
      speakers = await invoke<Speaker[]>("list_speakers");
    } catch (e) {
      console.error("Failed to load speakers:", e);
    }
  }

  function handleSearch(query: string) {
    meetingsStore.searchMeetings(query);
  }

  // Derived values for sidebar
  let pinnedMeetings = $derived(meetingsStore.getPinnedMeetings());
  let recentMeetings = $derived(meetingsStore.getRecentMeetings());
</script>

{#if showSplash}
  <div class="fixed inset-0 z-[100] flex flex-col items-center justify-center bg-phantom-ear-bg {splashFadingOut ? 'animate-splash-fade-out' : ''}">
    <!-- PhantomEar Logo -->
    <div class="animate-phantom-fly">
      <img
        src="/PhantomEarNoBackground.png"
        alt="PhantomEar"
        class="w-28 h-28 object-contain opacity-90"
      />
    </div>

  </div>
{/if}

{#if !showSplash && needsSetup}
  <Setup onComplete={handleSetupComplete} />
{:else if !showSplash && showOnboarding}
  <Onboarding onComplete={() => showOnboarding = false} />
{:else if !showSplash}
  <div class="flex h-screen bg-phantom-ear-bg no-select">
    <!-- Sidebar -->
    <Sidebar
      collapsed={sidebarCollapsed}
      {currentView}
      {pinnedMeetings}
      {recentMeetings}
      activeMeetingId={meetingsStore.activeMeetingId}
      {isRecording}
      onToggle={() => sidebarCollapsed = !sidebarCollapsed}
      onNavigate={handleNavigate}
      onSelectMeeting={handleSelectMeeting}
      onRenameMeeting={(id, title) => meetingsStore.renameMeeting(id, title)}
      onTogglePinMeeting={(id) => meetingsStore.togglePin(id)}
      onDeleteMeeting={(id) => meetingsStore.deleteMeeting(id)}
      onSearch={handleSearch}
      onOpenSearchOverlay={() => showSearchOverlay = true}
      onToggleRecording={toggleRecording}
      onUpdateMeetingTags={(id, tags) => meetingsStore.updateMeetingTags(id, tags)}
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
        showLiveIndicator={showRecordingIndicator}
        onReturnToLive={returnToLiveRecording}
      />

      <!-- Content Area -->
      <div class="flex-1 flex flex-col overflow-hidden">
        {#if currentView === 'home'}
          <!-- HOME VIEW: Clean CTA + Metrics (when NOT recording and NOT viewing past meeting) -->
          {#if !isRecording && !isViewingPastMeeting}
            <div class="flex-1 flex flex-col items-center justify-center p-6 relative overflow-hidden">
              <!-- Animated Background Orbs -->
              <div class="absolute inset-0 overflow-hidden pointer-events-none">
                <div class="absolute w-64 h-64 rounded-full bg-gradient-to-br from-phantom-ear-accent/10 to-phantom-ear-purple/5 blur-3xl animate-float animate-float-delay-1" style="top: 10%; left: 20%;"></div>
                <div class="absolute w-48 h-48 rounded-full bg-gradient-to-br from-phantom-ear-purple/10 to-phantom-ear-accent/5 blur-3xl animate-float-slow animate-float-delay-2" style="top: 60%; right: 15%;"></div>
                <div class="absolute w-32 h-32 rounded-full bg-gradient-to-br from-phantom-ear-accent/8 to-transparent blur-2xl animate-float-fast animate-float-delay-3" style="bottom: 20%; left: 10%;"></div>
              </div>

              <!-- Large centered recording button -->
              <div class="flex flex-col items-center relative z-10">
                <div class="relative">
                  <!-- Outer glow ring -->
                  <div class="absolute inset-0 w-24 h-24 rounded-full bg-gradient-accent opacity-20 blur-xl animate-glow-pulse"></div>
                  <!-- Ripple rings -->
                  <div class="absolute inset-0 w-24 h-24 rounded-full border-2 border-phantom-ear-accent/30 animate-ring-pulse"></div>
                  <button
                    onclick={toggleRecording}
                    class="relative w-24 h-24 rounded-full transition-all duration-300 btn-shine btn-ripple bg-gradient-accent animate-glow-pulse hover:scale-105 active:scale-95"
                    title="Start recording"
                  >
                    <svg class="w-8 h-8 mx-auto text-white drop-shadow-lg" fill="currentColor" viewBox="0 0 24 24">
                      <circle cx="12" cy="12" r="6" />
                    </svg>
                  </button>
                </div>

                <p class="mt-5 text-lg font-medium text-phantom-ear-text animate-scale-in">
                  Start Recording
                </p>
                <p class="mt-1.5 text-xs text-phantom-ear-text-muted animate-scale-in" style="animation-delay: 0.1s;">
                  Press {isMacOS() ? '⌘' : 'Ctrl'} + Shift + R to start
                </p>
              </div>

              <!-- Home Metrics Section -->
              <div class="mt-10 relative z-10 w-full flex justify-center">
                <HomeMetrics />
              </div>
            </div>
          {:else if isRecording}
            <!-- LIVE RECORDING VIEW -->
            <div class="flex-1 flex flex-col p-6 overflow-hidden">
              <!-- Timeline -->
              <div class="mb-4">
                <TranscriptTimeline
                  segments={transcript}
                  duration={recordingDuration}
                  currentPosition={recordingDuration}
                  isRecording={isRecording}
                  onSeek={(timestampMs) => {
                    const targetSec = timestampMs / 1000;
                    const closestSegment = transcript.reduce((prev, curr) => {
                      const prevDiff = Math.abs((prev.timestamp_ms || 0) / 1000 - targetSec);
                      const currDiff = Math.abs((curr.timestamp_ms || 0) / 1000 - targetSec);
                      return currDiff < prevDiff ? curr : prev;
                    }, transcript[0]);
                    if (closestSegment && transcriptContainer) {
                      const segmentEl = transcriptContainer.querySelector(`[data-segment-id="${closestSegment.id}"]`);
                      if (segmentEl) {
                        segmentEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
                        segmentEl.classList.add('bg-phantom-ear-accent/10');
                        setTimeout(() => segmentEl.classList.remove('bg-phantom-ear-accent/10'), 1500);
                      }
                    }
                  }}
                />
              </div>

              <!-- Live Transcript -->
              <div class="flex-1 flex flex-col min-h-0">
                <div class="flex items-center justify-between mb-3">
                  <h2 class="text-sm font-medium text-phantom-ear-text-muted uppercase tracking-wide">Live Transcript</h2>
                  <span class="text-xs text-phantom-ear-text-muted">{transcript.length} segments</span>
                </div>

                <div class="flex-1 glass rounded-xl border border-phantom-ear-border overflow-hidden shadow-glow-surface">
                  {#if transcript.length === 0}
                    <div class="flex flex-col items-center justify-center h-full text-phantom-ear-text-muted">
                      <p class="text-sm font-medium">Listening...</p>
                      <p class="text-xs mt-1 opacity-70">Speech will appear here in real-time</p>
                    </div>
                  {:else}
                    <div bind:this={transcriptContainer} class="p-4 space-y-2 overflow-y-auto h-full scroll-smooth">
                      {#each transcript as segment (segment.id)}
                        <div data-segment-id={segment.id} class="flex gap-3 animate-fade-in p-2 rounded-lg hover:bg-phantom-ear-surface/50 transition-colors">
                          <span class="text-xs text-phantom-ear-accent font-mono shrink-0 pt-0.5">{segment.time}</span>
                          <p class="text-sm leading-relaxed text-phantom-ear-text">{segment.text}</p>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>

                <!-- Q&A Section (available during live recording) -->
                {#if transcript.length > 0}
                  <div class="mt-4 p-4 glass rounded-xl border border-phantom-ear-border">
                    <div class="flex items-center gap-2 mb-3">
                      <svg class="w-4 h-4 text-phantom-ear-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                      </svg>
                      <h3 class="text-xs font-medium text-phantom-ear-text-muted uppercase tracking-wide">Ask About This Recording</h3>
                    </div>
                    <form onsubmit={(e) => { e.preventDefault(); askQuestion(); }} class="flex gap-2">
                      <input
                        type="text"
                        bind:value={question}
                        placeholder="Ask a question about the transcript so far..."
                        class="flex-1 px-3 py-2 text-sm bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none focus:border-phantom-ear-accent transition-colors"
                        disabled={isAsking}
                      />
                      <button
                        type="submit"
                        disabled={!question.trim() || isAsking}
                        class="px-4 py-2 bg-gradient-accent rounded-lg text-sm font-medium text-white disabled:opacity-50 transition-all hover-lift"
                      >
                        {#if isAsking}
                          <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                          </svg>
                        {:else}
                          Ask
                        {/if}
                      </button>
                    </form>
                    {#if answer}
                      <div class="mt-3 p-3 bg-phantom-ear-surface/50 rounded-lg">
                        <p class="text-sm text-phantom-ear-text leading-relaxed">{answer}</p>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            </div>
          {:else if isViewingPastMeeting}
            <!-- PAST MEETING VIEW (Read-only) -->
            <div class="flex-1 flex flex-col p-6 overflow-hidden">
              <!-- Past Meeting Header -->
              <div class="flex items-center justify-between mb-4">
                <div class="flex items-center gap-2">
                  <span class="px-2 py-0.5 text-xs rounded bg-phantom-ear-surface text-phantom-ear-text-muted">Past Meeting</span>
                  <h2 class="text-sm font-medium text-phantom-ear-text">
                    {meetingsStore.activeMeeting?.title || 'Untitled Meeting'}
                  </h2>
                </div>
                <div class="flex items-center gap-2">
                  {#if transcript.length > 0}
                    <div class="relative" onkeydown={(e) => e.key === 'Escape' && (exportDropdownOpen = false)}>
                      <button
                        onclick={toggleExportDropdown}
                        class="px-2 py-1 text-xs rounded-md bg-phantom-ear-surface border border-phantom-ear-border text-phantom-ear-text-muted hover:text-phantom-ear-text hover:border-phantom-ear-accent transition-colors flex items-center gap-1"
                      >
                        {exportCopied ? 'Copied!' : 'Export'}
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                        </svg>
                      </button>
                      {#if exportDropdownOpen}
                        <!-- Click outside to close -->
                        <div 
                          class="fixed inset-0 z-0" 
                          onclick={() => exportDropdownOpen = false}
                          onkeydown={() => {}}
                          role="button"
                          tabindex="-1"
                        ></div>
                        <div class="absolute right-0 top-full mt-1 w-36 bg-phantom-ear-surface border border-phantom-ear-border rounded-lg shadow-lg z-10 overflow-hidden">
                          <div class="px-2 py-1.5 text-[10px] text-phantom-ear-text-muted border-b border-phantom-ear-border">
                            Format
                          </div>
                          <button
                            onclick={() => { selectExportFormat('markdown'); handleExportMeeting('copy'); }}
                            class="w-full px-3 py-2 text-left text-xs hover:bg-phantom-ear-surface-hover transition-colors {selectedExportFormat === 'markdown' ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}"
                          >
                            📄 Markdown
                          </button>
                          <button
                            onclick={() => { selectExportFormat('txt'); handleExportMeeting('copy'); }}
                            class="w-full px-3 py-2 text-left text-xs hover:bg-phantom-ear-surface-hover transition-colors {selectedExportFormat === 'txt' ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}"
                          >
                            📝 Plain Text
                          </button>
                          <button
                            onclick={() => { selectExportFormat('srt'); handleExportMeeting('copy'); }}
                            class="w-full px-3 py-2 text-left text-xs hover:bg-phantom-ear-surface-hover transition-colors {selectedExportFormat === 'srt' ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}"
                          >
                            🎬 Subtitle (SRT)
                          </button>
                          <div class="border-t border-phantom-ear-border"></div>
                          <button
                            onclick={() => handleExportMeeting('save')}
                            class="w-full px-3 py-2 text-left text-xs hover:bg-phantom-ear-surface-hover transition-colors text-phantom-ear-accent"
                          >
                            💾 Save As...
                          </button>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>

              <!-- Transcript -->
              <div class="flex-1 flex flex-col min-h-0">
                <div class="flex-1 glass rounded-xl border border-phantom-ear-border overflow-hidden">
                  {#if transcript.length === 0}
                    <div class="flex flex-col items-center justify-center h-full text-phantom-ear-text-muted">
                      <p class="text-sm">No transcript available</p>
                    </div>
                  {:else}
                    <div bind:this={transcriptContainer} class="p-4 space-y-1 overflow-y-auto h-full scroll-smooth">
                      {#each transcript as segment (segment.id)}
                        <EditableSegment
                          {segment}
                          {speakers}
                          onUpdate={handleSegmentUpdate}
                          onDelete={handleSegmentDelete}
                          onSpeakersChange={loadSpeakers}
                        />
                      {/each}

                      <!-- Summary Display -->
                      {#if summary}
                        <div class="mt-4 p-4 rounded-xl bg-phantom-ear-purple/5 border border-phantom-ear-purple/20">
                          <div class="flex items-center gap-2 mb-3">
                            <div class="w-5 h-5 rounded-full bg-phantom-ear-purple flex items-center justify-center">
                              <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                              </svg>
                            </div>
                            <h3 class="text-xs font-medium text-phantom-ear-text-muted uppercase tracking-wide">Meeting Summary</h3>
                          </div>
                          {#if summary.overview}
                            <p class="text-sm text-phantom-ear-text leading-relaxed mb-4">{summary.overview}</p>
                          {/if}
                          {#if summary.key_points.length > 0}
                            <div class="mb-3">
                              <h4 class="text-xs font-semibold text-phantom-ear-text-muted uppercase tracking-wide mb-2">Key Points</h4>
                              <ul class="space-y-1">
                                {#each summary.key_points as point}
                                  <li class="flex items-start gap-2 text-sm text-phantom-ear-text">
                                    <span class="text-phantom-ear-accent mt-1">&#8226;</span>
                                    <span>{point}</span>
                                  </li>
                                {/each}
                              </ul>
                            </div>
                          {/if}
                          {#if summary.action_items.length > 0}
                            <div>
                              <h4 class="text-xs font-semibold text-phantom-ear-text-muted uppercase tracking-wide mb-2">Action Items</h4>
                              <ul class="space-y-1">
                                {#each summary.action_items as item}
                                  <li class="flex items-start gap-2 text-sm text-phantom-ear-text">
                                    <span class="text-phantom-ear-success mt-1">&#10003;</span>
                                    <span>{item}</span>
                                  </li>
                                {/each}
                              </ul>
                            </div>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>

                <!-- Q&A Section -->
                {#if transcript.length > 0}
                  <div class="mt-4 p-4 glass rounded-xl border border-phantom-ear-border">
                    <div class="flex items-center gap-2 mb-3">
                      <svg class="w-4 h-4 text-phantom-ear-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                      </svg>
                      <h3 class="text-xs font-medium text-phantom-ear-text-muted uppercase tracking-wide">Ask About This Meeting</h3>
                    </div>
                    <form onsubmit={(e) => { e.preventDefault(); askQuestion(); }} class="flex gap-2">
                      <input
                        type="text"
                        bind:value={question}
                        placeholder="Ask a question about this meeting..."
                        class="flex-1 px-3 py-2 text-sm bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none focus:border-phantom-ear-accent transition-colors"
                        disabled={isAsking}
                      />
                      <button
                        type="submit"
                        disabled={!question.trim() || isAsking}
                        class="px-4 py-2 bg-gradient-accent rounded-lg text-sm font-medium text-white disabled:opacity-50 transition-all hover-lift"
                      >
                        {#if isAsking}
                          <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                          </svg>
                        {:else}
                          Ask
                        {/if}
                      </button>
                    </form>
                    {#if answer}
                      <div class="mt-3 p-3 bg-phantom-ear-surface/50 rounded-lg">
                        <p class="text-sm text-phantom-ear-text leading-relaxed">{answer}</p>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            </div>
          {/if}

        {:else if currentView === 'phomy'}
          <div class="flex-1 flex flex-col p-6 overflow-hidden">
            <!-- Phomy Header -->
            <div class="flex items-center gap-3 mb-4">
              <div class="w-10 h-10 rounded-xl bg-phantom-ear-purple/20 flex items-center justify-center">
                <svg class="w-5 h-5 text-phantom-ear-purple" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 2C7.58 2 4 5.58 4 10v9c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1V10c0-4.42-3.58-8-8-8zm-2 10a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm4 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
                </svg>
              </div>
              <div>
                <h2 class="text-base font-semibold text-phantom-ear-text">Phomy</h2>
                <p class="text-xs text-phantom-ear-text-muted">Your meeting memory</p>
              </div>
              {#if embeddingDownloading}
                <span class="ml-auto text-xs text-phantom-ear-text-muted flex items-center gap-1">
                  <svg class="w-3 h-3 text-phantom-ear-purple opacity-50 animate-pulse" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M12 2C7.58 2 4 5.58 4 10v9c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1V10c0-4.42-3.58-8-8-8zm-2 10a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm4 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
                  </svg>
                  Loading embeddings...
                </span>
              {:else if embeddingDownloadFailed}
                <button
                  onclick={openEmbeddingManualDownload}
                  class="ml-auto text-xs text-phantom-ear-warning hover:text-phantom-ear-text transition-colors flex items-center gap-1"
                >
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                  </svg>
                  Download blocked - click for manual
                </button>
              {:else if !embeddingModelLoaded}
                <span class="ml-auto text-xs text-phantom-ear-text-muted">Embedding model not loaded</span>
              {/if}
            </div>

            <!-- Chat History -->
            <div class="flex-1 glass rounded-xl border border-phantom-ear-border overflow-y-auto p-4 space-y-4">
              {#if phomyHistory.length === 0}
                <div class="flex flex-col items-center justify-center h-full text-phantom-ear-text-muted">
                  <div class="w-14 h-14 mb-4 rounded-2xl bg-phantom-ear-purple/10 flex items-center justify-center">
                    <svg class="w-7 h-7 opacity-40 text-phantom-ear-purple" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 2C7.58 2 4 5.58 4 10v9c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1V10c0-4.42-3.58-8-8-8zm-2 10a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm4 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
                    </svg>
                  </div>
                  <p class="text-sm font-medium">Ask Phomy anything about your meetings.</p>
                  <p class="text-xs mt-1 opacity-70">Recalls what was said, summarizes time ranges, and searches across all recordings.</p>
                </div>
              {:else}
                {#each phomyHistory as msg}
                  {#if msg.role === 'user'}
                    <div class="flex justify-end">
                      <div class="max-w-[80%] px-4 py-2.5 rounded-2xl rounded-br-sm bg-phantom-ear-accent text-white text-sm">
                        {msg.text}
                      </div>
                    </div>
                  {:else}
                    <div class="space-y-3">
                      <div class="max-w-[80%] px-4 py-2.5 rounded-2xl rounded-bl-sm bg-phantom-ear-surface border border-phantom-ear-border text-sm text-phantom-ear-text whitespace-pre-wrap">
                        {msg.text}
                      </div>
                      {#if msg.refs && msg.refs.length > 0}
                        <div class="space-y-2 max-w-[80%]">
                          <p class="text-xs text-phantom-ear-text-muted uppercase tracking-wide font-medium">References</p>
                          {#each msg.refs.slice(0, 5) as ref}
                            <ReferenceCard result={ref} onSelect={handleSelectMeeting} />
                          {/each}
                        </div>
                      {/if}
                    </div>
                  {/if}
                {/each}

                {#if phomyIsAsking}
                  <div class="flex items-center gap-2 px-4 py-2 text-phantom-ear-text-muted text-sm">
                    <svg class="w-4 h-4 text-phantom-ear-purple animate-pulse" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 2C7.58 2 4 5.58 4 10v9c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1V10c0-4.42-3.58-8-8-8zm-2 10a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm4 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
                    </svg>
                    Thinking...
                  </div>
                {/if}
              {/if}
            </div>

            <!-- Expand Context Button -->
            {#if phomyHistory.length > 0 && phomyContextLimit < 30 && !phomyIsAsking}
              <div class="flex justify-center mt-2">
                <button
                  onclick={expandPhomyContext}
                  class="px-3 py-1.5 text-xs rounded-lg bg-phantom-ear-surface border border-phantom-ear-border text-phantom-ear-text-muted hover:text-phantom-ear-text hover:border-phantom-ear-purple/40 transition-colors"
                >
                  Show more context ({phomyContextLimit + 10} chunks)
                </button>
              </div>
            {/if}

            <!-- Phomy Input Bar -->
            <div class="mt-4">
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  askPhomy();
                }}
                class="relative flex items-center"
              >
                <input
                  type="text"
                  bind:value={phomyQuestion}
                  placeholder={embeddingModelLoaded ? "Ask Phomy about your meetings..." : "Loading embedding model..."}
                  disabled={!embeddingModelLoaded}
                  class="w-full pl-4 pr-14 py-3.5 glass border border-phantom-ear-border rounded-2xl text-sm text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none focus:border-phantom-ear-purple/50 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                />
                <div class="absolute right-2">
                  <button
                    type="submit"
                    disabled={!phomyQuestion.trim() || phomyIsAsking || !embeddingModelLoaded}
                    class="p-2 rounded-xl bg-phantom-ear-purple text-white hover:opacity-80 transition-all disabled:opacity-20 disabled:cursor-not-allowed"
                    title="Ask Phomy"
                  >
                    {#if phomyIsAsking}
                      <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                      </svg>
                    {:else}
                      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18" />
                      </svg>
                    {/if}
                  </button>
                </div>
              </form>
            </div>
          </div>

        {:else if currentView === 'settings'}
          <div class="flex-1 min-h-0 overflow-hidden">
            <Settings onClose={handleSettingsSaved} inline={true} onShowOnboarding={() => showOnboarding = true} />
          </div>
        {/if}
      </div>
    </main>
  </div>

  <!-- Download Progress Overlay -->
  {#if downloadingModel}
    <div class="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center">
      <div class="bg-phantom-ear-surface rounded-2xl border border-phantom-ear-border p-6 w-80 shadow-glow-surface">
        <div class="flex items-center gap-3 mb-4">
          <div class="w-10 h-10 rounded-xl bg-phantom-ear-accent/20 flex items-center justify-center">
            <svg class="w-5 h-5 text-phantom-ear-accent animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
          </div>
          <div>
            <h3 class="text-sm font-semibold text-phantom-ear-text">Downloading Model</h3>
            <p class="text-xs text-phantom-ear-text-muted capitalize">{downloadingModel}</p>
          </div>
        </div>

        {#if downloadProgress}
          <div class="space-y-2">
            <div class="h-2 bg-phantom-ear-border rounded-full overflow-hidden">
              <div
                class="h-full bg-gradient-accent transition-all duration-300"
                style="width: {downloadProgress.percentage}%"
              ></div>
            </div>
            <div class="flex justify-between text-xs text-phantom-ear-text-muted">
              <span>{downloadProgress.status}</span>
              <span>{downloadProgress.percentage.toFixed(0)}%</span>
            </div>
          </div>
        {:else}
          <div class="flex items-center justify-center py-2">
            <div class="w-5 h-5 border-2 border-phantom-ear-accent border-t-transparent rounded-full animate-spin"></div>
            <span class="ml-2 text-sm text-phantom-ear-text-muted">Preparing...</span>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Embedding Model Manual Download Modal -->
  {#if showEmbeddingManualDownload}
    <div
      class="fixed inset-0 bg-black/70 backdrop-blur-md z-40"
      onclick={() => showEmbeddingManualDownload = false}
      onkeydown={(e) => e.key === "Escape" && (showEmbeddingManualDownload = false)}
      role="button"
      tabindex="-1"
    ></div>
    <div class="fixed inset-4 md:inset-auto md:top-1/2 md:left-1/2 md:-translate-x-1/2 md:-translate-y-1/2 md:w-[450px] glass-strong rounded-2xl border border-phantom-ear-border shadow-glow-surface z-50 flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-phantom-ear-border/50">
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-lg bg-phantom-ear-purple/20 flex items-center justify-center">
            <svg class="w-4 h-4 text-phantom-ear-purple" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
          </div>
          <h2 class="text-lg font-semibold text-phantom-ear-text">Manual Embedding Model Download</h2>
        </div>
        <button
          onclick={() => showEmbeddingManualDownload = false}
          class="p-2 rounded-lg hover:bg-phantom-ear-surface-hover transition-colors"
        >
          <svg class="w-5 h-5 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="p-6 space-y-4">
        <p class="text-sm text-phantom-ear-text-muted">
          The automatic download was blocked by a corporate firewall. Download the files manually and import them below.
        </p>

        <div class="bg-phantom-ear-surface/50 border border-phantom-ear-border/50 rounded-xl p-4 space-y-3">
          <div class="flex items-start gap-3">
            <div class="w-6 h-6 rounded-full bg-phantom-ear-purple/20 flex items-center justify-center shrink-0 mt-0.5">
              <span class="text-xs font-bold text-phantom-ear-purple">1</span>
            </div>
            <div class="flex-1">
              <p class="text-sm text-phantom-ear-text">Download these two files:</p>
              <ul class="mt-2 space-y-1 text-xs text-phantom-ear-text-muted">
                <li class="flex items-center gap-2">
                  <span class="w-1.5 h-1.5 rounded-full bg-phantom-ear-purple"></span>
                  model.onnx (~33MB)
                </li>
                <li class="flex items-center gap-2">
                  <span class="w-1.5 h-1.5 rounded-full bg-phantom-ear-purple"></span>
                  tokenizer.json (~700KB)
                </li>
              </ul>
            </div>
          </div>
          <div class="flex items-start gap-3">
            <div class="w-6 h-6 rounded-full bg-phantom-ear-purple/20 flex items-center justify-center shrink-0 mt-0.5">
              <span class="text-xs font-bold text-phantom-ear-purple">2</span>
            </div>
            <p class="text-sm text-phantom-ear-text-muted text-left">
              Click <span class="text-phantom-ear-text">Import Files</span> below and select both downloaded files (or a .zip containing them)
            </p>
          </div>
        </div>

        <div class="flex gap-3">
          <button
            onclick={openEmbeddingManualDownload}
            class="flex-1 py-2.5 px-4 border border-phantom-ear-border rounded-xl text-sm text-phantom-ear-text-muted hover:text-phantom-ear-text hover:border-phantom-ear-text-muted transition-colors"
          >
            Re-open Links
          </button>
          <button
            onclick={importEmbeddingModel}
            disabled={embeddingImporting}
            class="flex-1 py-2.5 px-4 bg-gradient-accent hover:bg-gradient-accent-hover rounded-xl text-sm font-medium text-white transition-all hover-lift btn-shine disabled:opacity-50"
          >
            {#if embeddingImporting}
              Importing...
            {:else}
              Import Files
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}
{/if}

<!-- Search Overlay -->
<SearchOverlay
  isOpen={showSearchOverlay}
  onClose={() => showSearchOverlay = false}
/>

<!-- Meeting Detection Notification -->
{#if autoDetectMeetings}
  <MeetingNotification
    onStartRecording={toggleRecording}
    {isRecording}
  />
{/if}
