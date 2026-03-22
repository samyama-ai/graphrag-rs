use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Trait for LLM chat completions.
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, system: &str, user: &str) -> Result<String>;
}

// ── OpenAI Implementation ──────────────────────────────────────────

pub struct OpenAiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    api_base: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    temperature: f32,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

impl OpenAiClient {
    pub fn new(api_key: String, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.to_string(),
            api_base: "https://api.openai.com/v1".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, system: &str, user: &str) -> Result<String> {
        let request = ChatRequest {
            model: &self.model,
            messages: vec![
                Message {
                    role: "system",
                    content: system,
                },
                Message {
                    role: "user",
                    content: user,
                },
            ],
            temperature: 0.0,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_base))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API error {status}: {body}");
        }

        let resp: ChatResponse = response.json().await?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("Empty response from OpenAI"))
    }
}

// ── Mock Implementation (for testing) ──────────────────────────────

pub struct MockLlmClient;

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn chat(&self, _system: &str, _user: &str) -> Result<String> {
        Ok(r#"{
  "entities": [
    {"name": "Rust", "type": "ProgrammingLanguage", "description": "A systems programming language"},
    {"name": "GraphRAG", "type": "Tool", "description": "Knowledge graph builder with LLM extraction"}
  ],
  "relationships": [
    {"source": "GraphRAG", "target": "Rust", "type": "WRITTEN_IN", "description": "GraphRAG is implemented in Rust"}
  ]
}"#
        .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_returns_valid_json() {
        let client = MockLlmClient;
        let result = client.chat("system", "user").await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["entities"].is_array());
        assert!(parsed["relationships"].is_array());
    }
}
