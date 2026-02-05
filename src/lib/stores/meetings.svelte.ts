import type { Meeting, TranscriptSegment } from '$lib/types';

const STORAGE_KEY = 'sidecar-meetings';

function generateId(): string {
  return `meeting-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

function formatMeetingTitle(date: Date): string {
  const day = date.toLocaleDateString('en-US', { weekday: 'short' });
  const dateStr = date.toLocaleDateString('en-GB', {
    day: '2-digit',
    month: '2-digit',
    year: '2-digit'
  }).replace(/\//g, '/');
  const time = date.toLocaleTimeString('en-US', {
    hour: 'numeric',
    minute: '2-digit',
    hour12: true
  });
  return `${day} ${dateStr} · ${time}`;
}

function loadMeetings(): Meeting[] {
  if (typeof window === 'undefined') return [];
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (e) {
    console.error('Failed to load meetings:', e);
  }
  return [];
}

function saveMeetings(meetings: Meeting[]) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(meetings));
  } catch (e) {
    console.error('Failed to save meetings:', e);
  }
}

function createMeetingsStore() {
  let meetings = $state<Meeting[]>(loadMeetings());
  let activeMeetingId = $state<string | null>(null);

  function createMeeting(): Meeting {
    const meeting: Meeting = {
      id: generateId(),
      title: formatMeetingTitle(new Date()),
      createdAt: new Date().toISOString(),
      pinned: false,
      transcript: [],
    };
    meetings = [meeting, ...meetings];
    activeMeetingId = meeting.id;
    saveMeetings(meetings);
    return meeting;
  }

  function renameMeeting(id: string, newTitle: string) {
    meetings = meetings.map(m =>
      m.id === id ? { ...m, title: newTitle.trim() || m.title } : m
    );
    saveMeetings(meetings);
  }

  function togglePin(id: string) {
    meetings = meetings.map(m =>
      m.id === id ? { ...m, pinned: !m.pinned } : m
    );
    saveMeetings(meetings);
  }

  function deleteMeeting(id: string) {
    meetings = meetings.filter(m => m.id !== id);
    if (activeMeetingId === id) {
      activeMeetingId = null;
    }
    saveMeetings(meetings);
  }

  function setActive(id: string | null) {
    activeMeetingId = id;
  }

  function addTranscriptSegment(segment: TranscriptSegment) {
    if (!activeMeetingId) return;
    meetings = meetings.map(m =>
      m.id === activeMeetingId
        ? { ...m, transcript: [...m.transcript, segment] }
        : m
    );
    saveMeetings(meetings);
  }

  function setTranscript(transcript: TranscriptSegment[]) {
    if (!activeMeetingId) return;
    meetings = meetings.map(m =>
      m.id === activeMeetingId
        ? { ...m, transcript }
        : m
    );
    saveMeetings(meetings);
  }

  function getActiveMeeting(): Meeting | undefined {
    return meetings.find(m => m.id === activeMeetingId);
  }

  function getPinnedMeetings(): Meeting[] {
    return meetings.filter(m => m.pinned);
  }

  function getRecentMeetings(): Meeting[] {
    return meetings.filter(m => !m.pinned);
  }

  return {
    get meetings() { return meetings; },
    get activeMeetingId() { return activeMeetingId; },
    createMeeting,
    renameMeeting,
    togglePin,
    deleteMeeting,
    setActive,
    addTranscriptSegment,
    setTranscript,
    getActiveMeeting,
    getPinnedMeetings,
    getRecentMeetings,
  };
}

export const meetingsStore = createMeetingsStore();
