use anyhow::{bail, Result};

use crate::llm::{LlmClient, MockLlmClient, OpenAiClient};

#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAi,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub data_dir: String,
}

impl Config {
    pub fn new(provider: &str, model: &str, api_key: Option<&str>, data_dir: &str) -> Result<Self> {
        let provider = match provider {
            "openai" => LlmProvider::OpenAi,
            other => bail!("Unsupported LLM provider: {other}. Supported: openai"),
        };

        // Resolve API key: CLI arg → env var
        let api_key = api_key.map(String::from).or_else(|| match &provider {
            LlmProvider::OpenAi => std::env::var("OPENAI_API_KEY").ok(),
        });

        Ok(Config {
            provider,
            model: model.to_string(),
            api_key,
            data_dir: data_dir.to_string(),
        })
    }

    pub fn build_llm_client(&self) -> Result<Box<dyn LlmClient>> {
        // If GRAPHRAG_MOCK_LLM is set, use mock client (for testing)
        if std::env::var("GRAPHRAG_MOCK_LLM").is_ok() {
            return Ok(Box::new(MockLlmClient));
        }

        match &self.provider {
            LlmProvider::OpenAi => {
                let api_key = self
                    .api_key
                    .as_ref()
                    .ok_or_else(|| {
                        anyhow::anyhow!("OpenAI API key required. Set --api-key or OPENAI_API_KEY")
                    })?
                    .clone();
                Ok(Box::new(OpenAiClient::new(api_key, &self.model)))
            }
        }
    }
}
