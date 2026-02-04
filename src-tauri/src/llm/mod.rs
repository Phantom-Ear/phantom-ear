// LLM module
// Interface with OpenAI and Ollama for AI features

use anyhow::Result;
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

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
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
        let (url, model, headers) = match &self.provider {
            LlmProvider::OpenAI { api_key } => (
                "https://api.openai.com/v1/chat/completions".to_string(),
                "gpt-4o-mini".to_string(),
                vec![("Authorization", format!("Bearer {}", api_key))],
            ),
            LlmProvider::Ollama { url, model } => (
                format!("{}/api/chat", url),
                model.clone(),
                vec![],
            ),
        };

        let request = ChatRequest {
            model,
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

        let mut req = self.client.post(&url).json(&request);
        for (key, value) in headers {
            req = req.header(key, value);
        }

        let response: ChatResponse = req.send().await?.json().await?;

        Ok(response.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
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
