<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount, onDestroy } from "svelte";

  interface ModelInfo {
    name: string;
    size_mb: number;
    downloaded: boolean;
    recommended: boolean;
    description: string;
  }

  interface DownloadProgress {
    model_name: string;
    downloaded_bytes: number;
    total_bytes: number;
    percentage: number;
    status: "Starting" | "Downloading" | "Completed" | "Failed" | "Cancelled";
  }

  interface DeviceSpecs {
    cpu_cores: number;
    cpu_threads: number;
    cpu_name: string;
    ram_gb: number;
    available_ram_gb: number;
    has_gpu: boolean;
    gpu_name: string | null;
    is_apple_silicon: boolean;
    os: string;
  }

  interface ModelRecommendation {
    recommended_model: string;
    reason: string;
    estimated_speed: number;
    model_compatibility: { name: string; compatible: boolean; reason: string }[];
  }

  let { onComplete }: { onComplete: () => void } = $props();

  let models = $state<ModelInfo[]>([]);
  let selectedModel = $state("base");
  let isDownloading = $state(false);
  let downloadProgress = $state<DownloadProgress | null>(null);
  let error = $state("");
  let unlisten: (() => void) | null = null;
  let deviceSpecs = $state<DeviceSpecs | null>(null);
  let recommendation = $state<ModelRecommendation | null>(null);

  // Manual download flow
  let showManualDownload = $state(false);
  let isImporting = $state(false);
  let downloadFailed = $state(false);

  onMount(async () => {
    try {
      const [loadedModels, specs, rec] = await Promise.all([
        invoke<ModelInfo[]>("get_models_info"),
        invoke<DeviceSpecs>("get_device_specs"),
        invoke<ModelRecommendation>("get_model_recommendation"),
      ]);

      models = loadedModels;
      deviceSpecs = specs;
      recommendation = rec;

      if (rec.recommended_model) {
        selectedModel = rec.recommended_model;
      }

      unlisten = await listen<DownloadProgress>("model-download-progress", (event) => {
        downloadProgress = event.payload;
        if (event.payload.status === "Completed") {
          isDownloading = false;
          downloadFailed = false;
          setTimeout(() => {
            onComplete();
          }, 1000);
        } else if (event.payload.status === "Failed") {
          isDownloading = false;
          downloadFailed = true;
          error = "";
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
    downloadFailed = false;
    downloadProgress = null;
    showManualDownload = false;

    try {
      await invoke("download_model", { modelName: selectedModel });
    } catch (e) {
      const errMsg = String(e);
      if (errMsg.includes("too small") || errMsg.includes("firewall") || errMsg.includes("proxy")) {
        downloadFailed = true;
        error = "";
      } else {
        error = `Download error: ${e}`;
      }
      isDownloading = false;
    }
  }

  async function openManualDownload() {
    showManualDownload = true;
    downloadFailed = false;
    error = "";

    try {
      const url = await invoke<string>("get_model_download_url", { modelName: selectedModel });
      await openUrl(url);
    } catch (e) {
      error = `Failed to open download link: ${e}`;
    }
  }

  async function importModelFile() {
    error = "";
    isImporting = true;

    try {
      const selected = await open({
        title: "Select downloaded model file",
        filters: [{ name: "Model files", extensions: ["bin"] }],
        multiple: false,
      });

      if (!selected) {
        isImporting = false;
        return;
      }

      const filePath = typeof selected === "string" ? selected : selected;

      await invoke("import_model", {
        filePath,
        modelName: selectedModel,
      });

      isImporting = false;
      onComplete();
    } catch (e) {
      error = `Import failed: ${e}`;
      isImporting = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  function getModelSizeMb(): number {
    const m = models.find(m => m.name === selectedModel);
    return m?.size_mb ?? 0;
  }
</script>

<div class="flex flex-col items-center justify-center min-h-screen p-8 bg-sidecar-bg">
  <div class="w-full max-w-md">
    <!-- Logo and Title -->
    <div class="text-center mb-8">
      <div class="relative w-20 h-20 mx-auto mb-5">
        <div class="absolute inset-0 rounded-2xl bg-gradient-accent blur-xl opacity-50"></div>
        <div class="relative w-full h-full rounded-2xl bg-gradient-accent flex items-center justify-center shadow-glow-accent">
          <svg class="w-10 h-10 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
          </svg>
        </div>
      </div>
      <h1 class="text-2xl font-semibold mb-2">Welcome to PhantomEar</h1>
      <p class="text-sidecar-text-muted text-sm">
        Privacy-first meeting assistant with real-time transcription
      </p>
    </div>

    <!-- Device Specs Card (if detected) -->
    {#if deviceSpecs && !isDownloading && !showManualDownload}
      <div class="glass rounded-xl border border-sidecar-border p-4 mb-4 shadow-glow-surface">
        <div class="flex items-center gap-2 mb-3">
          <svg class="w-4 h-4 text-sidecar-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
          </svg>
          <span class="text-sm font-medium text-sidecar-text">Your System</span>
        </div>
        <div class="grid grid-cols-2 gap-3 text-xs">
          <div class="flex items-center gap-2">
            <span class="text-sidecar-text-muted">CPU:</span>
            <span class="text-sidecar-text truncate">{deviceSpecs.cpu_cores} cores</span>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-sidecar-text-muted">RAM:</span>
            <span class="text-sidecar-text">{deviceSpecs.ram_gb.toFixed(0)} GB</span>
          </div>
          {#if deviceSpecs.has_gpu}
            <div class="flex items-center gap-2 col-span-2">
              <span class="text-sidecar-text-muted">GPU:</span>
              <span class="text-sidecar-success">{deviceSpecs.gpu_name || "Available"}</span>
            </div>
          {/if}
        </div>
        {#if recommendation}
          <div class="mt-3 pt-3 border-t border-sidecar-border/50">
            <p class="text-xs text-sidecar-text-muted">
              <span class="text-sidecar-accent font-medium">{recommendation.recommended_model}</span> model recommended - {recommendation.reason}
            </p>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Setup Card -->
    <div class="glass rounded-2xl border border-sidecar-border p-6 shadow-glow-surface">

      {#if showManualDownload}
        <!-- Manual Download Flow -->
        <div class="text-center">
          <div class="w-14 h-14 mx-auto mb-4 rounded-xl bg-sidecar-accent/20 flex items-center justify-center">
            <svg class="w-7 h-7 text-sidecar-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
          </div>

          <h2 class="text-lg font-medium mb-2">Manual Download</h2>
          <p class="text-sm text-sidecar-text-muted mb-4">
            A browser window has opened to download the <span class="text-sidecar-text font-medium capitalize">{selectedModel}</span> model ({getModelSizeMb()} MB).
          </p>

          <div class="bg-sidecar-surface/50 border border-sidecar-border/50 rounded-xl p-4 mb-5">
            <div class="flex items-start gap-3">
              <div class="w-6 h-6 rounded-full bg-sidecar-accent/20 flex items-center justify-center shrink-0 mt-0.5">
                <span class="text-xs font-bold text-sidecar-accent">1</span>
              </div>
              <p class="text-sm text-sidecar-text-muted text-left">
                Wait for the <span class="text-sidecar-text">.bin</span> file to finish downloading in your browser (~1 min)
              </p>
            </div>
            <div class="flex items-start gap-3 mt-3">
              <div class="w-6 h-6 rounded-full bg-sidecar-accent/20 flex items-center justify-center shrink-0 mt-0.5">
                <span class="text-xs font-bold text-sidecar-accent">2</span>
              </div>
              <p class="text-sm text-sidecar-text-muted text-left">
                Click <span class="text-sidecar-text">Import Model</span> below and select the downloaded file
              </p>
            </div>
          </div>

          {#if error}
            <div class="mb-4 p-3 rounded-xl bg-sidecar-danger/10 border border-sidecar-danger/20 text-sidecar-danger text-sm">
              {error}
            </div>
          {/if}

          <button
            onclick={importModelFile}
            disabled={isImporting}
            class="w-full py-3.5 px-4 bg-gradient-accent hover:bg-gradient-accent-hover rounded-xl font-medium transition-all hover-lift btn-shine disabled:opacity-50"
          >
            {#if isImporting}
              Importing...
            {:else}
              Import Model
            {/if}
          </button>

          <div class="flex gap-3 mt-3">
            <button
              onclick={openManualDownload}
              class="flex-1 py-2.5 px-4 border border-sidecar-border rounded-xl text-sm text-sidecar-text-muted hover:text-sidecar-text hover:border-sidecar-text-muted transition-colors"
            >
              Re-open Link
            </button>
            <button
              onclick={() => { showManualDownload = false; downloadFailed = false; }}
              class="flex-1 py-2.5 px-4 border border-sidecar-border rounded-xl text-sm text-sidecar-text-muted hover:text-sidecar-text hover:border-sidecar-text-muted transition-colors"
            >
              Back
            </button>
          </div>
        </div>

      {:else if downloadFailed}
        <!-- Download Failed - Retry / Manual -->
        <div class="text-center">
          <div class="w-14 h-14 mx-auto mb-4 rounded-xl bg-sidecar-warning/20 flex items-center justify-center">
            <svg class="w-7 h-7 text-sidecar-warning" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>

          <h2 class="text-lg font-medium mb-2">Download Blocked</h2>
          <p class="text-sm text-sidecar-text-muted mb-6">
            The automatic download was blocked, likely by a corporate firewall or proxy. You can retry or download the model manually.
          </p>

          <div class="space-y-3">
            <button
              onclick={startDownload}
              class="w-full py-3.5 px-4 bg-gradient-accent hover:bg-gradient-accent-hover rounded-xl font-medium transition-all hover-lift btn-shine"
            >
              Retry Download
            </button>
            <button
              onclick={openManualDownload}
              class="w-full py-3 px-4 border border-sidecar-border rounded-xl font-medium text-sidecar-text hover:border-sidecar-accent hover:text-sidecar-accent transition-colors"
            >
              Download Manually
            </button>
          </div>

          <p class="text-xs text-sidecar-text-muted mt-4">
            Manual download takes about 1 minute on a fast connection
          </p>
        </div>

      {:else if !isDownloading}
        <!-- Normal Setup - Model Selection -->
        <h2 class="text-lg font-medium mb-4">Download Speech Model</h2>
        <p class="text-sm text-sidecar-text-muted mb-6">
          PhantomEar uses a local AI model for speech recognition. All transcription happens on your device - your audio never leaves your computer.
        </p>

        <!-- Model Selection -->
        <div class="space-y-3 mb-6">
          {#each models as model}
            {@const compat = recommendation?.model_compatibility.find(c => c.name === model.name)}
            {@const isRecommended = recommendation?.recommended_model === model.name}
            <label
              class="flex items-center gap-3 p-3 rounded-xl border cursor-pointer transition-colors
                {selectedModel === model.name
                  ? 'border-sidecar-accent bg-sidecar-accent/10'
                  : compat && !compat.compatible
                    ? 'border-sidecar-border/50 opacity-60'
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
                <div class="flex items-center gap-2 flex-wrap">
                  <span class="font-medium capitalize">{model.name}</span>
                  {#if isRecommended}
                    <span class="text-xs px-2 py-0.5 rounded-full bg-gradient-accent text-white">
                      Recommended
                    </span>
                  {/if}
                  {#if model.downloaded}
                    <span class="text-xs px-2 py-0.5 rounded-full bg-sidecar-success/20 text-sidecar-success">
                      Downloaded
                    </span>
                  {/if}
                  {#if compat && !compat.compatible}
                    <span class="text-xs px-2 py-0.5 rounded-full bg-sidecar-warning/20 text-sidecar-warning">
                      May be slow
                    </span>
                  {/if}
                </div>
                <div class="flex items-center gap-2 mt-0.5">
                  <span class="text-xs text-sidecar-text-muted">{model.size_mb} MB</span>
                  {#if model.description}
                    <span class="text-xs text-sidecar-text-muted">• {model.description}</span>
                  {/if}
                </div>
              </div>
              <div class="w-5 h-5 rounded-full border-2 flex items-center justify-center shrink-0
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
          class="w-full py-3.5 px-4 bg-gradient-accent hover:bg-gradient-accent-hover rounded-xl font-medium transition-all hover-lift btn-shine"
        >
          Download & Continue
        </button>

        <p class="text-xs text-sidecar-text-muted text-center mt-4">
          You can change the model later in Settings
        </p>

      {:else}
        <!-- Download Progress -->
        <div class="text-center">
          <div class="relative w-14 h-14 mx-auto mb-4">
            {#if downloadProgress?.status === "Completed"}
              <div class="w-full h-full rounded-xl bg-sidecar-success/20 flex items-center justify-center">
                <svg class="w-7 h-7 text-sidecar-success" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
              </div>
            {:else}
              <div class="absolute inset-0 rounded-xl bg-gradient-accent opacity-20 animate-pulse"></div>
              <div class="relative w-full h-full rounded-xl bg-sidecar-surface flex items-center justify-center">
                <svg class="w-6 h-6 text-sidecar-accent animate-spin" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                </svg>
              </div>
            {/if}
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
            <div class="w-full h-2.5 bg-sidecar-border rounded-full overflow-hidden mb-2">
              <div
                class="h-full bg-gradient-accent transition-all duration-300 ease-out rounded-full"
                style="width: {downloadProgress.percentage}%"
              ></div>
            </div>

            <p class="text-sm text-sidecar-text-muted font-mono">
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
    <div class="flex items-center justify-center gap-2 mt-6 px-4 py-2 rounded-full bg-sidecar-surface/50 border border-sidecar-border/50 mx-auto w-fit">
      <svg class="w-4 h-4 text-sidecar-success" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
      </svg>
      <p class="text-xs text-sidecar-text-muted">
        Your audio and transcripts stay on your device
      </p>
    </div>
  </div>
</div>
