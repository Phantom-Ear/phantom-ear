<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";

  interface Settings {
    llm_provider: string;
    openai_api_key: string | null;
    ollama_url: string | null;
    ollama_model: string | null;
    auto_detect_meetings: boolean;
    show_system_notifications: boolean;
    onboarding_completed: boolean;
    whisper_model: string;
    language: string;
    asr_backend: string;
    audio_device: string | null;
  }

  interface ModelInfo {
    name: string;
    size_mb: number;
    downloaded: boolean;
    description: string;
  }

  interface BackendInfo {
    backend_type: string;
    name: string;
    description: string;
    supported_languages: string[];
  }

  interface AudioDeviceInfo {
    name: string;
    is_default: boolean;
  }

  let { onClose, inline = false, onShowOnboarding }: { onClose: () => void; inline?: boolean; onShowOnboarding?: () => void } = $props();

  let settings = $state<Settings>({
    llm_provider: "ollama",
    openai_api_key: null,
    ollama_url: "http://localhost:11434",
    ollama_model: "llama3.2",
    auto_detect_meetings: false,
    show_system_notifications: true,
    onboarding_completed: false,
    whisper_model: "base",
    language: "en",
    asr_backend: "whisper",
    audio_device: null,
  });

  let asrBackends = $state<BackendInfo[]>([]);
  let audioDevices = $state<AudioDeviceInfo[]>([]);

  let models = $state<ModelInfo[]>([]);
  let isLoading = $state(true);
  let isSaving = $state(false);
  let isImporting = $state(false);
  let importError = $state("");
  let importSuccess = $state("");
  let activeTab = $state<"general" | "llm">("general");

  // Permission state for meeting detection
  let isMacOS = $state(false);
  let hasScreenRecordingPermission = $state<boolean | null>(null);
  let isCheckingPermission = $state(false);

  const languages = [
    { code: "auto", name: "Auto-detect" },
    { code: "en", name: "English" },
    { code: "fr", name: "French" },
    { code: "es", name: "Spanish" },
    { code: "de", name: "German" },
    { code: "it", name: "Italian" },
    { code: "pt", name: "Portuguese" },
    { code: "nl", name: "Dutch" },
    { code: "pl", name: "Polish" },
    { code: "ru", name: "Russian" },
    { code: "ja", name: "Japanese" },
    { code: "ko", name: "Korean" },
    { code: "zh", name: "Chinese" },
    { code: "ar", name: "Arabic" },
  ];

  async function loadSettings() {
    try {
      const [loadedSettings, loadedModels, loadedBackends, loadedDevices] = await Promise.all([
        invoke<Settings>("get_settings"),
        invoke<ModelInfo[]>("get_models_info"),
        invoke<BackendInfo[]>("get_asr_backends"),
        invoke<AudioDeviceInfo[]>("list_audio_devices"),
      ]);
      settings = loadedSettings;
      models = loadedModels;
      asrBackends = loadedBackends;
      audioDevices = loadedDevices;

      // Check platform using navigator (works in Tauri webview)
      isMacOS = navigator.platform.toLowerCase().includes("mac");

      // Check screen recording permission on macOS
      if (isMacOS) {
        await checkPermission();
      }
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
    isLoading = false;
  }

  async function checkPermission() {
    isCheckingPermission = true;
    try {
      hasScreenRecordingPermission = await invoke<boolean>("check_screen_recording_permission");
    } catch (e) {
      console.error("Failed to check permission:", e);
      hasScreenRecordingPermission = false;
    }
    isCheckingPermission = false;
  }

  async function openScreenRecordingSettings() {
    try {
      await invoke("open_screen_recording_settings");
      // Re-check permission after a delay (user may grant it)
      setTimeout(async () => {
        await checkPermission();
      }, 2000);
    } catch (e) {
      console.error("Failed to open settings:", e);
    }
  }

  async function saveSettings() {
    isSaving = true;
    try {
      await invoke("save_settings", { settings });
      await invoke("load_model", { modelName: settings.whisper_model });
      onClose();
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
    isSaving = false;
  }

  async function importModelFile() {
    importError = "";
    importSuccess = "";
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Whisper Model", extensions: ["bin", "zip"] }],
      });
      if (!selected) return;

      isImporting = true;
      await invoke("import_model", {
        filePath: selected,
        modelName: settings.whisper_model,
      });
      importSuccess = "Model imported successfully!";
      models = await invoke<ModelInfo[]>("get_models_info");
    } catch (e: any) {
      importError = typeof e === "string" ? e : e.message || "Import failed";
    }
    isImporting = false;
  }

  $effect(() => {
    loadSettings();
  });
</script>

{#if !inline}
  <div
    class="fixed inset-0 bg-black/70 backdrop-blur-md z-40"
    onclick={onClose}
    onkeydown={(e) => e.key === "Escape" && onClose()}
    role="button"
    tabindex="-1"
  ></div>
{/if}

<!-- Modal / Inline Container -->
<div class="{inline ? 'flex flex-col h-full min-h-0 overflow-hidden' : 'fixed inset-4 md:inset-auto md:top-1/2 md:left-1/2 md:-translate-x-1/2 md:-translate-y-1/2 md:w-[500px] md:max-h-[80vh] glass-strong rounded-2xl border border-phantom-ear-border shadow-glow-surface z-50 flex flex-col overflow-hidden'}">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 border-b border-phantom-ear-border/50 shrink-0">
    <div class="flex items-center gap-2">
      <svg class="w-5 h-5 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
      <h2 class="text-sm font-semibold text-phantom-ear-text">Settings</h2>
    </div>
    {#if !inline}
      <button
        onclick={onClose}
        class="p-1.5 rounded-lg hover:bg-phantom-ear-surface-hover transition-colors"
        title="Close"
      >
        <svg class="w-4 h-4 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    {/if}
  </div>

  {#if isLoading}
    <div class="flex-1 flex items-center justify-center py-12">
      <div class="w-6 h-6 border-2 border-phantom-ear-accent border-t-transparent rounded-full animate-spin"></div>
    </div>
  {:else}
    <!-- Tabs -->
    <div class="flex border-b border-phantom-ear-border/50 px-3 shrink-0">
      <button
        onclick={() => activeTab = "general"}
        class="relative px-3 py-2 text-xs font-medium transition-colors {activeTab === 'general' ? 'text-phantom-ear-accent' : 'text-phantom-ear-text-muted hover:text-phantom-ear-text'}"
      >
        General
        {#if activeTab === 'general'}
          <div class="absolute bottom-0 left-1 right-1 h-0.5 bg-phantom-ear-accent rounded-full"></div>
        {/if}
      </button>
      <button
        onclick={() => activeTab = "llm"}
        class="relative px-3 py-2 text-xs font-medium transition-colors {activeTab === 'llm' ? 'text-phantom-ear-accent' : 'text-phantom-ear-text-muted hover:text-phantom-ear-text'}"
      >
        LLM
        {#if activeTab === 'llm'}
          <div class="absolute bottom-0 left-1 right-1 h-0.5 bg-phantom-ear-accent rounded-full"></div>
        {/if}
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 min-h-0 overflow-y-auto p-4 space-y-4">
      {#if activeTab === "general"}
        <!-- Language & Audio Row -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">Language</label>
            <select
              bind:value={settings.language}
              class="w-full px-3 py-2 bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-sm text-phantom-ear-text focus:outline-none focus:border-phantom-ear-accent transition-colors"
            >
              {#each languages as lang}
                <option value={lang.code}>{lang.name}</option>
              {/each}
            </select>
          </div>
          <div>
            <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">Audio Device</label>
            <select
              bind:value={settings.audio_device}
              class="w-full px-3 py-2 bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-sm text-phantom-ear-text focus:outline-none focus:border-phantom-ear-accent transition-colors"
            >
              <option value={null}>Default</option>
              {#each audioDevices as device}
                <option value={device.name}>{device.name}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- Meeting Detection Section -->
        <div class="space-y-3">
          <label class="block text-xs font-medium text-phantom-ear-text-muted">Meeting Detection</label>

          <!-- Auto-detect Toggle -->
          <div class="flex items-center justify-between p-3 bg-phantom-ear-surface/50 rounded-xl border border-phantom-ear-border/50">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-lg bg-green-500/10 flex items-center justify-center">
                <svg class="w-4 h-4 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
                </svg>
              </div>
              <div>
                <span class="block text-sm font-medium text-phantom-ear-text">Auto-detect Meetings</span>
                <span class="block text-xs text-phantom-ear-text-muted">Detects Zoom, Teams, Meet, etc.</span>
              </div>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input type="checkbox" bind:checked={settings.auto_detect_meetings} class="sr-only peer" />
              <div class="w-10 h-5 bg-phantom-ear-surface border border-phantom-ear-border rounded-full peer peer-checked:bg-green-500 peer-checked:border-green-500 after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-phantom-ear-text-muted after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-5 peer-checked:after:bg-white"></div>
            </label>
          </div>

          <!-- System Notifications Toggle (only show when auto-detect is on) -->
          {#if settings.auto_detect_meetings}
            <div class="flex items-center justify-between p-3 bg-phantom-ear-surface/50 rounded-xl border border-phantom-ear-border/50">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-lg bg-blue-500/10 flex items-center justify-center">
                  <svg class="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
                  </svg>
                </div>
                <div>
                  <span class="block text-sm font-medium text-phantom-ear-text">System Notifications</span>
                  <span class="block text-xs text-phantom-ear-text-muted">Show OS notifications when meeting detected</span>
                </div>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input type="checkbox" bind:checked={settings.show_system_notifications} class="sr-only peer" />
                <div class="w-10 h-5 bg-phantom-ear-surface border border-phantom-ear-border rounded-full peer peer-checked:bg-blue-500 peer-checked:border-blue-500 after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-phantom-ear-text-muted after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-5 peer-checked:after:bg-white"></div>
              </label>
            </div>

            <!-- macOS Screen Recording Permission Notice -->
            {#if isMacOS}
              <div class="p-3 rounded-xl border {hasScreenRecordingPermission === false ? 'bg-amber-500/10 border-amber-500/30' : hasScreenRecordingPermission === true ? 'bg-green-500/10 border-green-500/30' : 'bg-phantom-ear-surface/50 border-phantom-ear-border/50'}">
                <div class="flex items-start gap-3">
                  {#if hasScreenRecordingPermission === null || isCheckingPermission}
                    <!-- Checking -->
                    <div class="w-8 h-8 rounded-lg bg-phantom-ear-surface flex items-center justify-center shrink-0">
                      <svg class="w-4 h-4 text-phantom-ear-text-muted animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                      </svg>
                    </div>
                    <div class="flex-1">
                      <span class="block text-sm font-medium text-phantom-ear-text">Checking permission...</span>
                    </div>
                  {:else if hasScreenRecordingPermission === false}
                    <!-- Permission not granted -->
                    <div class="w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center shrink-0">
                      <svg class="w-4 h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                      </svg>
                    </div>
                    <div class="flex-1 min-w-0">
                      <span class="block text-sm font-medium text-amber-400">Screen Recording Required</span>
                      <p class="text-xs text-phantom-ear-text-muted mt-1">
                        Required to detect meetings by reading window titles.
                      </p>
                      <div class="flex items-center gap-2 mt-2">
                        <button
                          onclick={openScreenRecordingSettings}
                          class="px-3 py-1.5 rounded-lg text-xs font-medium bg-amber-500 hover:bg-amber-600 text-white transition-colors"
                        >
                          Open Settings
                        </button>
                        <button
                          onclick={checkPermission}
                          class="px-3 py-1.5 rounded-lg text-xs font-medium border border-phantom-ear-border text-phantom-ear-text-muted hover:text-phantom-ear-text transition-colors"
                        >
                          Recheck
                        </button>
                      </div>
                    </div>
                  {:else}
                    <!-- Permission granted -->
                    <div class="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center shrink-0">
                      <svg class="w-4 h-4 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                      </svg>
                    </div>
                    <div class="flex-1">
                      <span class="block text-sm font-medium text-green-400">Screen Recording Enabled</span>
                      <span class="block text-xs text-phantom-ear-text-muted mt-0.5">Detection is ready</span>
                    </div>
                  {/if}
                </div>
              </div>
            {/if}
          {/if}
        </div>

        <!-- ASR Backend -->
        <div>
          <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">Speech Recognition</label>
          <div class="space-y-1.5">
            {#each asrBackends as backend}
              {@const isParakeet = backend.backend_type === "parakeet"}
              <label
                class="flex items-center gap-2.5 px-3 py-2 rounded-lg border cursor-pointer transition-colors {settings.asr_backend === backend.backend_type ? 'border-phantom-ear-accent bg-phantom-ear-accent/10' : 'border-phantom-ear-border hover:border-phantom-ear-text-muted'} {isParakeet ? 'opacity-50' : ''}"
              >
                <input
                  type="radio"
                  name="asr_backend"
                  value={backend.backend_type}
                  bind:group={settings.asr_backend}
                  disabled={isParakeet}
                  class="sr-only"
                />
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="text-sm text-phantom-ear-text">{backend.name}</span>
                    {#if isParakeet}
                      <span class="text-[10px] px-1.5 py-0.5 rounded bg-phantom-ear-warning/20 text-phantom-ear-warning">Soon</span>
                    {/if}
                  </div>
                </div>
                <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center shrink-0 {settings.asr_backend === backend.backend_type ? 'border-phantom-ear-accent' : 'border-phantom-ear-border'}">
                  {#if settings.asr_backend === backend.backend_type}
                    <div class="w-1.5 h-1.5 rounded-full bg-phantom-ear-accent"></div>
                  {/if}
                </div>
              </label>
            {/each}
          </div>
        </div>

        <!-- Whisper Model (only show when Whisper is selected) -->
        {#if settings.asr_backend === "whisper"}
          <div>
            <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">Whisper Model</label>
            <div class="space-y-1.5">
              {#each models as model}
                <label
                  class="flex items-center gap-2.5 px-3 py-2 rounded-lg border cursor-pointer transition-colors {settings.whisper_model === model.name ? 'border-phantom-ear-accent bg-phantom-ear-accent/10' : 'border-phantom-ear-border hover:border-phantom-ear-text-muted'}"
                >
                  <input
                    type="radio"
                    name="whisper_model"
                    value={model.name}
                    bind:group={settings.whisper_model}
                    class="sr-only"
                  />
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                      <span class="text-sm text-phantom-ear-text capitalize">{model.name}</span>
                      <span class="text-[10px] text-phantom-ear-text-muted">{model.size_mb}MB</span>
                      {#if model.downloaded}
                        <span class="text-[10px] text-phantom-ear-success">Ready</span>
                      {/if}
                    </div>
                  </div>
                  <div class="w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center shrink-0 {settings.whisper_model === model.name ? 'border-phantom-ear-accent' : 'border-phantom-ear-border'}">
                    {#if settings.whisper_model === model.name}
                      <div class="w-1.5 h-1.5 rounded-full bg-phantom-ear-accent"></div>
                    {/if}
                  </div>
                </label>
              {/each}
            </div>

            <!-- Import Model -->
            <button
              onclick={importModelFile}
              disabled={isImporting}
              class="mt-2 px-2.5 py-1.5 rounded text-[11px] font-medium border border-phantom-ear-border text-phantom-ear-text-muted hover:text-phantom-ear-text hover:border-phantom-ear-text-muted transition-colors disabled:opacity-50"
            >
              {isImporting ? 'Importing...' : 'Import .bin file'}
            </button>
            {#if importError}
              <p class="mt-1 text-[11px] text-phantom-ear-danger">{importError}</p>
            {/if}
            {#if importSuccess}
              <p class="mt-1 text-[11px] text-phantom-ear-success">{importSuccess}</p>
            {/if}
          </div>
        {/if}

        <!-- Help Section -->
        <div class="pt-3 border-t border-phantom-ear-border/50">
          <label class="block text-xs font-medium text-phantom-ear-text-muted mb-2">Help</label>
          <div class="flex gap-2">
            {#if onShowOnboarding}
              <button
                onclick={() => {
                  onShowOnboarding();
                  onClose();
                }}
                class="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-phantom-ear-border hover:border-phantom-ear-accent/50 hover:bg-phantom-ear-accent/5 transition-colors"
              >
                <svg class="w-4 h-4 text-phantom-ear-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span class="text-xs text-phantom-ear-text">Tour</span>
              </button>
            {/if}
            <button
              onclick={() => openUrl("https://github.com/Phantom-Ear/phantom-ear")}
              class="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-phantom-ear-border hover:border-phantom-ear-text-muted transition-colors"
            >
              <svg class="w-4 h-4 text-phantom-ear-text-muted" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
              </svg>
              <span class="text-xs text-phantom-ear-text">GitHub</span>
            </button>
          </div>
        </div>

      {:else if activeTab === "llm"}
        <!-- LLM Provider Pills -->
        <div>
          <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">Provider</label>
          <div class="flex gap-2">
            <button
              onclick={() => settings.llm_provider = "ollama"}
              class="flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors {settings.llm_provider === 'ollama' ? 'bg-phantom-ear-accent text-white' : 'bg-phantom-ear-bg border border-phantom-ear-border text-phantom-ear-text hover:border-phantom-ear-text-muted'}"
            >
              Ollama
            </button>
            <button
              onclick={() => settings.llm_provider = "openai"}
              class="flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors {settings.llm_provider === 'openai' ? 'bg-phantom-ear-accent text-white' : 'bg-phantom-ear-bg border border-phantom-ear-border text-phantom-ear-text hover:border-phantom-ear-text-muted'}"
            >
              OpenAI
            </button>
          </div>
        </div>

        {#if settings.llm_provider === "ollama"}
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">URL</label>
              <input
                type="text"
                bind:value={settings.ollama_url}
                placeholder="http://localhost:11434"
                class="w-full px-3 py-2 bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-sm text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none focus:border-phantom-ear-accent transition-colors"
              />
            </div>
            <div>
              <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">Model</label>
              <input
                type="text"
                bind:value={settings.ollama_model}
                placeholder="llama3.2"
                class="w-full px-3 py-2 bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-sm text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none focus:border-phantom-ear-accent transition-colors"
              />
            </div>
          </div>
          <p class="text-[11px] text-phantom-ear-text-muted">
            Pull model: <code class="text-phantom-ear-accent">ollama pull llama3.2</code>
          </p>
        {:else}
          <div>
            <label class="block text-xs font-medium text-phantom-ear-text-muted mb-1.5">API Key</label>
            <input
              type="password"
              bind:value={settings.openai_api_key}
              placeholder="sk-..."
              class="w-full px-3 py-2 bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-sm text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none focus:border-phantom-ear-accent transition-colors"
            />
            <p class="mt-1 text-[11px] text-phantom-ear-text-muted">Stored locally, never shared</p>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex justify-end gap-2 px-4 py-3 border-t border-phantom-ear-border/50 shrink-0">
      {#if !inline}
        <button
          onclick={onClose}
          class="px-3 py-2 rounded-lg text-sm text-phantom-ear-text-muted hover:text-phantom-ear-text hover:bg-phantom-ear-surface transition-colors"
        >
          Cancel
        </button>
      {/if}
      <button
        onclick={saveSettings}
        disabled={isSaving}
        class="px-4 py-2 bg-gradient-accent rounded-lg text-sm font-medium text-white transition-all disabled:opacity-50"
      >
        {#if isSaving}
          <span class="flex items-center gap-2">
            <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
            Saving...
          </span>
        {:else}
          Save
        {/if}
      </button>
    </div>
  {/if}
</div>
