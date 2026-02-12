<script lang="ts">
  import { onMount } from "svelte";
  import { meetingsStore } from "$lib/stores/meetings.svelte";
  import type { MeetingListItem } from "$lib/types";

  // Props
  let { isOpen = false, onClose = () => {} }: { isOpen?: boolean; onClose?: () => void } = $props();

  // State
  let searchQuery = $state("");
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | null = $state(null);
  let resultsContainer: HTMLDivElement | null = $state(null);

  // Derived results
  let filteredMeetings = $derived.by(() => {
    if (!searchQuery.trim()) {
      return meetingsStore.meetings.slice(0, 10);
    }
    const query = searchQuery.toLowerCase();
    return meetingsStore.meetings.filter(
      (m) =>
        m.title.toLowerCase().includes(query) ||
        (m.first_segment_text && m.first_segment_text.toLowerCase().includes(query))
    ).slice(0, 10);
  });

  // Quick actions
  const quickActions = [
    { id: "start-recording", label: "Start Recording", icon: "recording", action: () => {} },
    { id: "settings", label: "Open Settings", icon: "settings", action: () => {} },
  ];

  let allResults = $derived.by(() => {
    const actions = searchQuery.trim() === "" ? quickActions : 
      quickActions.filter(a => a.label.toLowerCase().includes(searchQuery.toLowerCase()));
    return [...actions.map(a => ({ type: 'action' as const, ...a })), 
            ...filteredMeetings.map(m => ({ type: 'meeting' as const, ...m }))];
  });

  // Focus input when opened
  $effect(() => {
    if (isOpen && inputEl) {
      inputEl.focus();
      searchQuery = "";
      selectedIndex = 0;
    }
  });

  // Handle keyboard navigation
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, allResults.length - 1);
      scrollSelectedIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      scrollSelectedIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      selectResult(selectedIndex);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  function scrollSelectedIntoView() {
    if (resultsContainer) {
      const selected = resultsContainer.querySelector(".selected");
      if (selected) {
        selected.scrollIntoView({ block: "nearest" });
      }
    }
  }

  function selectResult(index: number) {
    const result = allResults[index];
    if (!result) return;

    if (result.type === 'action') {
      // Handle action
      onClose();
    } else if (result.type === 'meeting') {
      // Navigate to meeting
      meetingsStore.setActive(result.id);
      onClose();
    }
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays} days ago`;
    return date.toLocaleDateString();
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div 
    class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm"
    onclick={onClose}
    onkeydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-label="Quick Search"
  >
    <!-- Search Modal -->
    <div 
      class="absolute top-[20%] left-1/2 -translate-x-1/2 w-full max-w-xl mx-4"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="glass-strong rounded-2xl border border-phantom-ear-border shadow-2xl overflow-hidden">
        <!-- Search Input -->
        <div class="flex items-center gap-3 px-4 py-3 border-b border-phantom-ear-border/50">
          <svg class="w-5 h-5 text-phantom-ear-text-muted shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input 
            bind:this={inputEl}
            bind:value={searchQuery}
            type="text"
            placeholder="Search meetings, transcripts..."
            class="flex-1 bg-transparent text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none text-base"
          />
          <kbd class="px-2 py-0.5 text-xs rounded bg-phantom-ear-surface text-phantom-ear-text-muted border border-phantom-ear-border">
            Esc
          </kbd>
        </div>

        <!-- Results -->
        <div 
          bind:this={resultsContainer}
          class="max-h-80 overflow-y-auto"
        >
          {#if allResults.length === 0}
            <div class="px-4 py-8 text-center text-phantom-ear-text-muted">
              <p>No results found</p>
              <p class="text-xs mt-1">Try a different search term</p>
            </div>
          {:else}
            {#each allResults as result, index (result.type === 'action' ? result.id : result.id)}
              <button
                class="w-full px-4 py-3 flex items-center gap-3 text-left transition-colors {selectedIndex === index ? 'bg-phantom-ear-accent/10 selected' : 'hover:bg-phantom-ear-surface-hover'}"
                onclick={() => selectResult(index)}
                onmouseenter={() => selectedIndex = index}
              >
                {#if result.type === 'action'}
                  <div class="w-8 h-8 rounded-lg bg-phantom-ear-accent/20 flex items-center justify-center shrink-0">
                    {#if result.icon === 'recording'}
                      <svg class="w-4 h-4 text-phantom-ear-danger" fill="currentColor" viewBox="0 0 24 24">
                        <circle cx="12" cy="12" r="6" />
                      </svg>
                    {:else if result.icon === 'settings'}
                      <svg class="w-4 h-4 text-phantom-ear-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      </svg>
                    {/if}
                  </div>
                  <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium text-phantom-ear-text">{result.label}</p>
                    <p class="text-xs text-phantom-ear-text-muted">Action</p>
                  </div>
                {:else}
                  <div class="w-8 h-8 rounded-lg bg-phantom-ear-purple/20 flex items-center justify-center shrink-0">
                    <svg class="w-4 h-4 text-phantom-ear-purple" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                  </div>
                  <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium text-phantom-ear-text truncate">{result.title}</p>
                    <p class="text-xs text-phantom-ear-text-muted">
                      {formatDate(result.created_at)} • {result.segment_count} segments
                    </p>
                  </div>
                  {#if result.pinned}
                    <svg class="w-4 h-4 text-phantom-ear-accent shrink-0" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"/>
                    </svg>
                  {/if}
                {/if}
              </button>
            {/each}
          {/if}
        </div>

        <!-- Footer Hints -->
        <div class="px-4 py-2 border-t border-phantom-ear-border/50 flex items-center gap-4 text-xs text-phantom-ear-text-muted">
          <span class="flex items-center gap-1">
            <kbd class="px-1.5 py-0.5 rounded bg-phantom-ear-surface border border-phantom-ear-border">↑↓</kbd>
            Navigate
          </span>
          <span class="flex items-center gap-1">
            <kbd class="px-1.5 py-0.5 rounded bg-phantom-ear-surface border border-phantom-ear-border">Enter</kbd>
            Select
          </span>
          <span class="flex items-center gap-1">
            <kbd class="px-1.5 py-0.5 rounded bg-phantom-ear-surface border border-phantom-ear-border">Esc</kbd>
            Close
          </span>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .glass-strong {
    background: rgba(47, 47, 47, 0.95);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
  }

  :global([data-theme="light"]) .glass-strong {
    background: rgba(255, 255, 255, 0.95);
  }
</style>
