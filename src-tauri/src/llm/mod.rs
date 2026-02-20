// LLM module
// Interface with OpenAI and Ollama for AI features

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAI { api_key: String },
    Ollama { url: String, model: String },
}

pub struct LlmClient {
    provider: LlmProvider,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

// OpenAI request format
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

// OpenAI response format
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: String,
}

// Ollama request format
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

// Ollama response format
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

impl LlmClient {
    pub fn new(provider: LlmProvider) -> Self {
        Self {
            provider,
            client: reqwest::Client::new(),
        }
    }

    /// Send a completion request
    pub async fn complete(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        match &self.provider {
            LlmProvider::OpenAI { api_key } => {
                self.complete_openai(api_key, system_prompt, user_prompt).await
            }
            LlmProvider::Ollama { url, model } => {
                self.complete_ollama(url, model, system_prompt, user_prompt).await
            }
        }
    }

    async fn complete_openai(&self, api_key: &str, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let request = OpenAIRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            temperature: 0.7,
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error ({}): {}", status, error_text));
        }

        let response: OpenAIResponse = response.json().await?;
        Ok(response.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    async fn complete_ollama(&self, url: &str, model: &str, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let request = OllamaRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            stream: false,
        };

        let api_url = format!("{}/api/chat", url);
        let response = self.client
            .post(&api_url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama API error ({}): {}. Is Ollama running?", status, error_text));
        }

        let response: OllamaResponse = response.json().await?;
        Ok(response.message.content)
    }

    /// Generate a meeting summary
    pub async fn summarize(&self, transcript: &str) -> Result<String> {
        let system = "You are a helpful assistant that creates concise meeting summaries. \
                      Extract key points, action items, and important decisions.";
        let user = format!("Please summarize this meeting transcript:\n\n{}", transcript);
        self.complete(system, &user).await
    }

    /// Answer a question about the meeting
    pub async fn answer_question(&self, context: &str, question: &str) -> Result<String> {
        let system = "You are a helpful assistant answering questions about a meeting. \
                      Use only the provided context to answer. If the answer isn't in the context, say so.";
        let user = format!("Context:\n{}\n\nQuestion: {}", context, question);
        self.complete(system, &user).await
    }

    /// Generate a meeting title from transcript
    pub async fn generate_title(&self, transcript: &str) -> Result<String> {
        let system = "You are a helpful assistant that creates short, descriptive titles for meeting transcripts. \
                      Create a concise title (3-6 words) that captures the main topic or purpose of the meeting. \
                      Just return the title, nothing else.";
        let user = format!("Based on this transcript, create a short title:\n\n{}", transcript);
        self.complete(system, &user).await
    }

    /// Enhance a transcript segment with context from surrounding segments
    pub async fn enhance_segment(&self, prev_text: Option<&str>, current_text: &str, next_text: Option<&str>) -> Result<String> {
        let mut context = String::new();
        if let Some(prev) = prev_text {
            context.push_str(&format!("Previous segment: {}\n", prev));
        }
        context.push_str(&format!("Current segment: {}\n", current_text));
        if let Some(next) = next_text {
            context.push_str(&format!("Next segment: {}", next));
        }

        let system = "You are a transcription enhancer. Improve the clarity of the current transcript segment. \
                      Fix any misheard words, add proper punctuation, and make it more readable. \
                      Return ONLY the enhanced text, nothing else.";
        let user = format!("Enhance this transcript segment:\n\n{}", context);
        self.complete(system, &user).await
    }

    /// Enhance a batch of transcript segments together for better context
    pub async fn enhance_batch(&self, segments: &[String]) -> Result<Vec<String>> {
        if segments.is_empty() {
            return Ok(vec![]);
        }

        // Join all segments with context
        let context = segments.iter().enumerate()
            .map(|(i, s)| format!("Segment {}: {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");

        let system = "You are a transcription enhancer. Improve the clarity of the transcript segments. \
                      Fix any misheard words, add proper punctuation, make it more readable and well-structured. \
                      For each segment, provide the enhanced version. \
                      Return the enhanced segments as a JSON array of strings, nothing else.";
        let user = format!("Enhance these transcript segments as a JSON array:\n\n[\n{}\n]", context);
        
        let result = self.complete(system, &user).await?;
        
        // Parse the JSON array result
        let parsed: Vec<String> = serde_json::from_str(&result)
            .unwrap_or_else(|_| {
                // If parsing fails, try to extract segments from the text
                segments.iter().cloned().collect()
            });
        
        Ok(parsed)
    }

    /// Detect if text contains a question
    pub async fn detect_question(&self, text: &str) -> Result<bool> {
        let system = "You are a helpful assistant. Determine if the given text contains a question being asked. \
                      A question is a sentence that asks for information, clarification, or an answer. \
                      Return ONLY 'YES' if it contains a question, or 'NO' if it does not.";
        let user = format!("Does this contain a question?\n\n{}", text);
        let result = self.complete(system, &user).await?;
        Ok(result.trim().to_uppercase().starts_with("YES"))
    }

    /// Extract metadata from a meeting transcript
    pub async fn extract_metadata(&self, transcript: &str) -> Result<MeetingMetadata> {
        let system = "You are a helpful assistant that analyzes meeting transcripts. \
                      Extract structured metadata and return it as JSON. \
                      Return ONLY valid JSON with no additional text.";
        
        let json_example = r#"{"topics": ["topic1"], "action_items": [], "decisions": [], "participant_count_estimate": 3}"#;
        let user = format!(
            "Analyze this meeting transcript and extract metadata as JSON with this structure: {}\nTranscript:\n{}",
            json_example,
            transcript
        );
        
        let result = self.complete(system, &user).await?;
        
        // Parse JSON from result
        let json_str = result.trim();
        // Handle potential markdown code blocks
        let json_str = json_str.trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```");
        
        serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Failed to parse metadata JSON: {} - raw: {}", e, json_str))
    }
}

/// Metadata extracted from a meeting
#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingMetadata {
    pub topics: Vec<String>,
    pub action_items: Vec<String>,
    pub decisions: Vec<String>,
    #[serde(rename = "participant_count_estimate")]
    pub participant_count_estimate: i32,
}
