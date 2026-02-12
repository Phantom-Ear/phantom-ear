<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { TranscriptSegment, Speaker } from "$lib/types";

  let {
    segment,
    speakers = [],
    onUpdate,
    onDelete,
    onSpeakersChange,
  }: {
    segment: TranscriptSegment;
    speakers?: Speaker[];
    onUpdate: (segment: TranscriptSegment) => void;
    onDelete: (segmentId: string) => void;
    onSpeakersChange?: () => void;
  } = $props();

  let isEditing = $state(false);
  let editText = $state(segment.text);
  let isSaving = $state(false);
  let showDeleteConfirm = $state(false);
  let showSpeakerDropdown = $state(false);
  let newSpeakerName = $state("");
  let isCreatingSpeaker = $state(false);

  // Get current speaker
  const currentSpeaker = $derived(
    speakers.find(s => s.id === segment.speaker_id)
  );

  // Predefined colors for new speakers
  const speakerColors = [
    "#3b82f6", // blue
    "#8b5cf6", // purple
    "#ec4899", // pink
    "#ef4444", // red
    "#f97316", // orange
    "#eab308", // yellow
    "#22c55e", // green
    "#14b8a6", // teal
  ];

  function getNextColor(): string {
    const usedColors = speakers.map(s => s.color);
    return speakerColors.find(c => !usedColors.includes(c)) || speakerColors[0];
  }

  async function saveEdit() {
    if (editText.trim() === segment.text || !editText.trim()) {
      isEditing = false;
      editText = segment.text;
      return;
    }

    isSaving = true;
    try {
      await invoke("update_segment", {
        segmentId: segment.id,
        text: editText.trim(),
        speakerId: segment.speaker_id,
      });
      onUpdate({ ...segment, text: editText.trim() });
    } catch (e) {
      console.error("Failed to save segment:", e);
      editText = segment.text;
    }
    isSaving = false;
    isEditing = false;
  }

  async function assignSpeaker(speakerId: string | null) {
    try {
      await invoke("update_segment", {
        segmentId: segment.id,
        text: null,
        speakerId: speakerId,
      });
      onUpdate({ ...segment, speaker_id: speakerId });
    } catch (e) {
      console.error("Failed to assign speaker:", e);
    }
    showSpeakerDropdown = false;
  }

  async function createSpeaker() {
    if (!newSpeakerName.trim()) return;

    isCreatingSpeaker = true;
    try {
      const id = await invoke<string>("create_speaker", {
        name: newSpeakerName.trim(),
        color: getNextColor(),
      });
      // Assign the new speaker to this segment
      await assignSpeaker(id);
      newSpeakerName = "";
      onSpeakersChange?.();
    } catch (e) {
      console.error("Failed to create speaker:", e);
    }
    isCreatingSpeaker = false;
  }

  async function confirmDelete() {
    try {
      await invoke("delete_segment", { segmentId: segment.id });
      onDelete(segment.id);
    } catch (e) {
      console.error("Failed to delete segment:", e);
    }
    showDeleteConfirm = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      saveEdit();
    } else if (e.key === "Escape") {
      isEditing = false;
      editText = segment.text;
    }
  }

  function handleNewSpeakerKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      createSpeaker();
    } else if (e.key === "Escape") {
      showSpeakerDropdown = false;
      newSpeakerName = "";
    }
  }

  function startEditing() {
    editText = segment.text;
    isEditing = true;
  }
</script>

<div class="segment-row" data-segment-id={segment.id}>
  <span class="time-label">{segment.time}</span>

  <!-- Speaker Badge -->
  <div class="speaker-area">
    <button
      class="speaker-badge"
      style={currentSpeaker ? `--speaker-color: ${currentSpeaker.color}` : ''}
      onclick={() => showSpeakerDropdown = !showSpeakerDropdown}
      title={currentSpeaker ? `Speaker: ${currentSpeaker.name}` : "Assign speaker"}
    >
      {#if currentSpeaker}
        <span class="speaker-dot" style="background: {currentSpeaker.color}"></span>
        <span class="speaker-name">{currentSpeaker.name}</span>
      {:else}
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
        </svg>
      {/if}
    </button>

    {#if showSpeakerDropdown}
      <div class="speaker-dropdown">
        <div class="speaker-list">
          {#if currentSpeaker}
            <button
              class="speaker-option"
              onclick={() => assignSpeaker(null)}
            >
              <span class="text-phantom-ear-text-muted">Remove speaker</span>
            </button>
          {/if}
          {#each speakers as speaker}
            <button
              class="speaker-option {speaker.id === segment.speaker_id ? 'active' : ''}"
              onclick={() => assignSpeaker(speaker.id)}
            >
              <span class="speaker-dot" style="background: {speaker.color}"></span>
              <span>{speaker.name}</span>
              {#if speaker.id === segment.speaker_id}
                <svg class="w-3 h-3 ml-auto text-phantom-ear-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </button>
          {/each}
        </div>
        <div class="new-speaker">
          <input
            type="text"
            placeholder="New speaker name..."
            bind:value={newSpeakerName}
            onkeydown={handleNewSpeakerKeydown}
            class="new-speaker-input"
          />
          <button
            class="add-speaker-btn"
            onclick={createSpeaker}
            disabled={!newSpeakerName.trim() || isCreatingSpeaker}
          >
            {#if isCreatingSpeaker}
              <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
              </svg>
            {:else}
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
              </svg>
            {/if}
          </button>
        </div>
      </div>
    {/if}
  </div>

  <div class="segment-content">
    {#if isEditing}
      <textarea
        bind:value={editText}
        onblur={saveEdit}
        onkeydown={handleKeydown}
        class="edit-textarea"
        rows="2"
        disabled={isSaving}
      ></textarea>
    {:else}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <p
        class="segment-text"
        onclick={startEditing}
        title="Click to edit"
      >
        {segment.text}
      </p>
    {/if}
  </div>

  <div class="segment-actions">
    {#if !isEditing}
      <button
        class="action-btn edit-btn"
        onclick={startEditing}
        title="Edit segment"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
        </svg>
      </button>
      <button
        class="action-btn delete-btn"
        onclick={() => showDeleteConfirm = true}
        title="Delete segment"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
      </button>
    {/if}
  </div>

  {#if showDeleteConfirm}
    <div class="delete-confirm">
      <p>Delete this segment?</p>
      <div class="confirm-buttons">
        <button class="cancel-btn" onclick={() => showDeleteConfirm = false}>Cancel</button>
        <button class="confirm-btn" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  {/if}
</div>

<!-- Click outside to close speaker dropdown -->
{#if showSpeakerDropdown}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-5"
    onclick={() => showSpeakerDropdown = false}
  ></div>
{/if}

<style>
  .segment-row {
    display: flex;
    gap: 0.75rem;
    padding: 0.5rem;
    border-radius: 0.5rem;
    transition: background 0.15s ease;
    position: relative;
    align-items: flex-start;
  }

  .segment-row:hover {
    background: rgba(47, 47, 47, 0.5);
  }

  .segment-row:hover .segment-actions {
    opacity: 1;
  }

  .time-label {
    font-size: 0.75rem;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    color: var(--phantom-ear-accent);
    flex-shrink: 0;
    padding-top: 0.125rem;
  }

  .speaker-area {
    position: relative;
    flex-shrink: 0;
  }

  .speaker-badge {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.625rem;
    font-weight: 500;
    background: var(--speaker-color, var(--phantom-ear-surface));
    background: color-mix(in srgb, var(--speaker-color, var(--phantom-ear-text-muted)) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--speaker-color, var(--phantom-ear-border)) 30%, transparent);
    color: var(--speaker-color, var(--phantom-ear-text-muted));
    cursor: pointer;
    transition: all 0.15s ease;
    min-width: 1.5rem;
    justify-content: center;
  }

  .speaker-badge:hover {
    background: color-mix(in srgb, var(--speaker-color, var(--phantom-ear-text-muted)) 25%, transparent);
    border-color: color-mix(in srgb, var(--speaker-color, var(--phantom-ear-border)) 50%, transparent);
  }

  .speaker-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .speaker-name {
    max-width: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .speaker-dropdown {
    position: absolute;
    left: 0;
    top: 100%;
    margin-top: 0.25rem;
    z-index: 20;
    background: var(--phantom-ear-surface);
    border: 1px solid var(--phantom-ear-border);
    border-radius: 0.5rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    min-width: 140px;
    overflow: hidden;
  }

  .speaker-list {
    max-height: 150px;
    overflow-y: auto;
  }

  .speaker-option {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    color: var(--phantom-ear-text);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 0.15s ease;
    text-align: left;
  }

  .speaker-option:hover {
    background: var(--phantom-ear-surface-hover);
  }

  .speaker-option.active {
    background: var(--phantom-ear-accent)/10;
  }

  .new-speaker {
    display: flex;
    gap: 0.25rem;
    padding: 0.5rem;
    border-top: 1px solid var(--phantom-ear-border);
  }

  .new-speaker-input {
    flex: 1;
    padding: 0.375rem 0.5rem;
    font-size: 0.75rem;
    background: var(--phantom-ear-bg);
    border: 1px solid var(--phantom-ear-border);
    border-radius: 0.25rem;
    color: var(--phantom-ear-text);
    outline: none;
  }

  .new-speaker-input:focus {
    border-color: var(--phantom-ear-accent);
  }

  .add-speaker-btn {
    padding: 0.375rem;
    background: var(--phantom-ear-accent);
    border: none;
    border-radius: 0.25rem;
    color: white;
    cursor: pointer;
    transition: background 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .add-speaker-btn:hover:not(:disabled) {
    background: var(--phantom-ear-accent-hover);
  }

  .add-speaker-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .segment-content {
    flex: 1;
    min-width: 0;
  }

  .segment-text {
    margin: 0;
    font-size: 0.875rem;
    line-height: 1.5;
    color: var(--phantom-ear-text);
    cursor: text;
    padding: 0.125rem;
    border-radius: 0.25rem;
    transition: background 0.15s ease;
  }

  .segment-text:hover {
    background: rgba(59, 130, 246, 0.1);
  }

  .edit-textarea {
    width: 100%;
    font-size: 0.875rem;
    line-height: 1.5;
    color: var(--phantom-ear-text);
    background: var(--phantom-ear-bg);
    border: 1px solid var(--phantom-ear-accent);
    border-radius: 0.375rem;
    padding: 0.375rem 0.5rem;
    resize: vertical;
    font-family: inherit;
  }

  .edit-textarea:focus {
    outline: none;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
  }

  .segment-actions {
    display: flex;
    gap: 0.25rem;
    opacity: 0;
    transition: opacity 0.15s ease;
    flex-shrink: 0;
  }

  .action-btn {
    padding: 0.375rem;
    border-radius: 0.375rem;
    border: none;
    background: transparent;
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--phantom-ear-text-muted);
  }

  .edit-btn:hover {
    background: rgba(59, 130, 246, 0.1);
    color: var(--phantom-ear-accent);
  }

  .delete-btn:hover {
    background: rgba(239, 68, 68, 0.1);
    color: var(--phantom-ear-danger);
  }

  .delete-confirm {
    position: absolute;
    right: 0;
    top: 100%;
    z-index: 10;
    background: var(--phantom-ear-surface);
    border: 1px solid var(--phantom-ear-border);
    border-radius: 0.5rem;
    padding: 0.75rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    min-width: 10rem;
  }

  .delete-confirm p {
    margin: 0 0 0.5rem;
    font-size: 0.8125rem;
    color: var(--phantom-ear-text);
  }

  .confirm-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .cancel-btn,
  .confirm-btn {
    flex: 1;
    padding: 0.375rem 0.75rem;
    border-radius: 0.375rem;
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border: none;
  }

  .cancel-btn {
    background: var(--phantom-ear-bg);
    color: var(--phantom-ear-text-muted);
  }

  .cancel-btn:hover {
    background: var(--phantom-ear-surface-hover);
    color: var(--phantom-ear-text);
  }

  .confirm-btn {
    background: var(--phantom-ear-danger);
    color: white;
  }

  .confirm-btn:hover {
    background: #dc2626;
  }
</style>
