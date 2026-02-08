import { invoke } from '@tauri-apps/api/core';
import type { MeetingListItem, MeetingWithTranscript, TranscriptSegment, SearchResult, SemanticSearchResult } from '$lib/types';

function createMeetingsStore() {
  let meetings = $state<MeetingListItem[]>([]);
  let activeMeetingId = $state<string | null>(null);
  let activeTranscript = $state<TranscriptSegment[]>([]);
  let searchResults = $state<SearchResult[]>([]);
  let searchQuery = $state('');

  async function loadMeetings() {
    try {
      meetings = await invoke<MeetingListItem[]>('list_meetings');
    } catch (e) {
      console.error('Failed to load meetings:', e);
    }
  }

  async function selectMeeting(id: string) {
    activeMeetingId = id;
    try {
      const meeting = await invoke<MeetingWithTranscript>('get_meeting', { id });
      activeTranscript = meeting.segments;
    } catch (e) {
      console.error('Failed to load meeting:', e);
      activeTranscript = [];
    }
  }

  function setActive(id: string | null) {
    activeMeetingId = id;
    if (!id) {
      activeTranscript = [];
    }
  }

  function setActiveTranscript(segments: TranscriptSegment[]) {
    activeTranscript = segments;
  }

  function addLocalSegment(segment: TranscriptSegment) {
    activeTranscript = [...activeTranscript, segment];
  }

  async function renameMeeting(id: string, newTitle: string) {
    const title = newTitle.trim();
    if (!title) return;
    try {
      await invoke('rename_meeting', { id, title });
      meetings = meetings.map(m =>
        m.id === id ? { ...m, title } : m
      );
    } catch (e) {
      console.error('Failed to rename meeting:', e);
    }
  }

  async function togglePin(id: string) {
    try {
      await invoke('toggle_pin_meeting', { id });
      meetings = meetings.map(m =>
        m.id === id ? { ...m, pinned: !m.pinned } : m
      );
    } catch (e) {
      console.error('Failed to toggle pin:', e);
    }
  }

  async function deleteMeeting(id: string) {
    try {
      await invoke('delete_meeting', { id });
      meetings = meetings.filter(m => m.id !== id);
      if (activeMeetingId === id) {
        activeMeetingId = null;
        activeTranscript = [];
      }
    } catch (e) {
      console.error('Failed to delete meeting:', e);
    }
  }

  async function searchMeetings(query: string) {
    searchQuery = query;
    if (!query.trim()) {
      searchResults = [];
      return;
    }
    try {
      searchResults = await invoke<SearchResult[]>('search_meetings', { query });
    } catch (e) {
      console.error('Failed to search meetings:', e);
      searchResults = [];
    }
  }

  async function semanticSearch(query: string, meetingId?: string, limit?: number): Promise<SemanticSearchResult[]> {
    try {
      return await invoke<SemanticSearchResult[]>('semantic_search', {
        query,
        meetingId: meetingId || null,
        limit: limit || 10,
      });
    } catch (e) {
      console.error('Semantic search failed:', e);
      return [];
    }
  }

  async function exportMeeting(id: string, format: string = 'markdown'): Promise<string> {
    return invoke<string>('export_meeting', { id, format });
  }

  function getPinnedMeetings(): MeetingListItem[] {
    return meetings.filter(m => m.pinned);
  }

  function getRecentMeetings(): MeetingListItem[] {
    return meetings.filter(m => !m.pinned);
  }

  return {
    get meetings() { return meetings; },
    get activeMeetingId() { return activeMeetingId; },
    get activeTranscript() { return activeTranscript; },
    get searchResults() { return searchResults; },
    get searchQuery() { return searchQuery; },
    loadMeetings,
    selectMeeting,
    setActive,
    setActiveTranscript,
    addLocalSegment,
    renameMeeting,
    togglePin,
    deleteMeeting,
    searchMeetings,
    semanticSearch,
    exportMeeting,
    getPinnedMeetings,
    getRecentMeetings,
  };
}

export const meetingsStore = createMeetingsStore();
