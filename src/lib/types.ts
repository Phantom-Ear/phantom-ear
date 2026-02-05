// Shared TypeScript interfaces

export interface TranscriptSegment {
  id: string;
  time: string;
  text: string;
  timestamp_ms: number;
}

export interface Meeting {
  id: string;
  title: string;
  createdAt: string;
  pinned: boolean;
  transcript: TranscriptSegment[];
}

export interface Settings {
  llm_provider: string;
  openai_api_key: string | null;
  ollama_url: string | null;
  ollama_model: string | null;
  auto_detect_meetings: boolean;
  whisper_model: string;
  language: string;
  asr_backend: string;
}

export interface ModelInfo {
  name: string;
  size_mb: number;
  downloaded: boolean;
  description: string;
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
  overview: string;
  action_items: string[];
  key_points: string[];
}

export type View = 'home' | 'genie' | 'settings';
export type Theme = 'light' | 'dark';
