<script lang="ts">
  import type { MeetingListItem } from '$lib/types';

  let {
    meeting,
    isActive = false,
    collapsed = false,
    onSelect,
    onRename,
    onTogglePin,
    onDelete,
  }: {
    meeting: MeetingListItem;
    isActive?: boolean;
    collapsed?: boolean;
    onSelect: () => void;
    onRename: (newTitle: string) => void;
    onTogglePin: () => void;
    onDelete: () => void;
  } = $props();

  let isEditing = $state(false);
  let editTitle = $state(meeting.title);
  let showMenu = $state(false);
  let inputEl: HTMLInputElement | null = null;

  function startEditing() {
    editTitle = meeting.title;
    isEditing = true;
    showMenu = false;
    requestAnimationFrame(() => {
      inputEl?.focus();
      inputEl?.select();
    });
  }

  function finishEditing() {
    if (editTitle.trim() && editTitle.trim() !== meeting.title) {
      onRename(editTitle.trim());
    }
    isEditing = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      finishEditing();
    } else if (e.key === 'Escape') {
      isEditing = false;
    }
  }
</script>

<div
  class="group relative flex items-center gap-2 px-3 py-2 rounded-lg cursor-pointer transition-colors {isActive ? 'bg-phantom-ear-surface-hover' : 'hover:bg-phantom-ear-surface'}"
  onclick={onSelect}
  onkeydown={(e) => e.key === 'Enter' && onSelect()}
  role="button"
  tabindex="0"
>
  {#if collapsed}
    <!-- Collapsed: show only icon -->
    <div class="w-6 h-6 rounded bg-phantom-ear-accent/20 flex items-center justify-center">
      <svg class="w-3.5 h-3.5 text-phantom-ear-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
    </div>
  {:else}
    <!-- Expanded: show title -->
    <div class="flex-1 min-w-0">
      {#if isEditing}
        <input
          bind:this={inputEl}
          bind:value={editTitle}
          onblur={finishEditing}
          onkeydown={handleKeydown}
          onclick={(e) => e.stopPropagation()}
          class="w-full bg-phantom-ear-bg border border-phantom-ear-accent rounded px-2 py-0.5 text-sm text-phantom-ear-text focus:outline-none"
        />
      {:else}
        <div class="flex items-center gap-1.5 min-w-0">
          {#if meeting.pinned}
            <svg class="w-3.5 h-3.5 shrink-0 text-phantom-ear-accent" fill="currentColor" viewBox="0 0 24 24">
              <path d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"/>
            </svg>
          {/if}
          <span class="block truncate text-sm text-phantom-ear-text">{meeting.title}</span>
        </div>
        {#if meeting.segment_count > 0}
          <span class="text-xs text-phantom-ear-text-muted">{meeting.segment_count} segments</span>
        {/if}
      {/if}
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-1 transition-opacity {showMenu ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'}">
      <div class="relative">
        <button
          onclick={(e) => { e.stopPropagation(); showMenu = !showMenu; }}
          class="p-1 rounded hover:bg-phantom-ear-surface-hover text-phantom-ear-text-muted"
          title="More options"
        >
          <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
            <circle cx="12" cy="5" r="2"/>
            <circle cx="12" cy="12" r="2"/>
            <circle cx="12" cy="19" r="2"/>
          </svg>
        </button>

        {#if showMenu}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="absolute right-0 top-full mt-1 py-1 bg-phantom-ear-surface border border-phantom-ear-border rounded-lg shadow-lg z-50 min-w-32"
            onclick={(e) => e.stopPropagation()}
          >
            <button
              onclick={() => startEditing()}
              class="w-full px-3 py-1.5 text-left text-sm text-phantom-ear-text hover:bg-phantom-ear-surface-hover flex items-center gap-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
              Rename
            </button>
            <button
              onclick={() => { onTogglePin(); showMenu = false; }}
              class="w-full px-3 py-1.5 text-left text-sm text-phantom-ear-text hover:bg-phantom-ear-surface-hover flex items-center gap-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
              </svg>
              {meeting.pinned ? 'Unpin' : 'Pin'}
            </button>
            <hr class="my-1 border-phantom-ear-border" />
            <button
              onclick={() => { onDelete(); showMenu = false; }}
              class="w-full px-3 py-1.5 text-left text-sm text-phantom-ear-danger hover:bg-phantom-ear-surface-hover flex items-center gap-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
              Delete
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Click outside to close menu -->
{#if showMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-40"
    onclick={() => showMenu = false}
  ></div>
{/if}
