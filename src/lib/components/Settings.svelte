<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Settings {
    llm_provider: string;
    openai_api_key: string | null;
    ollama_url: string | null;
    ollama_model: string | null;
    auto_detect_meetings: boolean;
    whisper_model: string;
    language: string;
    asr_backend: string;
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

  let { onClose, inline = false }: { onClose: () => void; inline?: boolean } = $props();

  let settings = $state<Settings>({
    llm_provider: "ollama",
    openai_api_key: null,
    ollama_url: "http://localhost:11434",
    ollama_model: "llama3.2",
    auto_detect_meetings: false,
    whisper_model: "base",
    language: "en",
    asr_backend: "whisper",
  });

  let asrBackends = $state<BackendInfo[]>([]);

  let models = $state<ModelInfo[]>([]);
  let isLoading = $state(true);
  let isSaving = $state(false);
  let activeTab = $state<"general" | "llm">("general");

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
      const [loadedSettings, loadedModels, loadedBackends] = await Promise.all([
        invoke<Settings>("get_settings"),
        invoke<ModelInfo[]>("get_models_info"),
        invoke<BackendInfo[]>("get_asr_backends"),
      ]);
      settings = loadedSettings;
      models = loadedModels;
      asrBackends = loadedBackends;
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
    isLoading = false;
  }

  async function saveSettings() {
    isSaving = true;
    try {
      await invoke("save_settings", { settings });

      // Reload model with new language if changed
      await invoke("load_model", { modelName: settings.whisper_model });

      onClose();
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
    isSaving = false;
  }

  // Load settings on mount
  $effect(() => {
    loadSettings();
  });
</script>

{#if !inline}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/70 backdrop-blur-md z-40"
    onclick={onClose}
    onkeydown={(e) => e.key === "Escape" && onClose()}
    role="button"
    tabindex="-1"
  ></div>
{/if}

<!-- Modal / Inline Container -->
<div class="{inline ? 'flex flex-col h-full' : 'fixed inset-4 md:inset-auto md:top-1/2 md:left-1/2 md:-translate-x-1/2 md:-translate-y-1/2 md:w-[500px] md:max-h-[80vh] glass-strong rounded-2xl border border-sidecar-border shadow-glow-surface z-50 flex flex-col overflow-hidden'}">
  <!-- Header -->
  <div class="flex items-center justify-between px-6 py-4 border-b border-sidecar-border/50">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-gradient-accent flex items-center justify-center">
        <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </div>
      <h2 class="text-lg font-semibold text-sidecar-text">Settings</h2>
    </div>
    {#if !inline}
      <button
        onclick={onClose}
        class="p-2 rounded-lg hover:bg-sidecar-surface-hover transition-colors"
        title="Close settings"
      >
        <svg class="w-5 h-5 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    {/if}
  </div>

  {#if isLoading}
    <div class="flex-1 flex items-center justify-center py-12">
      <div class="w-8 h-8 border-2 border-sidecar-accent border-t-transparent rounded-full animate-spin"></div>
    </div>
  {:else}
    <!-- Tabs -->
    <div class="flex border-b border-sidecar-border/50 px-2">
      <button
        onclick={() => activeTab = "general"}
        class="relative flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'general' ? 'text-sidecar-accent' : 'text-sidecar-text-muted hover:text-sidecar-text'}"
      >
        <span class="relative z-10 flex items-center justify-center gap-2">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
          </svg>
          General
        </span>
        {#if activeTab === 'general'}
          <div class="absolute bottom-0 left-2 right-2 h-0.5 bg-gradient-accent rounded-full"></div>
        {/if}
      </button>
      <button
        onclick={() => activeTab = "llm"}
        class="relative flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'llm' ? 'text-sidecar-accent' : 'text-sidecar-text-muted hover:text-sidecar-text'}"
      >
        <span class="relative z-10 flex items-center justify-center gap-2">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
          </svg>
          LLM
        </span>
        {#if activeTab === 'llm'}
          <div class="absolute bottom-0 left-2 right-2 h-0.5 bg-gradient-accent rounded-full"></div>
        {/if}
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 space-y-6">
      {#if activeTab === "general"}
        <!-- Language -->
        <div>
          <label class="block text-sm font-medium text-sidecar-text mb-2">
            Transcription Language
          </label>
          <select
            bind:value={settings.language}
            class="w-full px-4 py-3 bg-sidecar-bg border border-sidecar-border rounded-xl text-sm text-sidecar-text focus:outline-none focus:border-sidecar-accent transition-colors"
          >
            {#each languages as lang}
              <option value={lang.code}>{lang.name}</option>
            {/each}
          </select>
          <p class="mt-1 text-xs text-sidecar-text-muted">
            Choose "Auto-detect" if speakers use multiple languages
          </p>
        </div>

        <!-- ASR Backend -->
        <div>
          <label class="block text-sm font-medium text-sidecar-text mb-2">
            Speech Recognition Engine
          </label>
          <div class="space-y-2">
            {#each asrBackends as backend}
              {@const isParakeet = backend.backend_type === "parakeet"}
              <label
                class="flex items-center gap-3 p-3 rounded-xl border cursor-pointer transition-colors {settings.asr_backend === backend.backend_type ? 'border-sidecar-accent bg-sidecar-accent/10' : 'border-sidecar-border hover:border-sidecar-text-muted'} {isParakeet ? 'opacity-60' : ''}"
              >
                <input
                  type="radio"
                  name="asr_backend"
                  value={backend.backend_type}
                  bind:group={settings.asr_backend}
                  disabled={isParakeet}
                  class="sr-only"
                />
                <div class="flex-1">
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-medium text-sidecar-text">{backend.name}</span>
                    {#if isParakeet}
                      <span class="text-xs px-2 py-0.5 rounded-full bg-sidecar-warning/20 text-sidecar-warning">Coming Soon</span>
                    {/if}
                  </div>
                  <p class="text-xs text-sidecar-text-muted mt-0.5">{backend.description}</p>
                </div>
                <div class="w-4 h-4 rounded-full border-2 flex items-center justify-center {settings.asr_backend === backend.backend_type ? 'border-sidecar-accent' : 'border-sidecar-border'}">
                  {#if settings.asr_backend === backend.backend_type}
                    <div class="w-2 h-2 rounded-full bg-sidecar-accent"></div>
                  {/if}
                </div>
              </label>
            {/each}
          </div>
        </div>

        <!-- Whisper Model (only show when Whisper is selected) -->
        {#if settings.asr_backend === "whisper"}
          <div>
            <label class="block text-sm font-medium text-sidecar-text mb-2">
              Whisper Model
            </label>
            <div class="space-y-2">
              {#each models as model}
                <label
                  class="flex items-center gap-3 p-3 rounded-xl border cursor-pointer transition-colors {settings.whisper_model === model.name ? 'border-sidecar-accent bg-sidecar-accent/10' : 'border-sidecar-border hover:border-sidecar-text-muted'}"
                >
                  <input
                    type="radio"
                    name="whisper_model"
                    value={model.name}
                    bind:group={settings.whisper_model}
                    class="sr-only"
                  />
                  <div class="flex-1">
                    <div class="flex items-center gap-2">
                      <span class="text-sm font-medium text-sidecar-text capitalize">{model.name}</span>
                      <span class="text-xs text-sidecar-text-muted">{model.size_mb} MB</span>
                      {#if model.downloaded}
                        <span class="text-xs text-sidecar-success">Downloaded</span>
                      {/if}
                    </div>
                    <p class="text-xs text-sidecar-text-muted mt-0.5">{model.description}</p>
                  </div>
                  <div class="w-4 h-4 rounded-full border-2 flex items-center justify-center {settings.whisper_model === model.name ? 'border-sidecar-accent' : 'border-sidecar-border'}">
                    {#if settings.whisper_model === model.name}
                      <div class="w-2 h-2 rounded-full bg-sidecar-accent"></div>
                    {/if}
                  </div>
                </label>
              {/each}
            </div>
          </div>
        {/if}

      {:else if activeTab === "llm"}
        <!-- LLM Provider -->
        <div>
          <label class="block text-sm font-medium text-sidecar-text mb-2">
            LLM Provider
          </label>
          <div class="flex gap-2">
            <button
              onclick={() => settings.llm_provider = "ollama"}
              class="flex-1 px-4 py-3 rounded-xl text-sm font-medium transition-colors {settings.llm_provider === 'ollama' ? 'bg-sidecar-accent text-white' : 'bg-sidecar-bg border border-sidecar-border text-sidecar-text hover:border-sidecar-text-muted'}"
            >
              Ollama (Local)
            </button>
            <button
              onclick={() => settings.llm_provider = "openai"}
              class="flex-1 px-4 py-3 rounded-xl text-sm font-medium transition-colors {settings.llm_provider === 'openai' ? 'bg-sidecar-accent text-white' : 'bg-sidecar-bg border border-sidecar-border text-sidecar-text hover:border-sidecar-text-muted'}"
            >
              OpenAI
            </button>
          </div>
        </div>

        {#if settings.llm_provider === "ollama"}
          <!-- Ollama URL -->
          <div>
            <label class="block text-sm font-medium text-sidecar-text mb-2">
              Ollama URL
            </label>
            <input
              type="text"
              bind:value={settings.ollama_url}
              placeholder="http://localhost:11434"
              class="w-full px-4 py-3 bg-sidecar-bg border border-sidecar-border rounded-xl text-sm text-sidecar-text placeholder:text-sidecar-text-muted focus:outline-none focus:border-sidecar-accent transition-colors"
            />
          </div>

          <!-- Ollama Model -->
          <div>
            <label class="block text-sm font-medium text-sidecar-text mb-2">
              Model Name
            </label>
            <input
              type="text"
              bind:value={settings.ollama_model}
              placeholder="llama3.2"
              class="w-full px-4 py-3 bg-sidecar-bg border border-sidecar-border rounded-xl text-sm text-sidecar-text placeholder:text-sidecar-text-muted focus:outline-none focus:border-sidecar-accent transition-colors"
            />
            <p class="mt-1 text-xs text-sidecar-text-muted">
              Make sure the model is pulled: <code class="text-sidecar-accent">ollama pull llama3.2</code>
            </p>
          </div>
        {:else}
          <!-- OpenAI API Key -->
          <div>
            <label class="block text-sm font-medium text-sidecar-text mb-2">
              OpenAI API Key
            </label>
            <input
              type="password"
              bind:value={settings.openai_api_key}
              placeholder="sk-..."
              class="w-full px-4 py-3 bg-sidecar-bg border border-sidecar-border rounded-xl text-sm text-sidecar-text placeholder:text-sidecar-text-muted focus:outline-none focus:border-sidecar-accent transition-colors"
            />
            <p class="mt-1 text-xs text-sidecar-text-muted">
              Your API key is stored locally and never shared
            </p>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex justify-end gap-3 px-6 py-4 border-t border-sidecar-border/50">
      {#if !inline}
        <button
          onclick={onClose}
          class="px-4 py-2.5 rounded-xl text-sm font-medium text-sidecar-text-muted hover:text-sidecar-text hover:bg-sidecar-surface transition-colors"
        >
          Cancel
        </button>
      {/if}
      <button
        onclick={saveSettings}
        disabled={isSaving}
        class="px-5 py-2.5 bg-gradient-accent hover:bg-gradient-accent-hover rounded-xl text-sm font-medium text-white transition-all hover-lift disabled:opacity-50 disabled:hover:transform-none btn-shine"
      >
        {#if isSaving}
          <span class="flex items-center gap-2">
            <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
            Saving...
          </span>
        {:else}
          Save Changes
        {/if}
      </button>
    </div>
  {/if}
</div>
