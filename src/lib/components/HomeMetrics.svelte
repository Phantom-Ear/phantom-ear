<script lang="ts">
  import { meetingsStore } from "$lib/stores/meetings.svelte";
  import { onMount } from "svelte";

  // Animation state
  let mounted = $state(false);

  onMount(() => {
    setTimeout(() => {
      mounted = true;
    }, 100);
  });

  // Calculate metrics from meetings store
  const totalMeetings = $derived(meetingsStore.meetings.length);

  // Meetings this week
  const meetingsThisWeek = $derived.by(() => {
    const now = new Date();
    const weekStart = new Date(now);
    weekStart.setDate(now.getDate() - now.getDay());
    weekStart.setHours(0, 0, 0, 0);

    return meetingsStore.meetings.filter(m => {
      const meetingDate = new Date(m.created_at);
      return meetingDate >= weekStart;
    }).length;
  });

  // Total recording time (in seconds)
  const totalRecordingTime = $derived.by(() => {
    return meetingsStore.meetings.reduce((sum, m) => sum + Math.floor((m.duration_ms || 0) / 1000), 0);
  });

  // Average meeting duration
  const averageDuration = $derived.by(() => {
    if (totalMeetings === 0) return 0;
    return Math.round(totalRecordingTime / totalMeetings);
  });

  // Most active day
  const mostActiveDay = $derived.by(() => {
    if (totalMeetings === 0) return null;

    const dayCounts: Record<string, number> = {};
    meetingsStore.meetings.forEach(m => {
      const day = new Date(m.created_at).toLocaleDateString('en-US', { weekday: 'short' });
      dayCounts[day] = (dayCounts[day] || 0) + 1;
    });

    const sorted = Object.entries(dayCounts).sort((a, b) => b[1] - a[1]);
    return sorted[0]?.[0] || null;
  });

  // Weekly activity data (last 7 days)
  const weeklyActivity = $derived.by(() => {
    const days = ['Sat', 'Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri'];
    const now = new Date();
    const weekData: { day: string; count: number; isToday: boolean }[] = [];

    for (let i = 6; i >= 0; i--) {
      const date = new Date(now);
      date.setDate(now.getDate() - i);
      const dayIndex = date.getDay();
      const dayName = days[dayIndex === 0 ? 1 : dayIndex === 6 ? 0 : dayIndex + 1]; // Adjust for Sat/Sun

      const count = meetingsStore.meetings.filter(m => {
        const meetingDate = new Date(m.created_at);
        return meetingDate.toDateString() === date.toDateString();
      }).length;

      weekData.push({
        day: days[date.getDay() === 0 ? 1 : date.getDay() === 6 ? 0 : date.getDay() + 1],
        count,
        isToday: i === 0
      });
    }

    // Re-sort to show proper day order
    return weekData;
  });

  // Max count for scaling bars
  const maxWeeklyCount = $derived(Math.max(...weeklyActivity.map(d => d.count), 1));

  // Last meeting time
  const lastMeetingTime = $derived.by(() => {
    if (totalMeetings === 0) return null;
    const sorted = [...meetingsStore.meetings].sort((a, b) =>
      new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );
    return sorted[0]?.created_at || null;
  });

  // Format duration
  function formatDuration(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    const hours = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${mins}m`;
    }
    return `${mins}m`;
  }

  // Format relative time
  function formatRelativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }
</script>

<div class="w-full max-w-xl {mounted ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'} transition-all duration-500 ease-out">
  <!-- Stats Cards -->
  <div class="grid grid-cols-2 gap-4 mb-4">
    <!-- Total Meetings -->
    <div class="group relative overflow-hidden rounded-2xl bg-gradient-to-br from-phantom-ear-surface/80 to-phantom-ear-surface/40 border border-phantom-ear-border/50 p-5 hover:border-phantom-ear-accent/30 transition-all duration-300">
      <div class="absolute inset-0 bg-gradient-to-br from-phantom-ear-accent/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
      <div class="relative flex items-center gap-4">
        <div class="w-12 h-12 rounded-xl bg-gradient-to-br from-phantom-ear-accent/20 to-phantom-ear-accent/5 flex items-center justify-center border border-phantom-ear-accent/20">
          <svg class="w-6 h-6 text-phantom-ear-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
        </div>
        <div>
          <p class="text-3xl font-bold text-phantom-ear-text tabular-nums">{totalMeetings}</p>
          <p class="text-xs font-medium text-phantom-ear-text-muted uppercase tracking-wider">Total Meetings</p>
        </div>
      </div>
    </div>

    <!-- Total Time -->
    <div class="group relative overflow-hidden rounded-2xl bg-gradient-to-br from-phantom-ear-surface/80 to-phantom-ear-surface/40 border border-phantom-ear-border/50 p-5 hover:border-phantom-ear-purple/30 transition-all duration-300">
      <div class="absolute inset-0 bg-gradient-to-br from-phantom-ear-purple/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
      <div class="relative flex items-center gap-4">
        <div class="w-12 h-12 rounded-xl bg-gradient-to-br from-phantom-ear-purple/20 to-phantom-ear-purple/5 flex items-center justify-center border border-phantom-ear-purple/20">
          <svg class="w-6 h-6 text-phantom-ear-purple" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="12" cy="12" r="10" />
            <polyline points="12 6 12 12 16 14" />
          </svg>
        </div>
        <div>
          <p class="text-3xl font-bold text-phantom-ear-text tabular-nums">{formatDuration(totalRecordingTime)}</p>
          <p class="text-xs font-medium text-phantom-ear-text-muted uppercase tracking-wider">Total Time</p>
        </div>
      </div>
    </div>
  </div>

  <!-- Quick Stats Pills -->
  <div class="flex flex-wrap gap-2 justify-center mb-4">
    <div class="flex items-center gap-2 px-4 py-2 rounded-full bg-phantom-ear-surface/60 border border-phantom-ear-border/50">
      <span class="text-sm font-semibold text-phantom-ear-accent tabular-nums">{meetingsThisWeek}</span>
      <span class="text-xs text-phantom-ear-text-muted">This Week</span>
    </div>

    <div class="flex items-center gap-2 px-4 py-2 rounded-full bg-phantom-ear-surface/60 border border-phantom-ear-border/50">
      <span class="text-sm font-semibold text-phantom-ear-text tabular-nums">{formatDuration(averageDuration)}</span>
      <span class="text-xs text-phantom-ear-text-muted">Avg Duration</span>
    </div>

    {#if mostActiveDay}
      <div class="flex items-center gap-2 px-4 py-2 rounded-full bg-phantom-ear-surface/60 border border-phantom-ear-border/50">
        <span class="text-sm font-semibold text-phantom-ear-text">{mostActiveDay}</span>
        <span class="text-xs text-phantom-ear-text-muted">Most Active</span>
      </div>
    {/if}

    {#if lastMeetingTime}
      <div class="flex items-center gap-2 px-4 py-2 rounded-full bg-phantom-ear-surface/60 border border-phantom-ear-border/50">
        <span class="text-sm font-semibold text-phantom-ear-text tabular-nums">{formatRelativeTime(lastMeetingTime)}</span>
        <span class="text-xs text-phantom-ear-text-muted">Last Meeting</span>
      </div>
    {/if}
  </div>

  <!-- Weekly Activity -->
  {#if totalMeetings > 0}
    <div class="rounded-2xl bg-phantom-ear-surface/40 border border-phantom-ear-border/50 p-5">
      <div class="flex justify-between items-center mb-4">
        <span class="text-sm font-medium text-phantom-ear-text">This Week</span>
        <span class="text-xs text-phantom-ear-text-muted">{meetingsThisWeek} recording{meetingsThisWeek !== 1 ? 's' : ''}</span>
      </div>

      <div class="flex items-end justify-between gap-2 h-20">
        {#each weeklyActivity as day, i}
          <div class="flex-1 flex flex-col items-center gap-2">
            <div class="w-full relative">
              <div
                class="w-full rounded-md transition-all duration-500 ease-out {day.count > 0 ? 'bg-gradient-to-t from-phantom-ear-accent to-phantom-ear-purple' : 'bg-phantom-ear-border/50'}"
                style="height: {day.count > 0 ? Math.max(16, (day.count / maxWeeklyCount) * 64) : 4}px; animation-delay: {i * 50}ms"
              >
                {#if day.count > 0}
                  <span class="absolute -top-5 left-1/2 -translate-x-1/2 text-[10px] font-semibold text-phantom-ear-text tabular-nums">
                    {day.count}
                  </span>
                {/if}
              </div>
            </div>
            <span class="text-[10px] font-medium {day.isToday ? 'text-phantom-ear-accent' : 'text-phantom-ear-text-muted'}">
              {day.day}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
