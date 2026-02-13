<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let { onComplete }: { onComplete: () => void } = $props();

  let currentStep = $state(0);
  let isAnimating = $state(false);

  const steps = [
    {
      id: "welcome",
      title: "Welcome to PhantomEar",
      subtitle: "Your privacy-first meeting assistant",
      description: "PhantomEar captures and transcribes your meetings locally, never sending raw audio to the cloud.",
      icon: "ear",
      features: [
        { icon: "lock", text: "100% local transcription" },
        { icon: "eye-off", text: "No meeting bots or banners" },
        { icon: "zap", text: "Real-time AI summaries" },
      ],
    },
    {
      id: "recording",
      title: "One-Click Recording",
      subtitle: "Start capturing in seconds",
      description: "Press the record button or use the keyboard shortcut to start capturing your meetings instantly.",
      icon: "mic",
      tips: [
        { shortcut: "⌘ + Shift + R", label: "Toggle recording" },
        { shortcut: "⌘ + B", label: "Toggle sidebar" },
        { shortcut: "⌘ + K", label: "Quick search" },
      ],
    },
    {
      id: "detection",
      title: "Smart Meeting Detection",
      subtitle: "We'll remind you to record",
      description: "PhantomEar can detect when you join Zoom, Teams, Meet, or other apps and prompt you to start recording.",
      icon: "radar",
      note: "Enable this in Settings > Meeting Detection",
    },
    {
      id: "phomy",
      title: "Meet Phomy",
      subtitle: "Your AI meeting memory",
      description: "Ask Phomy anything about your past meetings. It searches across all your transcripts to find relevant context.",
      icon: "brain",
      examples: [
        "What did we discuss about the Q4 budget?",
        "When was the last time John mentioned the API?",
        "Summarize yesterday's standup",
      ],
    },
    {
      id: "ready",
      title: "You're All Set!",
      subtitle: "Start recording your first meeting",
      description: "You can always revisit this guide from Settings. Happy recording!",
      icon: "rocket",
    },
  ];

  function nextStep() {
    if (currentStep < steps.length - 1) {
      isAnimating = true;
      setTimeout(() => {
        currentStep++;
        isAnimating = false;
      }, 200);
    } else {
      completeOnboarding();
    }
  }

  function prevStep() {
    if (currentStep > 0) {
      isAnimating = true;
      setTimeout(() => {
        currentStep--;
        isAnimating = false;
      }, 200);
    }
  }

  function skipOnboarding() {
    completeOnboarding();
  }

  async function completeOnboarding() {
    try {
      const settings = await invoke("get_settings") as any;
      settings.onboarding_completed = true;
      await invoke("save_settings", { settings });
    } catch (e) {
      console.error("Failed to save onboarding status:", e);
    }
    onComplete();
  }

  const step = $derived(steps[currentStep]);
  const progress = $derived(((currentStep + 1) / steps.length) * 100);
</script>

<div class="fixed inset-0 z-50 bg-phantom-ear-bg overflow-hidden">
  <!-- Background Gradient Orbs -->
  <div class="absolute inset-0 overflow-hidden pointer-events-none">
    <div class="absolute w-[600px] h-[600px] rounded-full bg-gradient-to-br from-phantom-ear-accent/20 to-phantom-ear-purple/10 blur-[100px] animate-float" style="top: -20%; left: -10%;"></div>
    <div class="absolute w-[500px] h-[500px] rounded-full bg-gradient-to-br from-phantom-ear-purple/15 to-phantom-ear-accent/5 blur-[80px] animate-float-slow" style="bottom: -15%; right: -10%;"></div>
    <div class="absolute w-[300px] h-[300px] rounded-full bg-gradient-to-br from-green-500/10 to-phantom-ear-accent/5 blur-[60px] animate-float-fast" style="top: 40%; right: 20%;"></div>
  </div>

  <!-- Content Container -->
  <div class="relative h-full flex flex-col items-center justify-center p-8 max-w-2xl mx-auto">
    <!-- Progress Bar -->
    <div class="absolute top-6 left-8 right-8">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs text-phantom-ear-text-muted">Step {currentStep + 1} of {steps.length}</span>
        <button
          onclick={skipOnboarding}
          class="text-xs text-phantom-ear-text-muted hover:text-phantom-ear-text transition-colors"
        >
          Skip tour
        </button>
      </div>
      <div class="h-1 bg-phantom-ear-surface rounded-full overflow-hidden">
        <div
          class="h-full bg-gradient-to-r from-phantom-ear-accent to-phantom-ear-purple transition-all duration-500 ease-out rounded-full"
          style="width: {progress}%"
        ></div>
      </div>
    </div>

    <!-- Step Content -->
    <div class="flex-1 flex flex-col items-center justify-center text-center {isAnimating ? 'opacity-0 translate-y-4' : 'opacity-100 translate-y-0'} transition-all duration-200">
      <!-- Icon -->
      <div class="mb-8 relative">
        <div class="w-24 h-24 rounded-3xl bg-gradient-to-br from-phantom-ear-accent/20 to-phantom-ear-purple/10 flex items-center justify-center border border-phantom-ear-border/50 shadow-glow-surface">
          {#if step.icon === "ear"}
            <img src="/PhantomEarNoBackground.png" alt="PhantomEar" class="w-14 h-14 object-contain" />
          {:else if step.icon === "mic"}
            <svg class="w-12 h-12 text-phantom-ear-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z" />
              <path stroke-linecap="round" stroke-linejoin="round" d="M19 10v2a7 7 0 01-14 0v-2M12 19v4M8 23h8" />
            </svg>
          {:else if step.icon === "radar"}
            <svg class="w-12 h-12 text-phantom-ear-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="12" cy="12" r="10" />
              <path stroke-linecap="round" d="M12 12l4-4" />
              <circle cx="12" cy="12" r="3" />
              <circle cx="12" cy="12" r="6" stroke-dasharray="4 2" />
            </svg>
          {:else if step.icon === "brain"}
            <svg class="w-12 h-12 text-phantom-ear-purple" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2C7.58 2 4 5.58 4 10v9c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1V10c0-4.42-3.58-8-8-8zm-2 10a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm4 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
            </svg>
          {:else if step.icon === "rocket"}
            <svg class="w-12 h-12 text-green-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M15.59 14.37a6 6 0 01-5.84 7.38v-4.8m5.84-2.58a14.98 14.98 0 006.16-12.12A14.98 14.98 0 009.631 8.41m5.96 5.96a14.926 14.926 0 01-5.841 2.58m-.119-8.54a6 6 0 00-7.381 5.84h4.8m2.581-5.84a14.927 14.927 0 00-2.58 5.84m2.699 2.7c-.103.021-.207.041-.311.06a15.09 15.09 0 01-2.448-2.448 14.9 14.9 0 01.06-.312m-2.24 2.39a4.493 4.493 0 00-1.757 4.306 4.493 4.493 0 004.306-1.758M16.5 9a1.5 1.5 0 11-3 0 1.5 1.5 0 013 0z" />
            </svg>
          {/if}
        </div>
        <!-- Decorative rings -->
        <div class="absolute inset-0 -m-2 rounded-3xl border border-phantom-ear-accent/20 animate-ring-pulse"></div>
        <div class="absolute inset-0 -m-4 rounded-3xl border border-phantom-ear-accent/10 animate-ring-pulse" style="animation-delay: 0.2s;"></div>
      </div>

      <!-- Title -->
      <h1 class="text-3xl font-bold text-phantom-ear-text mb-2">{step.title}</h1>
      <p class="text-phantom-ear-accent font-medium mb-4">{step.subtitle}</p>
      <p class="text-phantom-ear-text-muted max-w-md mb-8">{step.description}</p>

      <!-- Step-specific content -->
      {#if step.features}
        <div class="flex gap-4 mb-8">
          {#each step.features as feature}
            <div class="flex items-center gap-2 px-4 py-2 rounded-xl bg-phantom-ear-surface/50 border border-phantom-ear-border/50">
              {#if feature.icon === "lock"}
                <svg class="w-4 h-4 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                </svg>
              {:else if feature.icon === "eye-off"}
                <svg class="w-4 h-4 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                </svg>
              {:else if feature.icon === "zap"}
                <svg class="w-4 h-4 text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                </svg>
              {/if}
              <span class="text-sm text-phantom-ear-text">{feature.text}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#if step.tips}
        <div class="space-y-3 mb-8">
          {#each step.tips as tip}
            <div class="flex items-center justify-center gap-4">
              <kbd class="px-3 py-1.5 rounded-lg bg-phantom-ear-surface border border-phantom-ear-border text-sm font-mono text-phantom-ear-text">{tip.shortcut}</kbd>
              <span class="text-sm text-phantom-ear-text-muted">{tip.label}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#if step.note}
        <div class="px-4 py-3 rounded-xl bg-phantom-ear-surface/50 border border-phantom-ear-border/50 mb-8">
          <p class="text-sm text-phantom-ear-text-muted">
            <span class="text-phantom-ear-accent">Tip:</span> {step.note}
          </p>
        </div>
      {/if}

      {#if step.examples}
        <div class="space-y-2 mb-8">
          <p class="text-xs text-phantom-ear-text-muted uppercase tracking-wide mb-3">Try asking:</p>
          {#each step.examples as example}
            <div class="px-4 py-2 rounded-xl bg-phantom-ear-surface/50 border border-phantom-ear-border/50 text-sm text-phantom-ear-text italic">
              "{example}"
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Navigation -->
    <div class="flex items-center justify-between w-full max-w-md">
      <button
        onclick={prevStep}
        disabled={currentStep === 0}
        class="px-6 py-2.5 rounded-xl text-sm font-medium text-phantom-ear-text-muted hover:text-phantom-ear-text transition-colors disabled:opacity-0 disabled:pointer-events-none"
      >
        Back
      </button>

      <!-- Step indicators -->
      <div class="flex gap-1.5">
        {#each steps as _, i}
          <button
            onclick={() => { currentStep = i; }}
            class="w-2 h-2 rounded-full transition-all duration-300 {i === currentStep ? 'w-6 bg-phantom-ear-accent' : 'bg-phantom-ear-surface-hover hover:bg-phantom-ear-border'}"
          ></button>
        {/each}
      </div>

      <button
        onclick={nextStep}
        class="px-6 py-2.5 rounded-xl text-sm font-medium bg-gradient-to-r from-phantom-ear-accent to-phantom-ear-purple text-white shadow-lg shadow-phantom-ear-accent/20 hover:opacity-90 transition-all"
      >
        {currentStep === steps.length - 1 ? "Get Started" : "Next"}
      </button>
    </div>
  </div>
</div>

<style>
  @keyframes float {
    0%, 100% { transform: translateY(0) translateX(0); }
    50% { transform: translateY(-20px) translateX(10px); }
  }

  @keyframes float-slow {
    0%, 100% { transform: translateY(0) translateX(0); }
    50% { transform: translateY(15px) translateX(-15px); }
  }

  @keyframes float-fast {
    0%, 100% { transform: translateY(0) translateX(0); }
    50% { transform: translateY(-10px) translateX(5px); }
  }

  :global(.animate-float) {
    animation: float 8s ease-in-out infinite;
  }

  :global(.animate-float-slow) {
    animation: float-slow 12s ease-in-out infinite;
  }

  :global(.animate-float-fast) {
    animation: float-fast 6s ease-in-out infinite;
  }

  @keyframes ring-pulse {
    0%, 100% { opacity: 0.3; transform: scale(1); }
    50% { opacity: 0.6; transform: scale(1.02); }
  }

  :global(.animate-ring-pulse) {
    animation: ring-pulse 3s ease-in-out infinite;
  }
</style>
