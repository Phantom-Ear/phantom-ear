// Shared TypeScript interfaces

export interface TranscriptSegment {
  id: string;
  time: string;
  text: string;
  timestamp_ms: number;
  speaker_id?: string | null;
}

export interface Meeting {
  id: string;
  title: string;
  createdAt: string;
  pinned: boolean;
  transcript: TranscriptSegment[];
}

export interface MeetingListItem {
  id: string;
  title: string;
  created_at: string;
  pinned: boolean;
  segment_count: number;
  duration_ms: number;
  first_segment_text?: string;
  tags?: string | null;
}

export interface MeetingWithTranscript {
  id: string;
  title: string;
  created_at: string;
  ended_at: string | null;
  pinned: boolean;
  duration_ms: number;
  segments: TranscriptSegment[];
}

export interface SearchResult {
  meeting_id: string;
  meeting_title: string;
  segment_id: string;
  text: string;
  time_label: string;
  snippet: string;
}

export interface Settings {
  llm_provider: string;
  openai_api_key: string | null;
  ollama_url: string | null;
  ollama_model: string | null;
  auto_detect_meetings: boolean;
  show_system_notifications: boolean;
  onboarding_completed: boolean;
  whisper_model: string;
  language: string;
  asr_backend: string;
  audio_device: string | null;
  enhance_transcripts: boolean;
  detect_questions: boolean;
}

export interface AudioDeviceInfo {
  name: string;
  is_default: boolean;
}

export interface Speaker {
  id: string;
  name: string;
  color: string;
  created_at: string;
}

export interface ModelInfo {
  name: string;
  size_mb: number;
  downloaded: boolean;
  description: string;
  backend: string;
  recommended: boolean;
}

export interface BackendInfo {
  backend_type: string;
  name: string;
  description: string;
  supported_languages: string[];
}

export interface ModelStatus {
  whisper_downloaded: boolean;
  whisper_model: string;
  whisper_size_mb: number;
  models_dir: string;
}

export interface TranscriptionEvent {
  id: string;
  text: string;
  start_ms: number;
  end_ms: number;
  is_partial: boolean;
}

export interface Summary {
  title?: string;
  overview: string;
  action_items: string[];
  key_points: string[];
}

export interface SemanticSearchResult {
  meeting_id: string;
  meeting_title: string;
  segment_id: string;
  text: string;
  time_label: string;
  score: number;
}

export interface EmbeddingStatus {
  model_loaded: boolean;
  embedded_count: number;
  total_segments: number;
}

export interface ConversationItem {
  question: string;
  answer: string;
  created_at: string;
}
export interface UserNote {
  id: string;
  text: string;
  createdAt: number;
  mentionCount: number;
  lastMentionedAt: number | null;
}

export interface NoteBriefing {
  id: string;
  noteId: string;
  noteText: string;
  briefing: string;
  timestamp: number;
}

export interface NoteCheckResult {
  note_id: string;
  note_text: string;
  mentioned: boolean;
  briefing: string | null;
}

export type View = 'home' | 'phomy' | 'settings';
export type Theme = 'light' | 'dark';
