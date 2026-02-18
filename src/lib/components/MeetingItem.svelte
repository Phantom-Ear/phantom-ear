<script lang="ts">
  import type { MeetingListItem } from '$lib/types';

  // Predefined tag options
  const TAG_OPTIONS = [
    { value: 'standup', label: 'Standup', emoji: '📅' },
    { value: '1:1', label: '1:1', emoji: '👥' },
    { value: 'interview', label: 'Interview', emoji: '🎯' },
    { value: 'client', label: 'Client Call', emoji: '🤝' },
    { value: 'team', label: 'Team Meeting', emoji: '👨‍👩‍👧‍👦' },
    { value: 'training', label: 'Training', emoji: '📚' },
    { value: 'brainstorm', label: 'Brainstorm', emoji: '💡' },
  ];

  let {
    meeting,
    isActive = false,
    isKeyboardSelected = false,
    collapsed = false,
    onSelect,
    onRename,
    onTogglePin,
    onDelete,
    onUpdateTags,
  }: {
    meeting: MeetingListItem;
    isActive?: boolean;
    isKeyboardSelected?: boolean;
    collapsed?: boolean;
    onSelect: () => void;
    onRename: (newTitle: string) => void;
    onTogglePin: () => void;
    onDelete: () => void;
    onUpdateTags?: (tags: string | null) => void;
  } = $props();

  let isEditing = $state(false);
  let editTitle = $state(meeting.title);
  let showMenu = $state(false);
  let showTagMenu = $state(false);
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

  // Get current tags as array
  function getCurrentTags(): string[] {
    if (!meeting.tags) return [];
    return meeting.tags.split(',').filter(t => t.trim());
  }

  function getTagLabel(tagValue: string): string {
    const tag = TAG_OPTIONS.find(t => t.value === tagValue);
    return tag ? `${tag.emoji} ${tag.label}` : tagValue;
  }

  function toggleTag(tagValue: string) {
    if (!onUpdateTags) return;
    const current = getCurrentTags();
    let newTags: string[];
    if (current.includes(tagValue)) {
      newTags = current.filter(t => t !== tagValue);
    } else {
      newTags = [...current, tagValue];
    }
    onUpdateTags(newTags.length > 0 ? newTags.join(',') : null);
  }
</script>

<div
  class="group relative flex items-center gap-2 px-3 py-2 rounded-lg cursor-pointer transition-colors {isActive ? 'bg-phantom-ear-surface-hover' : 'hover:bg-phantom-ear-surface'} {isKeyboardSelected ? 'meeting-item-selected ring-2 ring-phantom-ear-accent ring-inset' : ''}"
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
        {#if meeting.tags}
          <div class="flex flex-wrap gap-1 mt-1">
            {#each getCurrentTags() as tag}
              <span class="text-[10px] px-1.5 py-0.5 rounded bg-phantom-ear-purple/20 text-phantom-ear-purple">{getTagLabel(tag)}</span>
            {/each}
          </div>
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
            <button
              onclick={() => { showTagMenu = !showTagMenu; }}
              class="w-full px-3 py-1.5 text-left text-sm text-phantom-ear-text hover:bg-phantom-ear-surface-hover flex items-center gap-2"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
              </svg>
              Tags
            </button>
            {#if showTagMenu}
              <div class="pl-4 pr-2 py-1 bg-phantom-ear-bg border border-phantom-ear-border rounded-lg mx-2 mt-1 mb-1">
                {#each TAG_OPTIONS as tag}
                  <button
                    onclick={() => toggleTag(tag.value)}
                    class="w-full px-2 py-1 text-left text-xs hover:bg-phantom-ear-surface-hover rounded flex items-center justify-between {getCurrentTags().includes(tag.value) ? 'text-phantom-ear-accent' : 'text-phantom-ear-text-muted'}"
                  >
                    <span>{tag.emoji} {tag.label}</span>
                    {#if getCurrentTags().includes(tag.value)}
                      <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/>
                      </svg>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
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
