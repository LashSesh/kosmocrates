//! Minimal OpenAI-compatible LLM client.
//!
//! Works with any provider that implements the `/v1/chat/completions` endpoint:
//! Cerebras, OpenAI, Groq, Together AI, Fireworks, Ollama, LM Studio, etc.

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: u32,
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
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

pub struct LlmClient {
    pub base_url: String,
    pub model: String,
    api_key: String,
    client: reqwest::blocking::Client,
}

impl LlmClient {
    pub fn from_env() -> Self {
        let base_url = std::env::var("PSE_LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.cerebras.ai/v1".to_string());
        let model = std::env::var("PSE_LLM_MODEL").unwrap_or_else(|_| "llama3.1-8b".to_string());
        let api_key = std::env::var("PSE_LLM_API_KEY").unwrap_or_default();

        Self {
            base_url,
            model,
            api_key,
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn complete(&self, prompt: &str) -> Result<String, String> {
        self.complete_with_context(prompt, "")
    }

    /// Same as `complete` but prepends `pse_context` to the system prompt.
    /// Pass an empty string for the baseline (no-context) condition.
    pub fn complete_with_context(
        &self,
        prompt: &str,
        pse_context: &str,
    ) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let system_base = "You are a precise, analytical assistant. \
                           Answer in 4-8 dense, information-rich paragraphs.";
        let system_owned;
        let system = if pse_context.is_empty() {
            system_base
        } else {
            system_owned = format!("{system_base}\n\n{pse_context}");
            &system_owned
        };

        let body = ChatRequest {
            model: &self.model,
            messages: vec![
                Message {
                    role: "system",
                    content: system,
                },
                Message {
                    role: "user",
                    content: prompt,
                },
            ],
            max_tokens: 512,
            temperature: 0.3,
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| format!("HTTP error: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            return Err(format!("API returned {status}: {body}"));
        }

        let parsed: ChatResponse = resp.json().map_err(|e| format!("JSON parse error: {e}"))?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| "API returned empty choices".to_string())
    }
}
