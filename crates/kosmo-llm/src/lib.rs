//! Kosmocrates shared LLM transport.
//!
//! One place for the connection + sampling config, the provider-specific request
//! shapes (Anthropic Messages API and any OpenAI-compatible `/chat/completions`
//! endpoint), retry/backoff, response extraction, and a tolerant brace-balanced
//! JSON-object extractor. Task crates (`kosmo-intent-llm` for prose→`Wish`,
//! `kosmo-synthesizer-llm` for patches) supply the prompts and parse the result;
//! this crate carries the bytes.
//!
//! ## Contract
//!
//! [`LlmConfig::complete`] takes a `(system, user)` prompt pair and returns the
//! raw model text plus a token count — nothing more. It never parses, hashes, or
//! interprets the response; that is the caller's job, and the caller immediately
//! re-determinizes the result into a content-addressed artifact. This crate is
//! therefore the **single non-deterministic boundary** of the wish-to-system
//! machine (see [`docs/WISH_TO_SYSTEM.md`] §5); everything upstream and
//! downstream of it is deterministic and replayable.
//!
//! ## Constraints
//!
//! - **No floats in gate paths (CROSS-007).** A `temperature` float appears only
//!   inside the outbound request body — never in any gate path or content
//!   address. Confidence/score values cross back as integer percentages that
//!   callers convert to `Q16`.
//! - **Endpoints are caller-supplied.** The crate has no hard-coded network
//!   destination; the provider base URL comes from config or the environment, so
//!   it builds and tests with no external network dependency.
//!
//! [`docs/WISH_TO_SYSTEM.md`]: ../../../docs/WISH_TO_SYSTEM.md

use std::time::Duration;

use serde_json::json;

// ─── Provider + config ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlmProvider {
    /// Anthropic Messages API (`/v1/messages`, `x-api-key`).
    Anthropic,
    /// Any OpenAI-compatible `/chat/completions` endpoint (Cerebras, OpenAI, …).
    OpenAiCompatible,
}

/// Connection + sampling configuration for one LLM endpoint.
#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    /// Base URL with no trailing path, e.g. `https://api.anthropic.com`.
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub max_tokens: u32,
    /// Sampling temperature in thousandths (`0..=1000`); divided by 1000 only at
    /// serialization. Default `0` for determinism.
    pub temperature_milli: u32,
    pub timeout_secs: u64,
    /// Short human tag (`"claude"`, `"cerebras"`, `"openai"`).
    pub tag: String,
}

impl LlmConfig {
    /// Claude via the Anthropic Messages API. The default model is a neutral,
    /// overridable slug (set the real one via [`LlmConfig::with_model`] or
    /// `ANTHROPIC_MODEL`).
    pub fn claude(api_key: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::Anthropic,
            base_url: "https://api.anthropic.com".into(),
            model: "claude-sonnet-4-6".into(),
            api_key: api_key.into(),
            max_tokens: 4096,
            temperature_milli: 0,
            timeout_secs: 120,
            tag: "claude".into(),
        }
    }

    /// Cerebras — the OpenAI-compatible free-tier bridge.
    pub fn cerebras(api_key: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::OpenAiCompatible,
            base_url: "https://api.cerebras.ai/v1".into(),
            model: "llama-3.3-70b".into(),
            api_key: api_key.into(),
            max_tokens: 4096,
            temperature_milli: 0,
            timeout_secs: 120,
            tag: "cerebras".into(),
        }
    }

    /// Any other OpenAI-compatible endpoint (OpenAI, Groq, Together, Ollama …).
    pub fn openai_compatible(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            provider: LlmProvider::OpenAiCompatible,
            base_url: base_url.into(),
            model: model.into(),
            api_key: api_key.into(),
            max_tokens: 4096,
            temperature_milli: 0,
            timeout_secs: 120,
            tag: "openai".into(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = n;
        self
    }
    pub fn with_temperature_milli(mut self, m: u32) -> Self {
        self.temperature_milli = m.min(1000);
        self
    }
    pub fn with_timeout_secs(mut self, s: u64) -> Self {
        self.timeout_secs = s;
        self
    }

    /// `"claude:claude-sonnet-4-6"`, `"cerebras:llama-3.3-70b"`, …
    pub fn label(&self) -> String {
        format!("{}:{}", self.tag, self.model)
    }

    fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        match self.provider {
            LlmProvider::Anthropic => format!("{base}/v1/messages"),
            LlmProvider::OpenAiCompatible => format!("{base}/chat/completions"),
        }
    }

    fn request_body(&self, system: &str, user: &str) -> serde_json::Value {
        let temperature = self.temperature_milli as f64 / 1000.0;
        match self.provider {
            LlmProvider::Anthropic => json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "temperature": temperature,
                "system": system,
                "messages": [{ "role": "user", "content": user }],
            }),
            LlmProvider::OpenAiCompatible => json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "temperature": temperature,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user },
                ],
            }),
        }
    }

    fn apply_headers(
        &self,
        req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        match self.provider {
            LlmProvider::Anthropic => req
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01"),
            LlmProvider::OpenAiCompatible => {
                req.header("Authorization", format!("Bearer {}", self.api_key))
            }
        }
    }

    /// Pull `(assistant_text, output_tokens)` from a parsed response.
    fn extract_content(&self, val: &serde_json::Value) -> Result<(String, u32), LlmError> {
        match self.provider {
            LlmProvider::Anthropic => {
                let text = val["content"][0]["text"].as_str().ok_or_else(|| {
                    LlmError::permanent(format!(
                        "unexpected Anthropic response shape: {}",
                        truncate(&val.to_string(), 300)
                    ))
                })?;
                let tokens = val["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
                Ok((text.to_string(), tokens))
            }
            LlmProvider::OpenAiCompatible => {
                let text = val["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| {
                        LlmError::permanent(format!(
                            "unexpected OpenAI-compatible response shape: {}",
                            truncate(&val.to_string(), 300)
                        ))
                    })?;
                let tokens = val["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;
                Ok((text.to_string(), tokens))
            }
        }
    }

    /// Send one system+user turn and return `(assistant_text, output_tokens)`.
    ///
    /// Builds a client with the configured timeout, retries transient
    /// rate-limit / server errors (HTTP 429/529/5xx) with exponential backoff,
    /// and extracts the assistant text per provider shape. The only
    /// non-deterministic step in the substrate lives behind this call.
    pub fn complete(&self, system: &str, user: &str) -> Result<(String, u32), LlmError> {
        if self.api_key.trim().is_empty() {
            return Err(LlmError::permanent("no API key configured"));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| LlmError::permanent(format!("http client build failed: {e}")))?;
        let url = self.endpoint();
        let body = self.request_body(system, user);
        let retry_delays = [4u64, 8, 16];
        let mut attempt = 0usize;
        loop {
            let req = self.apply_headers(client.post(&url).json(&body));
            let resp = req
                .send()
                .map_err(|e| LlmError::transient(format!("http send failed: {e}")))?;
            let status = resp.status();
            let retryable =
                status.as_u16() == 429 || status.as_u16() == 529 || status.is_server_error();
            if retryable {
                if attempt < retry_delays.len() {
                    std::thread::sleep(Duration::from_secs(retry_delays[attempt]));
                    attempt += 1;
                    continue;
                }
                return Err(LlmError::transient(format!(
                    "endpoint unavailable after {} retries (HTTP {status})",
                    retry_delays.len()
                )));
            }
            if !status.is_success() {
                let text = resp.text().unwrap_or_default();
                return Err(LlmError::permanent(format!(
                    "HTTP {status}: {}",
                    truncate(&text, 300)
                )));
            }
            let val: serde_json::Value = resp
                .json()
                .map_err(|e| LlmError::permanent(format!("response was not JSON: {e}")))?;
            return self.extract_content(&val);
        }
    }
}

/// Detect an [`LlmConfig`] from environment variables.
///
/// Priority: explicit `KOSMO_LLM_*` (provider/base/model/key), then
/// `ANTHROPIC_API_KEY` (→ Claude, `ANTHROPIC_MODEL` override), then
/// `CEREBRAS_API_KEY` (→ Cerebras, `CEREBRAS_MODEL` override).
pub fn config_from_env() -> Result<LlmConfig, LlmError> {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());

    if let Some(key) = env("KOSMO_LLM_API_KEY") {
        let provider = env("KOSMO_LLM_PROVIDER").unwrap_or_default();
        let model = env("KOSMO_LLM_MODEL").unwrap_or_else(|| "llama-3.3-70b".into());
        let base = env("KOSMO_LLM_BASE_URL");
        return Ok(match provider.to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => {
                let mut c = LlmConfig::claude(key);
                if let Some(m) = env("KOSMO_LLM_MODEL") {
                    c = c.with_model(m);
                }
                if let Some(b) = base {
                    c.base_url = b;
                }
                c
            }
            _ => {
                let base = base.unwrap_or_else(|| "https://api.openai.com/v1".into());
                LlmConfig::openai_compatible(base, model, key)
            }
        });
    }
    if let Some(key) = env("ANTHROPIC_API_KEY") {
        let mut c = LlmConfig::claude(key);
        if let Some(m) = env("ANTHROPIC_MODEL") {
            c = c.with_model(m);
        }
        return Ok(c);
    }
    if let Some(key) = env("CEREBRAS_API_KEY") {
        let mut c = LlmConfig::cerebras(key);
        if let Some(m) = env("CEREBRAS_MODEL") {
            c = c.with_model(m);
        }
        return Ok(c);
    }
    Err(LlmError::permanent(
        "no LLM credentials in env (set ANTHROPIC_API_KEY, CEREBRAS_API_KEY, or KOSMO_LLM_API_KEY)",
    ))
}

// ─── Error ──────────────────────────────────────────────────────────────────

/// Transport-level error. `recoverable` ⇒ a retry may succeed (rate limit, 5xx).
#[derive(Clone, Debug)]
pub struct LlmError {
    pub message: String,
    pub recoverable: bool,
}

impl LlmError {
    pub fn permanent(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            recoverable: false,
        }
    }
    pub fn transient(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            recoverable: true,
        }
    }
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "llm error (recoverable={}): {}",
            self.recoverable, self.message
        )
    }
}
impl std::error::Error for LlmError {}

// ─── JSON extraction (pure) ─────────────────────────────────────────────────

/// Extract the first balanced top-level JSON object from `raw`.
///
/// Tolerant of code fences and surrounding prose; string-aware (braces inside
/// string literals don't affect nesting; escaped quotes are handled). Returns
/// `None` if there is no balanced object.
pub fn extract_json_object(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let start = raw.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(raw[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Truncate `s` to at most `max` chars, appending `…` when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_is_anthropic() {
        let c = LlmConfig::claude("k");
        assert_eq!(c.provider, LlmProvider::Anthropic);
        assert_eq!(c.tag, "claude");
        assert!(c.label().starts_with("claude:"));
    }

    #[test]
    fn cerebras_is_openai_compatible() {
        let c = LlmConfig::cerebras("k");
        assert_eq!(c.provider, LlmProvider::OpenAiCompatible);
        assert_eq!(c.label(), "cerebras:llama-3.3-70b");
    }

    #[test]
    fn endpoint_per_provider() {
        assert!(LlmConfig::claude("k").endpoint().ends_with("/v1/messages"));
        assert!(LlmConfig::cerebras("k")
            .endpoint()
            .ends_with("/chat/completions"));
    }

    #[test]
    fn anthropic_body_uses_top_level_system() {
        let body = LlmConfig::claude("k").request_body("SYS", "USR");
        assert_eq!(body["system"], "SYS");
        assert_eq!(body["messages"][0]["content"], "USR");
    }

    #[test]
    fn openai_body_uses_system_message() {
        let body = LlmConfig::cerebras("k").request_body("SYS", "USR");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "SYS");
        assert_eq!(body["messages"][1]["content"], "USR");
    }

    #[test]
    fn extract_anthropic_content() {
        let v = json!({ "content": [{ "text": "hello" }], "usage": { "output_tokens": 5 } });
        let (text, tokens) = LlmConfig::claude("k").extract_content(&v).unwrap();
        assert_eq!(text, "hello");
        assert_eq!(tokens, 5);
    }

    #[test]
    fn extract_openai_content() {
        let v = json!({
            "choices": [{ "message": { "content": "hi" } }],
            "usage": { "completion_tokens": 3 }
        });
        let (text, tokens) = LlmConfig::cerebras("k").extract_content(&v).unwrap();
        assert_eq!(text, "hi");
        assert_eq!(tokens, 3);
    }

    #[test]
    fn complete_with_empty_key_errors_without_network() {
        let err = LlmConfig::claude("").complete("s", "u").unwrap_err();
        assert!(!err.recoverable);
        assert!(err.message.contains("API key"));
    }

    #[test]
    fn extract_json_plain() {
        assert_eq!(
            extract_json_object(r#"{"a":1}"#).as_deref(),
            Some(r#"{"a":1}"#)
        );
    }

    #[test]
    fn extract_json_from_fenced_prose() {
        let raw = "Here you go:\n```json\n{\"a\": 1, \"b\": 2}\n```\nThanks!";
        assert_eq!(
            extract_json_object(raw).as_deref(),
            Some(r#"{"a": 1, "b": 2}"#)
        );
    }

    #[test]
    fn extract_json_handles_nested_and_strings() {
        let raw = r#"prefix {"k": "}{ not a brace", "n": {"x": 1}} suffix"#;
        let got = extract_json_object(raw).unwrap();
        assert_eq!(got, r#"{"k": "}{ not a brace", "n": {"x": 1}}"#);
    }

    #[test]
    fn extract_json_handles_escaped_quotes() {
        let raw = r#"{"k": "a \" } brace"}"#;
        let got = extract_json_object(raw).unwrap();
        assert_eq!(got, r#"{"k": "a \" } brace"}"#);
    }

    #[test]
    fn extract_json_none_when_unbalanced() {
        assert!(extract_json_object("{ unbalanced").is_none());
        assert!(extract_json_object("no json here").is_none());
    }

    #[test]
    fn truncate_appends_ellipsis() {
        assert_eq!(truncate("abcdef", 3), "abc…");
        assert_eq!(truncate("ab", 3), "ab");
    }
}
