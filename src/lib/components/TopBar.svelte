<script lang="ts">
  import type { ModelInfo } from '$lib/types';
  import AudioLevelIndicator from './AudioLevelIndicator.svelte';

  let {
    language = 'en',
    currentModel = 'base',
    models = [],
    llmProvider = 'ollama',
    llmModelName = '',
    onLanguageChange,
    onModelChange,
    onDownloadModel,
    onLlmChange,
    isRecording = false,
    recordingDuration = 0,
    isPaused = false,
    onToggleRecording,
    onTogglePause,
    showLiveIndicator = false,
    onReturnToLive,
  }: {
    language?: string;
    currentModel?: string;
    models?: ModelInfo[];
    llmProvider?: string;
    llmModelName?: string;
    onLanguageChange: (lang: string) => void;
    onModelChange: (model: string) => void;
    onDownloadModel: (model: string) => void;
    onLlmChange: (provider: string) => void;
    isRecording?: boolean;
    recordingDuration?: number;
    isPaused?: boolean;
    onToggleRecording: () => void;
    onTogglePause: () => void;
    showLiveIndicator?: boolean;
    onReturnToLive?: () => void;
  } = $props();

  function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }

  let showLanguageDropdown = $state(false);
  let showEngineDropdown = $state(false);
  let showLlmDropdown = $state(false);

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

  let llmLabel = $derived(() => {
    if (llmProvider === 'openai') return 'OpenAI';
    if (llmModelName) return llmModelName;
    return 'Ollama';
  });
</script>

<header class="flex items-center justify-between px-4 py-3 border-b border-phantom-ear-border/50 bg-phantom-ear-bg">

  <!-- Left: Recording Controls or Live Indicator -->
  <div class="flex items-center gap-3">
    {#if isRecording}
      {#if showLiveIndicator}
        <!-- Recording but NOT on live view - show "Return to Live" button -->
        <button
          onclick={onReturnToLive}
          class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-phantom-ear-danger/10 border border-phantom-ear-danger/30 hover:bg-phantom-ear-danger/15 hover:border-phantom-ear-danger/50 transition-colors"
          title="Return to live recording"
        >
          <span class="w-2 h-2 rounded-full bg-phantom-ear-danger animate-pulse"></span>
          <span class="text-xs font-medium text-phantom-ear-danger">Live</span>
          <span class="text-xs font-mono text-phantom-ear-danger/80">{formatDuration(recordingDuration)}</span>
          <svg class="w-3 h-3 text-phantom-ear-danger/60" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
        </button>
      {:else}
        <!-- On live view - show full recording controls -->
        <!-- Pulsing red dot + timer -->
        <div class="flex items-center gap-2">
          <div class="w-2.5 h-2.5 rounded-full bg-phantom-ear-danger {isPaused ? 'opacity-40' : 'animate-pulse-recording'}"></div>
          <span class="text-sm font-mono font-semibold {isPaused ? 'text-phantom-ear-text-muted' : 'text-phantom-ear-danger'}">
            {isPaused ? 'Paused' : formatDuration(recordingDuration)}
          </span>
        </div>

        <!-- Audio level indicator -->
        {#if !isPaused}
          <AudioLevelIndicator />
        {/if}

        <!-- Pause/Resume button -->
        <button
          onclick={onTogglePause}
          class="p-1.5 rounded-lg hover:bg-phantom-ear-surface transition-colors text-phantom-ear-text-muted hover:text-phantom-ear-text"
          title={isPaused ? 'Resume' : 'Pause'}
        >
          {#if isPaused}
            <!-- Play icon -->
            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
              <path d="M8 5v14l11-7z"/>
            </svg>
          {:else}
            <!-- Pause icon -->
            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
              <path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z"/>
            </svg>
          {/if}
        </button>

        <!-- Stop button -->
          <button
          onclick={onToggleRecording}
          class="p-1.5 rounded-lg bg-phantom-ear-danger/20 hover:bg-phantom-ear-danger/30 transition-colors text-phantom-ear-danger"
          title="Stop recording"
        >
          <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
            <rect x="6" y="6" width="12" height="12" rx="2" />
          </svg>
        </button>
      {/if}
    {/if}
  </div>

  <!-- Right: Dropdowns -->
  <div class="flex items-center gap-2">
    <!-- Language Selector -->
    <div class="relative">
      <button
        onclick={() => { showLanguageDropdown = !showLanguageDropdown; showEngineDropdown = false; showLlmDropdown = false; }}
        class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-phantom-ear-surface border border-phantom-ear-border hover:border-phantom-ear-text-muted transition-colors"
      >
        <svg class="w-4 h-4 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
        </svg>
        <span class="text-xs font-medium text-phantom-ear-text">{getLanguageName(language)}</span>
        <svg class="w-3 h-3 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {#if showLanguageDropdown}
        <div class="absolute right-0 top-full mt-1 py-1 bg-phantom-ear-surface border border-phantom-ear-border rounded-lg shadow-lg z-20 min-w-44 max-h-72 overflow-y-auto">
          {#each languages as lang}
            <button
              onclick={() => selectLanguage(lang.code)}
              class="w-full px-3 py-2.5 text-left text-sm hover:bg-phantom-ear-surface-hover flex items-center justify-between {language === lang.code ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}"
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
        onclick={() => { showEngineDropdown = !showEngineDropdown; showLanguageDropdown = false; showLlmDropdown = false; }}
        class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-phantom-ear-surface border border-phantom-ear-border hover:border-phantom-ear-text-muted transition-colors"
      >
        <svg class="w-4 h-4 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
        <span class="text-xs font-medium text-phantom-ear-text capitalize">{currentModel}</span>
        <svg class="w-3 h-3 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {#if showEngineDropdown}
        <div class="absolute right-0 top-full mt-1 py-1 bg-phantom-ear-surface border border-phantom-ear-border rounded-lg shadow-lg z-20 w-56 max-h-96 overflow-y-auto">
          <div class="px-3 py-2 text-xs font-semibold text-phantom-ear-text-muted uppercase tracking-wide border-b border-phantom-ear-border/50">
            Whisper Models
          </div>
          {#each models.filter(m => m.backend === 'whisper') as model}
            <button
              onclick={() => selectModel(model)}
              class="w-full px-3 py-2.5 text-left hover:bg-phantom-ear-surface-hover flex items-center justify-between gap-3"
            >
              <div class="flex items-center gap-2 shrink-0">
                <span class="text-sm font-medium capitalize {currentModel === model.name ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}">{model.name}</span>
                <span class="text-xs text-phantom-ear-text-muted whitespace-nowrap">{model.size_mb} MB</span>
              </div>
              <div class="flex items-center gap-1.5 shrink-0">
                {#if model.downloaded}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-phantom-ear-success/20 text-phantom-ear-success whitespace-nowrap">Ready</span>
                {:else}
                  <span class="text-xs px-1.5 py-0.5 rounded bg-phantom-ear-accent/20 text-phantom-ear-accent whitespace-nowrap">Download</span>
                {/if}
                {#if currentModel === model.name}
                  <svg class="w-4 h-4 text-phantom-ear-accent shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                  </svg>
                {/if}
              </div>
            </button>
          {/each}

          <div class="border-t border-phantom-ear-border/50 mt-1 pt-1">
            <div class="px-3 py-2 text-xs font-semibold text-phantom-ear-text-muted uppercase tracking-wide">
              Parakeet Models
            </div>
            <p class="px-3 pb-1 text-xs text-phantom-ear-text-muted">English only, fast ONNX inference</p>
            {#each models.filter(m => m.backend === 'parakeet') as model}
              <button
                onclick={() => selectModel(model)}
                class="w-full px-3 py-2.5 text-left hover:bg-phantom-ear-surface-hover flex items-center justify-between gap-3"
              >
                <div class="flex items-center gap-2 shrink-0">
                  <span class="text-sm font-medium {currentModel === model.name ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}">{model.name}</span>
                  <span class="text-xs text-phantom-ear-text-muted whitespace-nowrap">{model.size_mb} MB</span>
                </div>
                <div class="flex items-center gap-1.5 shrink-0">
                  {#if model.downloaded}
                    <span class="text-xs px-1.5 py-0.5 rounded bg-phantom-ear-success/20 text-phantom-ear-success whitespace-nowrap">Ready</span>
                  {:else}
                    <span class="text-xs px-1.5 py-0.5 rounded bg-phantom-ear-accent/20 text-phantom-ear-accent whitespace-nowrap">Download</span>
                  {/if}
                  {#if currentModel === model.name}
                    <svg class="w-4 h-4 text-phantom-ear-accent shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                    </svg>
                  {/if}
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <!-- LLM Provider Selector -->
    <div class="relative">
      <button
        onclick={() => { showLlmDropdown = !showLlmDropdown; showLanguageDropdown = false; showEngineDropdown = false; }}
        class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-phantom-ear-surface border border-phantom-ear-border hover:border-phantom-ear-text-muted transition-colors"
        title="LLM: {llmLabel()}"
      >
        <svg class="w-4 h-4 text-phantom-ear-purple" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
        </svg>
        <span class="text-xs font-medium text-phantom-ear-text">{llmLabel()}</span>
        <svg class="w-3 h-3 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {#if showLlmDropdown}
        <div class="absolute right-0 top-full mt-1 py-1 bg-phantom-ear-surface border border-phantom-ear-border rounded-lg shadow-lg z-20 min-w-40">
          <button
            onclick={() => { onLlmChange('ollama'); showLlmDropdown = false; }}
            class="w-full px-3 py-2.5 text-left text-sm hover:bg-phantom-ear-surface-hover flex items-center justify-between {llmProvider === 'ollama' ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}"
          >
            <span>Ollama</span>
            {#if llmProvider === 'ollama'}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
            {/if}
          </button>
          <button
            onclick={() => { onLlmChange('openai'); showLlmDropdown = false; }}
            class="w-full px-3 py-2.5 text-left text-sm hover:bg-phantom-ear-surface-hover flex items-center justify-between {llmProvider === 'openai' ? 'text-phantom-ear-accent' : 'text-phantom-ear-text'}"
          >
            <span>OpenAI</span>
            {#if llmProvider === 'openai'}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
            {/if}
          </button>
        </div>
      {/if}
    </div>
  </div>
</header>

<!-- Click outside to close dropdowns -->
{#if showLanguageDropdown || showEngineDropdown || showLlmDropdown}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-10"
    onclick={() => { showLanguageDropdown = false; showEngineDropdown = false; showLlmDropdown = false; }}
  ></div>
{/if}
