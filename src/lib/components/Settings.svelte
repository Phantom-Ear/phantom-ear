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
  }

  interface ModelInfo {
    name: string;
    size_mb: number;
    downloaded: boolean;
    description: string;
  }

  let { onClose }: { onClose: () => void } = $props();

  let settings = $state<Settings>({
    llm_provider: "ollama",
    openai_api_key: null,
    ollama_url: "http://localhost:11434",
    ollama_model: "llama3.2",
    auto_detect_meetings: false,
    whisper_model: "base",
    language: "en",
  });

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
      const [loadedSettings, loadedModels] = await Promise.all([
        invoke<Settings>("get_settings"),
        invoke<ModelInfo[]>("get_models_info"),
      ]);
      settings = loadedSettings;
      models = loadedModels;
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

<!-- Backdrop -->
<div
  class="fixed inset-0 bg-black/60 backdrop-blur-sm z-40"
  onclick={onClose}
  onkeydown={(e) => e.key === "Escape" && onClose()}
  role="button"
  tabindex="-1"
></div>

<!-- Modal -->
<div class="fixed inset-4 md:inset-auto md:top-1/2 md:left-1/2 md:-translate-x-1/2 md:-translate-y-1/2 md:w-[500px] md:max-h-[80vh] bg-sidecar-surface rounded-2xl border border-sidecar-border shadow-2xl z-50 flex flex-col overflow-hidden">
  <!-- Header -->
  <div class="flex items-center justify-between px-6 py-4 border-b border-sidecar-border">
    <h2 class="text-lg font-semibold text-sidecar-text">Settings</h2>
    <button
      onclick={onClose}
      class="p-2 rounded-lg hover:bg-sidecar-surface-hover transition-colors"
    >
      <svg class="w-5 h-5 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>

  {#if isLoading}
    <div class="flex-1 flex items-center justify-center py-12">
      <div class="w-8 h-8 border-2 border-sidecar-accent border-t-transparent rounded-full animate-spin"></div>
    </div>
  {:else}
    <!-- Tabs -->
    <div class="flex border-b border-sidecar-border">
      <button
        onclick={() => activeTab = "general"}
        class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'general' ? 'text-sidecar-accent border-b-2 border-sidecar-accent' : 'text-sidecar-text-muted hover:text-sidecar-text'}"
      >
        General
      </button>
      <button
        onclick={() => activeTab = "llm"}
        class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'llm' ? 'text-sidecar-accent border-b-2 border-sidecar-accent' : 'text-sidecar-text-muted hover:text-sidecar-text'}"
      >
        LLM
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

        <!-- Whisper Model -->
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
    <div class="flex justify-end gap-3 px-6 py-4 border-t border-sidecar-border">
      <button
        onclick={onClose}
        class="px-4 py-2 rounded-xl text-sm font-medium text-sidecar-text-muted hover:text-sidecar-text transition-colors"
      >
        Cancel
      </button>
      <button
        onclick={saveSettings}
        disabled={isSaving}
        class="px-4 py-2 bg-sidecar-accent hover:bg-sidecar-accent-hover rounded-xl text-sm font-medium text-white transition-colors disabled:opacity-50"
      >
        {isSaving ? "Saving..." : "Save Changes"}
      </button>
    </div>
  {/if}
</div>
