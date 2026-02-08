<script lang="ts">
  import type { SemanticSearchResult } from '$lib/types';

  let {
    result,
    onSelect,
  }: {
    result: SemanticSearchResult;
    onSelect: (meetingId: string) => void;
  } = $props();

  function truncate(text: string, maxLen: number): string {
    if (text.length <= maxLen) return text;
    return text.slice(0, maxLen) + '...';
  }

  const relevance = Math.round(result.score * 100);
</script>

<button
  onclick={() => onSelect(result.meeting_id)}
  class="w-full text-left p-3 rounded-lg bg-sidecar-surface/50 border border-sidecar-border hover:border-sidecar-accent/40 transition-colors group"
>
  <div class="flex items-center justify-between mb-1">
    <span class="text-xs font-medium text-sidecar-accent truncate max-w-[70%]">
      {result.meeting_title}
    </span>
    <span class="text-[10px] text-sidecar-text-muted font-mono">{result.time_label}</span>
  </div>
  <p class="text-sm text-sidecar-text leading-relaxed">
    {truncate(result.text, 120)}
  </p>
  <div class="mt-2 flex items-center gap-2">
    <div class="flex-1 h-1 rounded-full bg-sidecar-border overflow-hidden">
      <div
        class="h-full bg-sidecar-accent/60 rounded-full"
        style="width: {relevance}%"
      ></div>
    </div>
    <span class="text-[10px] text-sidecar-text-muted">{relevance}%</span>
  </div>
</button>
