<script lang="ts">
  import type { View, Meeting } from '$lib/types';
  import { themeStore } from '$lib/stores/theme.svelte';
  import MeetingItem from './MeetingItem.svelte';

  let {
    collapsed = false,
    currentView = 'home',
    pinnedMeetings = [],
    recentMeetings = [],
    activeMeetingId = null,
    onToggle,
    onNavigate,
    onSelectMeeting,
    onRenameMeeting,
    onTogglePinMeeting,
    onDeleteMeeting,
  }: {
    collapsed?: boolean;
    currentView?: View;
    pinnedMeetings?: Meeting[];
    recentMeetings?: Meeting[];
    activeMeetingId?: string | null;
    onToggle: () => void;
    onNavigate: (view: View) => void;
    onSelectMeeting: (id: string) => void;
    onRenameMeeting: (id: string, newTitle: string) => void;
    onTogglePinMeeting: (id: string) => void;
    onDeleteMeeting: (id: string) => void;
  } = $props();

  const navItems: { view: View; label: string; icon: string }[] = [
    { view: 'home', label: 'Home', icon: 'home' },
    { view: 'genie', label: 'Genie', icon: 'sparkles' },
    { view: 'settings', label: 'Settings', icon: 'cog' },
  ];
</script>

<aside
  class="flex flex-col h-full bg-sidecar-surface border-r border-sidecar-border transition-all duration-200 ease-in-out {collapsed ? 'w-16' : 'w-64'}"
>
  <!-- Header -->
  <div class="flex items-center gap-3 px-4 py-4 border-b border-sidecar-border/50">
    <button
      onclick={onToggle}
      class="p-2 rounded-lg hover:bg-sidecar-surface-hover transition-colors text-sidecar-text-muted"
      title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
      </svg>
    </button>
    {#if !collapsed}
      <span class="font-semibold text-sidecar-text">Sidecar</span>
    {/if}
  </div>

  <!-- Navigation -->
  <nav class="px-2 py-3 space-y-1">
    {#each navItems as item}
      <button
        onclick={() => onNavigate(item.view)}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-lg transition-colors {currentView === item.view ? 'bg-sidecar-accent/10 text-sidecar-accent' : 'text-sidecar-text-muted hover:bg-sidecar-surface-hover hover:text-sidecar-text'}"
        title={collapsed ? item.label : undefined}
      >
        {#if item.icon === 'home'}
          <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
          </svg>
        {:else if item.icon === 'sparkles'}
          <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z" />
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
  </nav>

  <!-- Meetings List -->
  <div class="flex-1 overflow-y-auto px-2 py-2">
    {#if pinnedMeetings.length > 0}
      <div class="mb-4">
        {#if !collapsed}
          <h3 class="px-3 py-2 text-xs font-semibold text-sidecar-text-muted uppercase tracking-wide">Pinned</h3>
        {/if}
        <div class="space-y-0.5">
          {#each pinnedMeetings as meeting (meeting.id)}
            <MeetingItem
              {meeting}
              isActive={meeting.id === activeMeetingId}
              {collapsed}
              onSelect={() => onSelectMeeting(meeting.id)}
              onRename={(title) => onRenameMeeting(meeting.id, title)}
              onTogglePin={() => onTogglePinMeeting(meeting.id)}
              onDelete={() => onDeleteMeeting(meeting.id)}
            />
          {/each}
        </div>
      </div>
    {/if}

    {#if recentMeetings.length > 0}
      <div>
        {#if !collapsed}
          <h3 class="px-3 py-2 text-xs font-semibold text-sidecar-text-muted uppercase tracking-wide">Recent</h3>
        {/if}
        <div class="space-y-0.5">
          {#each recentMeetings as meeting (meeting.id)}
            <MeetingItem
              {meeting}
              isActive={meeting.id === activeMeetingId}
              {collapsed}
              onSelect={() => onSelectMeeting(meeting.id)}
              onRename={(title) => onRenameMeeting(meeting.id, title)}
              onTogglePin={() => onTogglePinMeeting(meeting.id)}
              onDelete={() => onDeleteMeeting(meeting.id)}
            />
          {/each}
        </div>
      </div>
    {/if}

    {#if pinnedMeetings.length === 0 && recentMeetings.length === 0 && !collapsed}
      <div class="px-3 py-4 text-center">
        <p class="text-xs text-sidecar-text-muted">No meetings yet</p>
        <p class="text-xs text-sidecar-text-muted mt-1">Start recording to create one</p>
      </div>
    {/if}
  </div>

  <!-- Footer: Theme Toggle -->
  <div class="px-2 py-3 border-t border-sidecar-border/50">
    <button
      onclick={() => themeStore.toggleTheme()}
      class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sidecar-text-muted hover:bg-sidecar-surface-hover hover:text-sidecar-text transition-colors"
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
