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
}
