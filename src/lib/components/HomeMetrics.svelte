<script lang="ts">
  import { meetingsStore } from "$lib/stores/meetings.svelte";
  import { onMount } from "svelte";

  // Animation state
  let mounted = $state(false);

  onMount(() => {
    // Trigger animations after mount
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
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    const now = new Date();
    const weekData: { day: string; count: number; isToday: boolean }[] = [];

    for (let i = 6; i >= 0; i--) {
      const date = new Date(now);
      date.setDate(now.getDate() - i);
      const dayName = days[date.getDay()];

      const count = meetingsStore.meetings.filter(m => {
        const meetingDate = new Date(m.created_at);
        return meetingDate.toDateString() === date.toDateString();
      }).length;

      weekData.push({
        day: dayName,
        count,
        isToday: i === 0
      });
    }

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

<div class="metrics-container" class:mounted>
  <!-- Primary Metrics Row -->
  <div class="primary-metrics">
    <!-- Total Meetings Card -->
    <div class="metric-card glass-card animate-stagger-in">
      <div class="metric-icon blue">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
        </svg>
      </div>
      <div class="metric-content">
        <span class="metric-value">{totalMeetings}</span>
        <span class="metric-label">Total Meetings</span>
      </div>
    </div>

    <!-- Total Time Card -->
    <div class="metric-card glass-card animate-stagger-in">
      <div class="metric-icon purple">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      </div>
      <div class="metric-content">
        <span class="metric-value">{formatDuration(totalRecordingTime)}</span>
        <span class="metric-label">Total Time</span>
      </div>
    </div>
  </div>

  <!-- Secondary Metrics Row -->
  <div class="secondary-metrics">
    <div class="metric-pill glass-card animate-stagger-in">
      <span class="pill-value">{meetingsThisWeek}</span>
      <span class="pill-label">This Week</span>
    </div>

    <div class="metric-pill glass-card animate-stagger-in">
      <span class="pill-value">{formatDuration(averageDuration)}</span>
      <span class="pill-label">Avg Duration</span>
    </div>

    {#if mostActiveDay}
      <div class="metric-pill glass-card animate-stagger-in">
        <span class="pill-value">{mostActiveDay}</span>
        <span class="pill-label">Most Active</span>
      </div>
    {/if}

    {#if lastMeetingTime}
      <div class="metric-pill glass-card animate-stagger-in">
        <span class="pill-value">{formatRelativeTime(lastMeetingTime)}</span>
        <span class="pill-label">Last Meeting</span>
      </div>
    {/if}
  </div>

  <!-- Weekly Activity Chart -->
  {#if totalMeetings > 0}
    <div class="activity-chart glass-card animate-stagger-in">
      <div class="chart-header">
        <span class="chart-title">This Week</span>
        <span class="chart-subtitle">{meetingsThisWeek} recording{meetingsThisWeek !== 1 ? 's' : ''}</span>
      </div>
      <div class="chart-bars">
        {#each weeklyActivity as day, i}
          <div class="bar-container">
            <div
              class="bar"
              class:today={day.isToday}
              class:active={day.count > 0}
              style="height: {day.count > 0 ? Math.max(20, (day.count / maxWeeklyCount) * 100) : 8}%"
            >
              {#if day.count > 0}
                <span class="bar-count">{day.count}</span>
              {/if}
            </div>
            <span class="bar-label" class:today={day.isToday}>{day.day}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .metrics-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
    max-width: 28rem;
    opacity: 0;
    transform: translateY(10px);
    transition: opacity 0.5s ease, transform 0.5s ease;
  }

  .metrics-container.mounted {
    opacity: 1;
    transform: translateY(0);
  }

  /* Primary Metrics */
  .primary-metrics {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem;
  }

  .metric-card {
    display: flex;
    align-items: center;
    gap: 0.875rem;
    padding: 1rem;
    border-radius: 1rem;
  }

  .metric-icon {
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .metric-icon svg {
    width: 1.25rem;
    height: 1.25rem;
  }

  .metric-icon.blue {
    background: linear-gradient(135deg, rgba(59, 130, 246, 0.2), rgba(59, 130, 246, 0.1));
    color: var(--phantom-ear-accent);
  }

  .metric-icon.purple {
    background: linear-gradient(135deg, rgba(139, 92, 246, 0.2), rgba(139, 92, 246, 0.1));
    color: var(--phantom-ear-purple);
  }

  .metric-content {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .metric-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--phantom-ear-text);
    font-variant-numeric: tabular-nums;
    line-height: 1.2;
  }

  .metric-label {
    font-size: 0.6875rem;
    font-weight: 500;
    color: var(--phantom-ear-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Secondary Metrics */
  .secondary-metrics {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    justify-content: center;
  }

  .metric-pill {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.875rem;
    border-radius: 9999px;
    font-size: 0.8125rem;
  }

  .pill-value {
    font-weight: 600;
    color: var(--phantom-ear-text);
    font-variant-numeric: tabular-nums;
  }

  .pill-label {
    color: var(--phantom-ear-text-muted);
    font-size: 0.75rem;
  }

  /* Activity Chart */
  .activity-chart {
    padding: 1rem;
    border-radius: 1rem;
  }

  .chart-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.75rem;
  }

  .chart-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--phantom-ear-text);
  }

  .chart-subtitle {
    font-size: 0.6875rem;
    color: var(--phantom-ear-text-muted);
  }

  .chart-bars {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    height: 4rem;
    gap: 0.25rem;
  }

  .bar-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    flex: 1;
    height: 100%;
  }

  .bar {
    width: 100%;
    max-width: 1.5rem;
    background: var(--phantom-ear-border);
    border-radius: 0.25rem 0.25rem 0 0;
    transition: height 0.5s ease, background 0.3s ease;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 0.25rem;
    margin-top: auto;
  }

  .bar.active {
    background: linear-gradient(180deg, var(--phantom-ear-accent), var(--phantom-ear-purple));
  }

  .bar.today {
    box-shadow: 0 0 8px rgba(59, 130, 246, 0.4);
  }

  .bar-count {
    font-size: 0.5625rem;
    font-weight: 600;
    color: white;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .bar-label {
    font-size: 0.5625rem;
    color: var(--phantom-ear-text-muted);
    margin-top: 0.375rem;
    font-weight: 500;
  }

  .bar-label.today {
    color: var(--phantom-ear-accent);
    font-weight: 600;
  }

  /* Responsive */
  @media (max-width: 360px) {
    .primary-metrics {
      grid-template-columns: 1fr;
    }
  }
</style>
