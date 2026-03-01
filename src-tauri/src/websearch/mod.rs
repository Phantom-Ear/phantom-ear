// Web Search module
// Provides web search capability using DuckDuckGo API

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

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

        let response = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Search failed with status: {}", response.status()));
        }

        let html = response.text().await?;
        self.parse_results(&html, max_results)
    }

    /// Parse DuckDuckGo HTML results - unwraps redirect URLs
    fn parse_results(&self, html: &str, max_results: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        let result_regex = regex::Regex::new(
            r#"<a rel="nofollow" class="result__a" href="([^"]+)"[^>]*>([^<]+)</a>"#,
        )
        .unwrap();

        for cap in result_regex.captures_iter(html) {
            if results.len() >= max_results {
                break;
            }

            let mut url = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let title = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Attempt to unwrap duckduckgo redirect URLs
            if url.contains("duckduckgo.com/l/?uddg=") {
                if let Some(start_idx) = url.find("uddg=") {
                    let encoded_url = &url[start_idx + 5..];
                    if let Some(end_idx) = encoded_url.find('&') {
                        url = urlencoding::decode(&encoded_url[..end_idx])
                            .map(|cow| cow.into_owned())
                            .unwrap_or_else(|_| url);
                    } else {
                        url = urlencoding::decode(encoded_url)
                            .map(|cow| cow.into_owned())
                            .unwrap_or_else(|_| url);
                    }
                }
            }

            // Skip invalid or internal URLs
            if url.contains("duckduckgo.com") || url.is_empty() {
                continue;
            }

            results.push(SearchResult {
                title: title.to_string(),
                url,
                snippet: String::new(), // You can also extract snippets if needed
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duckduckgo_redirect() {
        let client = WebSearchClient::new();
        let html = r#"
            <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.speedtest.net%2F&amp;rut=446f6d">Speedtest by Ookla</a>
        "#;

        let results = client.parse_results(html, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Speedtest by Ookla");
        assert_eq!(results[0].url, "https://www.speedtest.net/");
    }
}
