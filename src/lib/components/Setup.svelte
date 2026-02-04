<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  interface ModelInfo {
    name: string;
    size_mb: number;
    downloaded: boolean;
    recommended: boolean;
  }

  interface DownloadProgress {
    model_name: string;
    downloaded_bytes: number;
    total_bytes: number;
    percentage: number;
    status: "Starting" | "Downloading" | "Completed" | "Failed" | "Cancelled";
  }

  let { onComplete }: { onComplete: () => void } = $props();

  let models = $state<ModelInfo[]>([]);
  let selectedModel = $state("base");
  let isDownloading = $state(false);
  let downloadProgress = $state<DownloadProgress | null>(null);
  let error = $state("");
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    try {
      models = await invoke<ModelInfo[]>("get_models_info");

      // Listen for download progress events
      unlisten = await listen<DownloadProgress>("model-download-progress", (event) => {
        downloadProgress = event.payload;
        if (event.payload.status === "Completed") {
          isDownloading = false;
          // Small delay to show completion, then proceed
          setTimeout(() => {
            onComplete();
          }, 1000);
        } else if (event.payload.status === "Failed") {
          isDownloading = false;
          error = "Download failed. Please try again.";
        }
      });
    } catch (e) {
      error = `Failed to load models: ${e}`;
    }
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  async function startDownload() {
    error = "";
    isDownloading = true;
    downloadProgress = null;

    try {
      await invoke("download_model", { modelName: selectedModel });
    } catch (e) {
      error = `Download error: ${e}`;
      isDownloading = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
</script>

<div class="flex flex-col items-center justify-center min-h-screen p-8 bg-sidecar-bg">
  <div class="w-full max-w-md">
    <!-- Logo and Title -->
    <div class="text-center mb-8">
      <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-sidecar-accent flex items-center justify-center">
        <svg class="w-9 h-9 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
        </svg>
      </div>
      <h1 class="text-2xl font-semibold mb-2">Welcome to Sidecar</h1>
      <p class="text-sidecar-text-muted text-sm">
        Privacy-first meeting assistant with real-time transcription
      </p>
    </div>

    <!-- Setup Card -->
    <div class="bg-sidecar-surface rounded-2xl border border-sidecar-border p-6">
      {#if !isDownloading}
        <h2 class="text-lg font-medium mb-4">Download Speech Model</h2>
        <p class="text-sm text-sidecar-text-muted mb-6">
          Sidecar uses a local AI model for speech recognition. All transcription happens on your device - your audio never leaves your computer.
        </p>

        <!-- Model Selection -->
        <div class="space-y-3 mb-6">
          {#each models as model}
            <label
              class="flex items-center gap-3 p-3 rounded-xl border cursor-pointer transition-colors
                {selectedModel === model.name
                  ? 'border-sidecar-accent bg-sidecar-accent/10'
                  : 'border-sidecar-border hover:border-sidecar-text-muted'}"
            >
              <input
                type="radio"
                name="model"
                value={model.name}
                bind:group={selectedModel}
                class="sr-only"
              />
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <span class="font-medium capitalize">{model.name}</span>
                  {#if model.recommended}
                    <span class="text-xs px-2 py-0.5 rounded-full bg-sidecar-accent/20 text-sidecar-accent">
                      Recommended
                    </span>
                  {/if}
                  {#if model.downloaded}
                    <span class="text-xs px-2 py-0.5 rounded-full bg-sidecar-success/20 text-sidecar-success">
                      Downloaded
                    </span>
                  {/if}
                </div>
                <span class="text-xs text-sidecar-text-muted">{model.size_mb} MB</span>
              </div>
              <div class="w-5 h-5 rounded-full border-2 flex items-center justify-center
                {selectedModel === model.name ? 'border-sidecar-accent' : 'border-sidecar-border'}">
                {#if selectedModel === model.name}
                  <div class="w-2.5 h-2.5 rounded-full bg-sidecar-accent"></div>
                {/if}
              </div>
            </label>
          {/each}
        </div>

        {#if error}
          <div class="mb-4 p-3 rounded-xl bg-sidecar-danger/10 border border-sidecar-danger/20 text-sidecar-danger text-sm">
            {error}
          </div>
        {/if}

        <button
          onclick={startDownload}
          class="w-full py-3 px-4 bg-sidecar-accent hover:bg-sidecar-accent-hover rounded-xl font-medium transition-colors"
        >
          Download & Continue
        </button>

        <p class="text-xs text-sidecar-text-muted text-center mt-4">
          You can change the model later in Settings
        </p>

      {:else}
        <!-- Download Progress -->
        <div class="text-center">
          <div class="w-12 h-12 mx-auto mb-4 rounded-xl bg-sidecar-accent/20 flex items-center justify-center">
            <svg class="w-6 h-6 text-sidecar-accent animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
          </div>

          <h2 class="text-lg font-medium mb-2">
            {#if downloadProgress?.status === "Completed"}
              Download Complete!
            {:else}
              Downloading Model
            {/if}
          </h2>

          {#if downloadProgress}
            <p class="text-sm text-sidecar-text-muted mb-4">
              {formatBytes(downloadProgress.downloaded_bytes)} / {formatBytes(downloadProgress.total_bytes)}
            </p>

            <!-- Progress Bar -->
            <div class="w-full h-2 bg-sidecar-border rounded-full overflow-hidden mb-2">
              <div
                class="h-full bg-sidecar-accent transition-all duration-300 ease-out"
                style="width: {downloadProgress.percentage}%"
              ></div>
            </div>

            <p class="text-sm text-sidecar-text-muted">
              {downloadProgress.percentage.toFixed(1)}%
            </p>
          {:else}
            <p class="text-sm text-sidecar-text-muted">
              Preparing download...
            </p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Privacy Note -->
    <p class="text-xs text-sidecar-text-muted text-center mt-6">
      <svg class="w-4 h-4 inline-block mr-1 -mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
      </svg>
      Your audio and transcripts stay on your device
    </p>
  </div>
</div>
