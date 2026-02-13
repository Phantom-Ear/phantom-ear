<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";

  interface MeetingDetectedEvent {
    app_name: string;
    message: string;
  }

  interface MeetingEndedEvent {
    message: string;
  }

  let { onStartRecording, isRecording = false }: {
    onStartRecording: () => void;
    isRecording?: boolean;
  } = $props();

  let showNotification = $state(false);
  let detectedApp = $state("");
  let isVisible = $state(false);
  let unlistenDetected: UnlistenFn | null = null;
  let unlistenEnded: UnlistenFn | null = null;

  onMount(async () => {
    // Request notification permission for native OS notifications
    try {
      let permissionGranted = await isPermissionGranted();
      if (!permissionGranted) {
        const permission = await requestPermission();
        permissionGranted = permission === "granted";
      }
      if (permissionGranted) {
        console.log("Native notification permission granted");
      }
    } catch (e) {
      console.warn("Failed to request notification permission:", e);
    }

    // Listen for meeting detected events (in-app notification)
    unlistenDetected = await listen<MeetingDetectedEvent>("meeting-detected", (event) => {
      if (!isRecording) {
        detectedApp = event.payload.app_name;
        showNotification = true;
        // Trigger animation
        requestAnimationFrame(() => {
          isVisible = true;
        });
      }
    });

    // Listen for meeting ended events
    unlistenEnded = await listen<MeetingEndedEvent>("meeting-ended", () => {
      hideNotification();
    });
  });

  onDestroy(() => {
    if (unlistenDetected) unlistenDetected();
    if (unlistenEnded) unlistenEnded();
  });

  // Hide notification when recording starts
  $effect(() => {
    if (isRecording && showNotification) {
      hideNotification();
    }
  });

  function hideNotification() {
    isVisible = false;
    setTimeout(() => {
      showNotification = false;
      detectedApp = "";
    }, 300);
  }

  async function handleStartRecording() {
    await dismiss();
    onStartRecording();
  }

  async function dismiss() {
    try {
      await invoke("dismiss_meeting_notification");
    } catch (e) {
      console.error("Failed to dismiss notification:", e);
    }
    hideNotification();
  }
</script>

{#if showNotification}
  <div
    class="fixed bottom-6 right-6 z-50 transform transition-all duration-300 ease-out {isVisible ? 'translate-y-0 opacity-100' : 'translate-y-4 opacity-0'}"
  >
    <div class="glass-strong rounded-2xl border border-phantom-ear-border shadow-glow-surface p-4 w-80">
      <!-- Header -->
      <div class="flex items-start gap-3">
        <!-- Animated icon -->
        <div class="relative shrink-0">
          <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-green-500/20 to-emerald-500/20 flex items-center justify-center">
            <svg class="w-5 h-5 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
          </div>
          <!-- Pulse indicator -->
          <span class="absolute -top-0.5 -right-0.5 flex h-3 w-3">
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
            <span class="relative inline-flex rounded-full h-3 w-3 bg-green-500"></span>
          </span>
        </div>

        <div class="flex-1 min-w-0">
          <h3 class="text-sm font-semibold text-phantom-ear-text">Meeting Detected</h3>
          <p class="text-xs text-phantom-ear-text-muted mt-0.5">
            <span class="font-medium text-green-400">{detectedApp}</span> is running
          </p>
        </div>

        <!-- Close button -->
        <button
          onclick={dismiss}
          class="p-1 rounded-lg text-phantom-ear-text-muted hover:text-phantom-ear-text hover:bg-phantom-ear-surface-hover transition-colors"
          title="Dismiss"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Message -->
      <p class="text-xs text-phantom-ear-text-muted mt-3 pl-13">
        Would you like to start recording this meeting?
      </p>

      <!-- Actions -->
      <div class="flex gap-2 mt-4">
        <button
          onclick={dismiss}
          class="flex-1 px-3 py-2 rounded-xl text-xs font-medium text-phantom-ear-text-muted hover:text-phantom-ear-text hover:bg-phantom-ear-surface-hover border border-phantom-ear-border transition-colors"
        >
          Not Now
        </button>
        <button
          onclick={handleStartRecording}
          class="flex-1 px-3 py-2 rounded-xl text-xs font-medium text-white bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-600 hover:to-emerald-700 transition-all shadow-lg shadow-green-500/20"
        >
          Start Recording
        </button>
      </div>
    </div>
  </div>
{/if}
