<script lang="ts">
  import type { ModelInfo } from '$lib/types';

  let {
    language = 'en',
    currentModel = 'base',
    models = [],
    onLanguageChange,
    onModelChange,
    onDownloadModel,
  }: {
    language?: string;
    currentModel?: string;
    models?: ModelInfo[];
    onLanguageChange: (lang: string) => void;
    onModelChange: (model: string) => void;
    onDownloadModel: (model: string) => void;
  } = $props();

  let showLanguageDropdown = $state(false);
  let showEngineDropdown = $state(false);

  const languages = [
    { code: 'auto', name: 'Auto-detect' },
    { code: 'en', name: 'English' },
    { code: 'fr', name: 'French' },
    { code: 'es', name: 'Spanish' },
    { code: 'de', name: 'German' },
    { code: 'it', name: 'Italian' },
    { code: 'pt', name: 'Portuguese' },
    { code: 'nl', name: 'Dutch' },
    { code: 'pl', name: 'Polish' },
    { code: 'ru', name: 'Russian' },
    { code: 'ja', name: 'Japanese' },
    { code: 'ko', name: 'Korean' },
    { code: 'zh', name: 'Chinese' },
    { code: 'ar', name: 'Arabic' },
  ];

  function getLanguageName(code: string): string {
    return languages.find(l => l.code === code)?.name || code;
  }

  function selectLanguage(code: string) {
    onLanguageChange(code);
    showLanguageDropdown = false;
  }

  function selectModel(model: ModelInfo) {
    if (model.downloaded) {
      onModelChange(model.name);
      showEngineDropdown = false;
    } else {
      // Trigger download for non-downloaded models
      onDownloadModel(model.name);
      showEngineDropdown = false;
    }
  }
</script>

<header class="flex items-center justify-end px-4 py-3 border-b border-sidecar-border/50 bg-sidecar-bg">

  <!-- Right: Dropdowns -->
  <div class="flex items-center gap-2">
    <!-- Language Selector -->
    <div class="relative">
      <button
        onclick={() => { showLanguageDropdown = !showLanguageDropdown; showEngineDropdown = false; }}
        class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-sidecar-surface border border-sidecar-border hover:border-sidecar-text-muted transition-colors"
      >
        <svg class="w-4 h-4 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
        </svg>
        <span class="text-xs font-medium text-sidecar-text">{getLanguageName(language)}</span>
        <svg class="w-3 h-3 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {#if showLanguageDropdown}
        <div class="absolute right-0 top-full mt-1 py-1 bg-sidecar-surface border border-sidecar-border rounded-lg shadow-lg z-20 min-w-40 max-h-64 overflow-y-auto">
          {#each languages as lang}
            <button
              onclick={() => selectLanguage(lang.code)}
              class="w-full px-3 py-2 text-left text-sm hover:bg-sidecar-surface-hover flex items-center justify-between {language === lang.code ? 'text-sidecar-accent' : 'text-sidecar-text'}"
            >
              <span>{lang.name}</span>
              {#if language === lang.code}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Engine/Model Selector -->
    <div class="relative">
      <button
        onclick={() => { showEngineDropdown = !showEngineDropdown; showLanguageDropdown = false; }}
        class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-sidecar-surface border border-sidecar-border hover:border-sidecar-text-muted transition-colors"
      >
        <svg class="w-4 h-4 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
        <span class="text-xs font-medium text-sidecar-text capitalize">{currentModel}</span>
        <svg class="w-3 h-3 text-sidecar-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {#if showEngineDropdown}
        <div class="absolute right-0 top-full mt-1 py-1 bg-sidecar-surface border border-sidecar-border rounded-lg shadow-lg z-20 min-w-48">
          <div class="px-3 py-2 text-xs font-semibold text-sidecar-text-muted uppercase tracking-wide border-b border-sidecar-border/50">
            Whisper Models
          </div>
          {#each models as model}
            <button
              onclick={() => selectModel(model)}
              class="w-full px-3 py-2 text-left hover:bg-sidecar-surface-hover flex items-center justify-between"
            >
              <div>
                <span class="text-sm capitalize {currentModel === model.name ? 'text-sidecar-accent' : 'text-sidecar-text'}">{model.name}</span>
                <span class="text-xs text-sidecar-text-muted ml-2">{model.size_mb} MB</span>
              </div>
              <div class="flex items-center gap-2">
                {#if model.downloaded}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-sidecar-success/20 text-sidecar-success">Ready</span>
                {:else}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-sidecar-accent/20 text-sidecar-accent">Download</span>
                {/if}
                {#if currentModel === model.name}
                  <svg class="w-4 h-4 text-sidecar-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                  </svg>
                {/if}
              </div>
            </button>
          {/each}

          <div class="border-t border-sidecar-border/50 mt-1 pt-1">
            <div class="px-3 py-2 text-xs font-semibold text-sidecar-text-muted uppercase tracking-wide">
              Other Engines
            </div>
            <div class="px-3 py-2 flex items-center justify-between opacity-60">
              <span class="text-sm text-sidecar-text">Parakeet</span>
              <span class="text-xs px-1.5 py-0.5 rounded bg-sidecar-warning/20 text-sidecar-warning">Coming Soon</span>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
</header>

<!-- Click outside to close dropdowns -->
{#if showLanguageDropdown || showEngineDropdown}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-10"
    onclick={() => { showLanguageDropdown = false; showEngineDropdown = false; }}
  ></div>
{/if}
