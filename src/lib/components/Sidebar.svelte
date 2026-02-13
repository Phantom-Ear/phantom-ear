<script lang="ts">
  import type { View, MeetingListItem } from '$lib/types';
  import { themeStore } from '$lib/stores/theme.svelte';
  import MeetingItem from './MeetingItem.svelte';

  let {
    collapsed = false,
    currentView = 'home',
    pinnedMeetings = [],
    recentMeetings = [],
    activeMeetingId = null,
    searchQuery = '',
    isRecording = false,
    onToggle,
    onNavigate,
    onSelectMeeting,
    onRenameMeeting,
    onTogglePinMeeting,
    onDeleteMeeting,
    onSearch,
    onOpenSearchOverlay,
    onToggleRecording,
  }: {
    collapsed?: boolean;
    currentView?: View;
    pinnedMeetings?: MeetingListItem[];
    recentMeetings?: MeetingListItem[];
    activeMeetingId?: string | null;
    searchQuery?: string;
    isRecording?: boolean;
    onToggle: () => void;
    onNavigate: (view: View) => void;
    onSelectMeeting: (id: string) => void;
    onRenameMeeting: (id: string, newTitle: string) => void;
    onTogglePinMeeting: (id: string) => void;
    onDeleteMeeting: (id: string) => void;
    onSearch?: (query: string) => void;
    onOpenSearchOverlay?: () => void;
    onToggleRecording?: () => void;
  } = $props();

  let localSearchQuery = $state(searchQuery);
  let selectedIndex = $state(-1); // -1 means no selection
  let sidebarEl: HTMLElement | null = null;

  // Combined list of all meetings for keyboard navigation
  const allMeetings = $derived([...pinnedMeetings, ...recentMeetings]);

  function handleSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    localSearchQuery = value;
    onSearch?.(value);
  }

  // Clear keyboard selection when clicking on a meeting
  function handleMeetingClick(callback: () => void) {
    selectedIndex = -1; // Clear keyboard selection
    callback();
  }

  // Handle keyboard navigation
  function handleKeydown(e: KeyboardEvent) {
    // Skip if in input field
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;

    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      if (allMeetings.length === 0) return;
      selectedIndex = Math.min(selectedIndex + 1, allMeetings.length - 1);
      scrollSelectedIntoView();
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      if (allMeetings.length === 0) return;
      selectedIndex = Math.max(selectedIndex - 1, 0);
      scrollSelectedIntoView();
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      e.preventDefault();
      const meeting = allMeetings[selectedIndex];
      if (meeting) {
        onSelectMeeting(meeting.id);
        selectedIndex = -1; // Clear after selection
      }
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && selectedIndex >= 0) {
      e.preventDefault();
      const meeting = allMeetings[selectedIndex];
      if (meeting) {
        onDeleteMeeting(meeting.id);
        // Adjust selection after deletion
        if (selectedIndex >= allMeetings.length - 1) {
          selectedIndex = Math.max(0, allMeetings.length - 2);
        }
      }
    }
  }

  function scrollSelectedIntoView() {
    if (sidebarEl) {
      const selected = sidebarEl.querySelector('.meeting-item-selected');
      if (selected) {
        selected.scrollIntoView({ block: 'nearest' });
      }
    }
  }

  // Reset selection when meetings change
  $effect(() => {
    if (selectedIndex >= allMeetings.length) {
      selectedIndex = Math.max(-1, allMeetings.length - 1);
    }
  });

  const navItems: { view: View; label: string; icon: string }[] = [
    { view: 'home', label: 'Home', icon: 'home' },
    { view: 'phomy', label: 'Phomy', icon: 'ghost' },
    { view: 'settings', label: 'Settings', icon: 'cog' },
  ];
</script>

<aside
  bind:this={sidebarEl}
  onkeydown={handleKeydown}
  tabindex="0"
  class="flex flex-col h-full bg-phantom-ear-surface border-r border-phantom-ear-border transition-all duration-200 ease-in-out {collapsed ? 'w-16' : 'w-64'} focus:outline-none"
>
  <!-- Header: Search + Collapse Toggle -->
  <div class="px-3 pt-3 pb-2">
    <div class="flex items-center gap-2">
      {#if !collapsed}
        <div class="relative flex-1">
          <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-phantom-ear-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            type="text"
            placeholder="Search..."
            value={localSearchQuery}
            oninput={handleSearchInput}
            onclick={() => onOpenSearchOverlay?.()}
            readonly
            class="w-full pl-8 pr-8 py-2 text-xs bg-phantom-ear-bg border border-phantom-ear-border rounded-lg text-phantom-ear-text placeholder:text-phantom-ear-text-muted focus:outline-none focus:border-phantom-ear-accent transition-colors cursor-pointer"
          />
          <span class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-phantom-ear-text-muted bg-phantom-ear-surface-hover px-1.5 py-0.5 rounded">⌘K</span>
        </div>
      {/if}
      <!-- Collapse Toggle Button -->
      <button
        onclick={onToggle}
        class="p-2 rounded-lg text-phantom-ear-text-muted hover:bg-phantom-ear-surface-hover hover:text-phantom-ear-text transition-colors shrink-0"
        title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          {#if collapsed}
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" />
          {:else}
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
          {/if}
        </svg>
      </button>
    </div>
  </div>

  <!-- Navigation - Vertical Column -->
  <nav class="px-2 py-2">
    <div class="flex flex-col gap-0.5">
      <!-- New Recording Button - same style as nav items -->
      <button
        onclick={() => onToggleRecording?.()}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-lg transition-colors {isRecording ? 'bg-phantom-ear-danger/10 text-phantom-ear-danger' : 'text-phantom-ear-text-muted hover:bg-phantom-ear-surface-hover hover:text-phantom-ear-text'}"
        title={collapsed ? (isRecording ? 'Stop recording' : 'New Recording') : undefined}
      >
        {#if isRecording}
          <span class="w-5 h-5 flex items-center justify-center shrink-0">
            <span class="w-2.5 h-2.5 rounded-full bg-phantom-ear-danger animate-pulse"></span>
          </span>
        {:else}
          <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="4" fill="currentColor" />
            <circle cx="12" cy="12" r="8" stroke-width="2" />
          </svg>
        {/if}
        {#if !collapsed}
          <span class="text-sm font-medium">{isRecording ? 'Recording...' : 'New Recording'}</span>
        {/if}
      </button>

      {#each navItems as item}
        <button
          onclick={() => onNavigate(item.view)}
          class="w-full flex items-center gap-3 px-3 py-2 rounded-lg transition-colors {currentView === item.view ? 'bg-phantom-ear-accent/10 text-phantom-ear-accent' : 'text-phantom-ear-text-muted hover:bg-phantom-ear-surface-hover hover:text-phantom-ear-text'}"
          title={collapsed ? item.label : undefined}
        >
          {#if item.icon === 'home'}
            <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
            </svg>
          {:else if item.icon === 'ghost'}
            <svg class="w-5 h-5 shrink-0" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12 2C7.58 2 4 5.58 4 10v9c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1v-1c0-.55.45-1 1-1s1 .45 1 1v1c0 .55.45 1 1 1s1-.45 1-1V10c0-4.42-3.58-8-8-8zm-2 10a1.5 1.5 0 110-3 1.5 1.5 0 010 3zm4 0a1.5 1.5 0 110-3 1.5 1.5 0 010 3z"/>
            </svg>
          {:else if item.icon === 'cog'}
            <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          {/if}
          {#if !collapsed}
            <span class="text-sm font-medium">{item.label}</span>
          {/if}
        </button>
      {/each}
    </div>
  </nav>

  <!-- Meetings List -->
  <div class="flex-1 overflow-y-auto px-2 py-2">
    {#if pinnedMeetings.length > 0}
      <div class="mb-4">
        {#if !collapsed}
          <h3 class="px-3 py-2 text-xs font-semibold text-phantom-ear-text-muted uppercase tracking-wide">Pinned</h3>
        {/if}
        <div class="space-y-0.5">
          {#each pinnedMeetings as meeting, index (meeting.id)}
            <MeetingItem
              {meeting}
              isActive={meeting.id === activeMeetingId}
              isKeyboardSelected={selectedIndex === index}
              {collapsed}
              onSelect={() => handleMeetingClick(() => onSelectMeeting(meeting.id))}
              onRename={(title) => { selectedIndex = -1; onRenameMeeting(meeting.id, title); }}
              onTogglePin={() => { selectedIndex = -1; onTogglePinMeeting(meeting.id); }}
              onDelete={() => { selectedIndex = -1; onDeleteMeeting(meeting.id); }}
            />
          {/each}
        </div>
      </div>
    {/if}

    {#if recentMeetings.length > 0}
      <div>
        {#if !collapsed}
          <h3 class="px-3 py-2 text-xs font-semibold text-phantom-ear-text-muted uppercase tracking-wide">Recent</h3>
        {/if}
        <div class="space-y-0.5">
          {#each recentMeetings as meeting, index (meeting.id)}
            <MeetingItem
              {meeting}
              isActive={meeting.id === activeMeetingId}
              isKeyboardSelected={selectedIndex === pinnedMeetings.length + index}
              {collapsed}
              onSelect={() => handleMeetingClick(() => onSelectMeeting(meeting.id))}
              onRename={(title) => { selectedIndex = -1; onRenameMeeting(meeting.id, title); }}
              onTogglePin={() => { selectedIndex = -1; onTogglePinMeeting(meeting.id); }}
              onDelete={() => { selectedIndex = -1; onDeleteMeeting(meeting.id); }}
            />
          {/each}
        </div>
      </div>
    {/if}

    {#if pinnedMeetings.length === 0 && recentMeetings.length === 0 && !collapsed}
      <div class="px-3 py-4 text-center">
        <p class="text-xs text-phantom-ear-text-muted">No meetings yet</p>
        <p class="text-xs text-phantom-ear-text-muted mt-1">Start recording to create one</p>
      </div>
    {/if}
  </div>

  <!-- Footer: Theme Toggle -->
  <div class="px-2 py-3 border-t border-phantom-ear-border/50">
    <button
      onclick={() => themeStore.toggleTheme()}
      class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-phantom-ear-text-muted hover:bg-phantom-ear-surface-hover hover:text-phantom-ear-text transition-colors"
      title={collapsed ? (themeStore.theme === 'dark' ? 'Light mode' : 'Dark mode') : undefined}
    >
      {#if themeStore.theme === 'dark'}
        <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
        </svg>
        {#if !collapsed}
          <span class="text-sm font-medium">Light mode</span>
        {/if}
      {:else}
        <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
        </svg>
        {#if !collapsed}
          <span class="text-sm font-medium">Dark mode</span>
        {/if}
      {/if}
    </button>
  </div>
</aside>
