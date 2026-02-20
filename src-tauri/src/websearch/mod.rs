// Web Search module
// Provides web search capability using DuckDuckGo API

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use reqwest::Client;

pub struct WebSearchClient {
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

impl WebSearchClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Search the web using DuckDuckGo HTML (no API key required)
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self.client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Search failed with status: {}", response.status()));
        }

        let html = response.text().await?;
        self.parse_results(&html, max_results)
    }

    /// Parse DuckDuckGo HTML results - simplified version
    fn parse_results(&self, html: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        
        let result_regex = regex::Regex::new(r#"<a rel="nofollow" class="result__a" href="([^"]+)"[^>]*>([^<]+)</a>"#).unwrap();
        
        for cap in result_regex.captures_iter(html) {
            if results.len() >= max_results {
                break;
            }
            
            let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let title = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            
            // Skip DuckDuckGo internal URLs
            if url.contains("duckduckgo.com") || url.is_empty() {
                continue;
            }
            
            results.push(SearchResult {
                title: title.to_string(),
                url: url.to_string(),
                snippet: String::new(),
            });
        }

        Ok(results)
    }
}

impl Default for WebSearchClient {
    fn default() -> Self {
        Self::new()
    }
}
