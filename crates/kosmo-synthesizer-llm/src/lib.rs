//! LLM-backed [`ActionSynthesizer`] implementations.
//!
//! Two wire protocols are supported behind a single [`LlmSynthesizer`]:
//!
//! * **Claude** — the Anthropic Messages API (`POST {base}/v1/messages`,
//!   `x-api-key` + `anthropic-version` headers, `content[0].text` response).
//! * **OpenAI-compatible** — any provider implementing
//!   `POST {base}/chat/completions` with `Authorization: Bearer …` and the
//!   `choices[0].message.content` response shape. This covers **Cerebras**
//!   (the free-tier bridge), OpenAI, Groq, Together, Fireworks, Ollama, etc.
//!
//! # The non-determinism boundary
//!
//! An LLM call is the one place in the substrate where identical inputs may
//! yield different outputs. That is acceptable and *contained*: the moment a
//! [`Patch`] comes back it is content-addressed again, and everything
//! downstream — [`SynthesisResult`] hashing, `kosmo-agent` validation — is
//! deterministic. Set `temperature = 0` (the default here) for best-effort
//! reproducibility.
//!
//! # Offline-testable core
//!
//! The pure functions [`system_prompt`], [`build_user_prompt`],
//! [`extract_json_object`] and [`parse_synthesis_response`] carry all the
//! logic and are unit-tested without any network access. The HTTP call in
//! [`LlmSynthesizer::synthesize`] is a thin shim around them.
//!
//! # Wire schema
//!
//! The model is instructed to return exactly one JSON object:
//!
//! ```json
//! {
//!   "rationale": "why this patch resolves the action",
//!   "confidence_pct": 85,
//!   "test_hint": "cargo test foo",
//!   "files": [
//!     { "path": "src/foo.rs", "op": "create", "content": "pub fn foo() {}" }
//!   ]
//! }
//! ```
//!
//! `confidence_pct` is an integer 0–100 (no floats cross our boundary —
//! it is mapped via [`Q16::ratio`]); `op` is `create` | `modify` | `delete`.

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use kosmo_core::{Digest, Q16};
use kosmo_pipeline::ActionItemKind;
use kosmo_synthesizer::{
    ActionSynthesizer, FileChange, Patch, SynthesisError, SynthesisRequest, SynthesisResult,
};

// ─── Provider / config ──────────────────────────────────────────────────────

/// Wire protocol of the backing endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlmProvider {
    /// Anthropic Messages API.
    Anthropic,
    /// Any OpenAI-compatible `/chat/completions` endpoint (Cerebras, OpenAI, …).
    OpenAiCompatible,
}

/// Connection + sampling configuration for one [`LlmSynthesizer`].
#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    /// Base URL with no trailing path, e.g. `https://api.anthropic.com` or
    /// `https://api.cerebras.ai/v1`.
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub max_tokens: u32,
    /// Sampling temperature in thousandths (`0..=1000`). Kept as an integer so
    /// no float enters our own logic; it is divided by 1000 only when the
    /// request body is serialized. Default `0` for determinism.
    pub temperature_milli: u32,
    pub timeout_secs: u64,
    /// Short human tag for the synthesizer name / patch `model_hint`
    /// (`"claude"`, `"cerebras"`, `"openai"`).
    pub tag: String,
}

impl LlmConfig {
    /// Claude via the Anthropic Messages API. Defaults to the most capable
    /// current model; override with [`LlmConfig::with_model`] or the
    /// `ANTHROPIC_MODEL` env var (see [`LlmSynthesizer::from_env`]).
    pub fn claude(api_key: impl Into<String>) -> Self {
        Self {
            provider: LlmProvider::Anthropic,
            base_url: "https://api.anthropic.com".into(),
            model: "claude-opus-4-8".into(),
            api_key: api_key.into(),
            max_tokens: 4096,
            temperature_milli: 0,
            timeout_secs: 120,
            tag: "claude".into(),
        }
    }

    /// Cerebras — the OpenAI-compatible free-tier bridge. Default model is a
    /// large free Cerebras model; override via `CEREBRAS_MODEL` if the slug
    /// differs for your account.
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

    /// `"claude:claude-opus-4-8"`, `"cerebras:llama-3.3-70b"`, …
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
        // Float appears only inside the outbound request body — never in any
        // gate path or content-address (CROSS-007 respected).
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

    /// Pull the assistant text and the output-token count from a parsed
    /// response, per provider response shape.
    fn extract_content(&self, val: &serde_json::Value) -> Result<(String, u32), SynthesisError> {
        match self.provider {
            LlmProvider::Anthropic => {
                let text = val["content"][0]["text"].as_str().ok_or_else(|| {
                    SynthesisError::permanent(format!(
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
                        SynthesisError::permanent(format!(
                            "unexpected OpenAI-compatible response shape: {}",
                            truncate(&val.to_string(), 300)
                        ))
                    })?;
                let tokens = val["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;
                Ok((text.to_string(), tokens))
            }
        }
    }
}

// ─── Synthesizer ────────────────────────────────────────────────────────────

/// An [`ActionSynthesizer`] that delegates patch generation to an LLM.
pub struct LlmSynthesizer {
    config: LlmConfig,
    label: String,
    client: reqwest::blocking::Client,
}

impl LlmSynthesizer {
    pub fn new(config: LlmConfig) -> Result<Self, SynthesisError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| SynthesisError::permanent(format!("http client build failed: {e}")))?;
        let label = config.label();
        Ok(Self {
            config,
            label,
            client,
        })
    }

    /// Claude (Anthropic) with the default model.
    pub fn claude(api_key: impl Into<String>) -> Result<Self, SynthesisError> {
        Self::new(LlmConfig::claude(api_key))
    }

    /// Cerebras (OpenAI-compatible free tier) with the default model.
    pub fn cerebras(api_key: impl Into<String>) -> Result<Self, SynthesisError> {
        Self::new(LlmConfig::cerebras(api_key))
    }

    /// Build from the environment.
    ///
    /// Provider selection:
    /// * `KOSMO_LLM_PROVIDER=claude|cerebras|openai` forces a provider; else
    /// * auto-detect: `ANTHROPIC_API_KEY` → Claude, then `CEREBRAS_API_KEY` →
    ///   Cerebras.
    ///
    /// Keys: `ANTHROPIC_API_KEY` / `CEREBRAS_API_KEY` / `KOSMO_LLM_API_KEY`.
    /// Models: `ANTHROPIC_MODEL` / `CEREBRAS_MODEL` / `KOSMO_LLM_MODEL`.
    /// Custom endpoints: `KOSMO_LLM_BASE_URL` (required for `openai`).
    pub fn from_env() -> Result<Self, SynthesisError> {
        let provider = std::env::var("KOSMO_LLM_PROVIDER")
            .unwrap_or_default()
            .to_lowercase();
        let generic_key = std::env::var("KOSMO_LLM_API_KEY").ok();
        let generic_model = std::env::var("KOSMO_LLM_MODEL").ok();

        let mut config = match provider.as_str() {
            "claude" | "anthropic" => {
                let key = std::env::var("ANTHROPIC_API_KEY")
                    .ok()
                    .or_else(|| generic_key.clone())
                    .ok_or_else(|| {
                        SynthesisError::permanent(
                            "KOSMO_LLM_PROVIDER=claude but no ANTHROPIC_API_KEY / KOSMO_LLM_API_KEY",
                        )
                    })?;
                let mut c = LlmConfig::claude(key);
                if let Ok(m) = std::env::var("ANTHROPIC_MODEL") {
                    c = c.with_model(m);
                }
                c
            }
            "cerebras" => {
                let key = std::env::var("CEREBRAS_API_KEY")
                    .ok()
                    .or_else(|| generic_key.clone())
                    .ok_or_else(|| {
                        SynthesisError::permanent(
                            "KOSMO_LLM_PROVIDER=cerebras but no CEREBRAS_API_KEY / KOSMO_LLM_API_KEY",
                        )
                    })?;
                let mut c = LlmConfig::cerebras(key);
                if let Ok(m) = std::env::var("CEREBRAS_MODEL") {
                    c = c.with_model(m);
                }
                c
            }
            "openai" | "openai-compatible" | "custom" => {
                let base = std::env::var("KOSMO_LLM_BASE_URL").map_err(|_| {
                    SynthesisError::permanent(
                        "KOSMO_LLM_PROVIDER=openai requires KOSMO_LLM_BASE_URL",
                    )
                })?;
                let key = generic_key.clone().unwrap_or_default();
                let model = generic_model
                    .clone()
                    .unwrap_or_else(|| "gpt-4o-mini".into());
                LlmConfig::openai_compatible(base, model, key)
            }
            "" => {
                if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                    let mut c = LlmConfig::claude(key);
                    if let Ok(m) = std::env::var("ANTHROPIC_MODEL") {
                        c = c.with_model(m);
                    }
                    c
                } else if let Ok(key) = std::env::var("CEREBRAS_API_KEY") {
                    let mut c = LlmConfig::cerebras(key);
                    if let Ok(m) = std::env::var("CEREBRAS_MODEL") {
                        c = c.with_model(m);
                    }
                    c
                } else {
                    return Err(SynthesisError::permanent(
                        "no LLM provider configured: set ANTHROPIC_API_KEY or CEREBRAS_API_KEY \
                         (or KOSMO_LLM_PROVIDER + KOSMO_LLM_API_KEY)",
                    ));
                }
            }
            other => {
                return Err(SynthesisError::permanent(format!(
                    "unknown KOSMO_LLM_PROVIDER '{other}' (expected claude|cerebras|openai)"
                )));
            }
        };

        // Generic override always wins if explicitly set.
        if let Some(m) = generic_model {
            config = config.with_model(m);
        }
        Self::new(config)
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Send one chat turn, retrying transient rate-limit / server errors with
    /// exponential backoff. Returns `(assistant_text, output_tokens)`.
    fn call(&self, system: &str, user: &str) -> Result<(String, u32), SynthesisError> {
        let url = self.config.endpoint();
        let body = self.config.request_body(system, user);
        let retry_delays = [4u64, 8, 16];
        let mut attempt = 0usize;

        loop {
            let req = self
                .config
                .apply_headers(self.client.post(&url).json(&body));
            let resp = req
                .send()
                .map_err(|e| SynthesisError::transient(format!("http send failed: {e}")))?;
            let status = resp.status();

            // 429 Too Many Requests / 529 Overloaded → retry.
            let retryable =
                status.as_u16() == 429 || status.as_u16() == 529 || status.is_server_error();
            if retryable {
                if attempt < retry_delays.len() {
                    std::thread::sleep(Duration::from_secs(retry_delays[attempt]));
                    attempt += 1;
                    continue;
                }
                return Err(SynthesisError::transient(format!(
                    "endpoint unavailable after {} retries (HTTP {status})",
                    retry_delays.len()
                )));
            }

            if !status.is_success() {
                let text = resp.text().unwrap_or_default();
                return Err(SynthesisError::permanent(format!(
                    "HTTP {status}: {}",
                    truncate(&text, 300)
                )));
            }

            let val: serde_json::Value = resp
                .json()
                .map_err(|e| SynthesisError::permanent(format!("response was not JSON: {e}")))?;
            return self.config.extract_content(&val);
        }
    }
}

impl ActionSynthesizer for LlmSynthesizer {
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisResult, SynthesisError> {
        if self.config.api_key.trim().is_empty() {
            return Err(SynthesisError::permanent(
                "no API key configured for LLM synthesizer",
            ));
        }
        let user = build_user_prompt(request);
        let (raw, tokens) = self.call(system_prompt(), &user)?;
        let mut result = parse_synthesis_response(&raw, request, &self.label)?;
        result.tokens_used = tokens;
        Ok(result.citing(request))
    }

    fn name(&self) -> &str {
        &self.label
    }

    fn token_budget(&self) -> u32 {
        self.config.max_tokens
    }
}

// ─── Prompt construction (pure) ─────────────────────────────────────────────

/// The fixed system prompt that pins the model to the JSON output contract.
pub fn system_prompt() -> &'static str {
    "You are a code synthesis engine embedded in the Kosmocrates substrate. \
     You receive ONE ranked ActionItem describing a structural gap in a Rust \
     workspace and must propose a concrete, minimal patch that resolves it.\n\n\
     Rules:\n\
     - Output EXACTLY one JSON object and nothing else. No prose, no markdown \
     code fences, no explanation outside the JSON.\n\
     - Keep the patch minimal and self-consistent. Prefer creating new files or \
     filling clearly-missing definitions over rewriting unrelated code.\n\
     - If you cannot confidently resolve the action, return an empty \"files\" \
     array and a low \"confidence_pct\".\n\n\
     JSON schema:\n\
     {\n\
       \"rationale\": string,            // why this patch resolves the action\n\
       \"confidence_pct\": integer,      // 0-100 self-assessed correctness\n\
       \"test_hint\": string|null,       // a command to validate, e.g. \"cargo test foo\"\n\
       \"files\": [                       // may be empty\n\
         { \"path\": string, \"op\": \"create\"|\"modify\"|\"delete\", \"content\": string }\n\
       ]\n\
     }"
}

fn short(d: &Digest) -> String {
    let hex = d.to_hex();
    hex.chars().take(12).collect()
}

fn kind_line(kind: &ActionItemKind) -> String {
    match kind {
        ActionItemKind::FillVoid { void_id } => format!(
            "FillVoid — a structural void (id {}) must be filled with new code.",
            short(void_id)
        ),
        ActionItemKind::RepairTopology { surgery_option_id } => format!(
            "RepairTopology — apply topology surgery option {} to repair a structural defect.",
            short(surgery_option_id)
        ),
        ActionItemKind::PromoteToPse { candidate_id } => format!(
            "PromoteToPse — prepare PSE candidate {} for external evaluation.",
            short(candidate_id)
        ),
        ActionItemKind::ReviewCrystal { candidate_id } => format!(
            "ReviewCrystal — review the evidence-only crystal candidate {}.",
            short(candidate_id)
        ),
        ActionItemKind::ApplyNorm { norm_candidate_id, name } => format!(
            "ApplyNorm — apply norm gene '{name}' (candidate {}).",
            short(norm_candidate_id)
        ),
        ActionItemKind::RealizeWishFacet { facet } => format!(
            "RealizeWishFacet — create the wished {:?} '{}' so the workspace satisfies the stated intent.",
            facet.kind, facet.key
        ),
    }
}

/// Build the per-request user prompt from one [`SynthesisRequest`].
pub fn build_user_prompt(request: &SynthesisRequest) -> String {
    let item = &request.action_item;
    let mut s = String::new();
    s.push_str("# Action to resolve\n\n");
    s.push_str(&format!("Kind: {}\n", kind_line(&item.kind)));
    s.push_str(&format!("Description: {}\n", item.description));
    s.push_str(&format!("Action id: {}\n", short(&item.action_id)));
    s.push_str(&format!(
        "Workspace: {}\n",
        request.workspace_path.to_string_lossy()
    ));

    if request.source_snippets.is_empty() {
        s.push_str("\n# Context\n\n(no source context was extracted for this action)\n");
    } else {
        s.push_str("\n# Source context (most relevant first)\n");
        for snip in &request.source_snippets {
            s.push_str(&format!(
                "\n## {}\n```\n{}\n```\n",
                snip.path.to_string_lossy(),
                snip.content
            ));
        }
    }

    if !request.memory_grounding.is_empty() {
        s.push_str(
            "\n# Anchored knowledge (recalled from the promotion ledger)\n\n\
             Certified structural insights this system has previously learned \
             and anchored, ranked by relevance to this action (Pfauenthron \
             D = psi*rho*omega). Advisory context: prefer consistency with it, \
             but always verify against the workspace above.\n",
        );
        for (rank, g) in request.memory_grounding.iter().enumerate() {
            s.push_str(&format!("\n{}. {}\n", rank + 1, g.compact_line()));
        }
    }

    s.push_str("\nReturn the JSON patch object now.");
    s
}

// ─── Response parsing (pure) ────────────────────────────────────────────────

#[derive(Deserialize)]
struct LlmPatchEnvelope {
    #[serde(default)]
    rationale: String,
    #[serde(default = "default_confidence")]
    confidence_pct: u32,
    #[serde(default)]
    test_hint: Option<String>,
    #[serde(default)]
    files: Vec<LlmFileChange>,
}

fn default_confidence() -> u32 {
    50
}

#[derive(Deserialize)]
struct LlmFileChange {
    path: String,
    #[serde(default = "default_op")]
    op: String,
    #[serde(default)]
    content: String,
}

fn default_op() -> String {
    "create".into()
}

impl LlmFileChange {
    fn into_file_change(self) -> FileChange {
        match self.op.to_lowercase().as_str() {
            "delete" | "remove" => FileChange::delete(self.path),
            "modify" | "edit" | "update" => FileChange::modify(self.path, self.content),
            _ => FileChange::create(self.path, self.content),
        }
    }
}

/// Extract the first balanced top-level JSON object from raw model output.
///
/// Tolerates markdown code fences and leading/trailing prose, but ignores
/// braces that occur inside JSON string literals (so `"}"` in content does not
/// terminate the object early).
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

/// Parse a raw model response into a content-addressed [`SynthesisResult`].
///
/// `model_hint` is recorded on the [`Patch`] (e.g. `"cerebras:llama-3.3-70b"`).
pub fn parse_synthesis_response(
    raw: &str,
    request: &SynthesisRequest,
    model_hint: &str,
) -> Result<SynthesisResult, SynthesisError> {
    let json = extract_json_object(raw).ok_or_else(|| {
        SynthesisError::permanent(format!(
            "no JSON object found in model response: {}",
            truncate(raw, 200)
        ))
    })?;
    let env: LlmPatchEnvelope = serde_json::from_str(&json).map_err(|e| {
        SynthesisError::permanent(format!("model response did not match patch schema: {e}"))
    })?;

    let changes: Vec<FileChange> = env
        .files
        .into_iter()
        .map(|f| f.into_file_change())
        .collect();
    let patch = Patch::new(request.request_id, changes, model_hint);

    let pct = env.confidence_pct.min(100) as u64;
    let confidence = Q16::ratio(pct, 100).unwrap_or(Q16::HALF);

    let mut result = SynthesisResult::new(patch, env.rationale, confidence);
    result.test_hint = env.test_hint.filter(|h| !h.trim().is_empty());
    Ok(result)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let kept: String = s.chars().take(max).collect();
        format!("{kept}…")
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::PolicyProfile;
    use kosmo_pipeline::ActionItem;

    fn make_request() -> SynthesisRequest {
        let policy = PolicyProfile::default_report_only();
        let item = ActionItem {
            action_id: Digest::of(&"action-1"),
            priority_score: Q16::ONE,
            kind: ActionItemKind::FillVoid {
                void_id: Digest::of(&"void-1"),
            },
            description: "missing module foo".into(),
            policy_id: policy.id,
        };
        SynthesisRequest::new(item, "/workspace/demo")
    }

    #[test]
    fn config_endpoints_are_correct() {
        let claude = LlmConfig::claude("k");
        assert_eq!(claude.endpoint(), "https://api.anthropic.com/v1/messages");
        assert_eq!(claude.provider, LlmProvider::Anthropic);

        let cb = LlmConfig::cerebras("k");
        assert_eq!(cb.endpoint(), "https://api.cerebras.ai/v1/chat/completions");
        assert_eq!(cb.provider, LlmProvider::OpenAiCompatible);
        assert_eq!(cb.tag, "cerebras");
    }

    #[test]
    fn config_label_combines_tag_and_model() {
        assert_eq!(LlmConfig::cerebras("k").label(), "cerebras:llama-3.3-70b");
        assert_eq!(
            LlmConfig::cerebras("k").with_model("qwen-3-coder").label(),
            "cerebras:qwen-3-coder"
        );
    }

    #[test]
    fn request_bodies_differ_by_provider() {
        let claude = LlmConfig::claude("k").request_body("SYS", "USER");
        // Anthropic carries `system` at the top level.
        assert_eq!(claude["system"], "SYS");
        assert_eq!(claude["messages"][0]["role"], "user");

        let cb = LlmConfig::cerebras("k").request_body("SYS", "USER");
        // OpenAI-compatible folds the system prompt into the messages array.
        assert_eq!(cb["messages"][0]["role"], "system");
        assert_eq!(cb["messages"][0]["content"], "SYS");
        assert_eq!(cb["messages"][1]["role"], "user");
    }

    #[test]
    fn extract_content_per_provider_shape() {
        let claude = LlmConfig::claude("k");
        let v = json!({ "content": [{ "text": "hello" }], "usage": { "output_tokens": 7 } });
        assert_eq!(claude.extract_content(&v).unwrap(), ("hello".into(), 7));

        let cb = LlmConfig::cerebras("k");
        let v = json!({
            "choices": [{ "message": { "content": "world" } }],
            "usage": { "completion_tokens": 9 }
        });
        assert_eq!(cb.extract_content(&v).unwrap(), ("world".into(), 9));
    }

    #[test]
    fn user_prompt_mentions_action_and_workspace() {
        let req = make_request();
        let p = build_user_prompt(&req);
        assert!(p.contains("FillVoid"));
        assert!(p.contains("missing module foo"));
        assert!(p.contains("/workspace/demo"));
        assert!(p.contains("no source context"));
    }

    #[test]
    fn extract_json_object_strips_fences_and_prose() {
        let raw =
            "Here is the patch:\n```json\n{\"confidence_pct\": 80, \"files\": []}\n```\nDone.";
        let json = extract_json_object(raw).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("confidence_pct"));
    }

    #[test]
    fn extract_json_object_ignores_braces_in_strings() {
        let raw = r#"{"content": "fn x() { let y = 1; }"}"#;
        let json = extract_json_object(raw).unwrap();
        // The whole object is returned, not truncated at the inner '}'.
        assert_eq!(json, raw);
    }

    #[test]
    fn parse_well_formed_response_builds_patch() {
        let req = make_request();
        let raw = r#"{
            "rationale": "create the missing module",
            "confidence_pct": 85,
            "test_hint": "cargo test foo",
            "files": [
                { "path": "src/foo.rs", "op": "create", "content": "pub fn foo() {}" }
            ]
        }"#;
        let result = parse_synthesis_response(raw, &req, "test-model").unwrap();
        assert_eq!(result.patch.file_changes.len(), 1);
        assert_eq!(result.patch.model_hint, "test-model");
        assert_eq!(result.patch.request_id, req.request_id);
        assert_eq!(result.test_hint.as_deref(), Some("cargo test foo"));
        // 85% → Q16 between HALF and ONE.
        assert!(result.confidence > Q16::HALF);
        assert!(result.confidence < Q16::ONE);
    }

    #[test]
    fn parse_clamps_confidence_over_100() {
        let req = make_request();
        let raw = r#"{ "confidence_pct": 150, "files": [] }"#;
        let result = parse_synthesis_response(raw, &req, "m").unwrap();
        assert_eq!(result.confidence, Q16::ONE);
        assert!(result.patch.is_empty());
    }

    #[test]
    fn parse_defaults_confidence_when_missing() {
        let req = make_request();
        let raw = r#"{ "files": [] }"#;
        let result = parse_synthesis_response(raw, &req, "m").unwrap();
        // default 50% == HALF
        assert_eq!(result.confidence, Q16::HALF);
    }

    #[test]
    fn parse_delete_op_maps_to_delete_change() {
        let req = make_request();
        let raw = r#"{ "confidence_pct": 60, "files": [ { "path": "old.rs", "op": "delete" } ] }"#;
        let result = parse_synthesis_response(raw, &req, "m").unwrap();
        let fc = &result.patch.file_changes[0];
        assert_eq!(fc.line_count(), 0);
        assert!(fc.content.is_empty());
    }

    #[test]
    fn parse_rejects_non_json() {
        let req = make_request();
        let err = parse_synthesis_response("I cannot help with that.", &req, "m").unwrap_err();
        assert!(!err.recoverable);
    }

    #[test]
    fn parse_is_deterministic_for_same_text() {
        let req = make_request();
        let raw = r#"{ "confidence_pct": 70, "files": [ { "path": "a.rs", "content": "x" } ] }"#;
        let r1 = parse_synthesis_response(raw, &req, "m").unwrap();
        let r2 = parse_synthesis_response(raw, &req, "m").unwrap();
        assert_eq!(r1.result_id, r2.result_id);
        assert_eq!(r1.patch.patch_id, r2.patch.patch_id);
    }

    #[test]
    fn empty_api_key_fails_fast() {
        let synth = LlmSynthesizer::cerebras("").unwrap();
        let req = make_request();
        let err = synth.synthesize(&req).unwrap_err();
        assert!(!err.recoverable);
        assert!(err.message.contains("API key"));
    }

    /// Live smoke test against a real Cerebras endpoint. Ignored by default;
    /// run with `CEREBRAS_API_KEY=… cargo test -p kosmo-synthesizer-llm \
    /// live_cerebras -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_cerebras_synthesizes() {
        let key = std::env::var("CEREBRAS_API_KEY").expect("CEREBRAS_API_KEY required");
        let synth = LlmSynthesizer::cerebras(key).unwrap();
        let req = make_request();
        let result = synth.synthesize(&req).unwrap();
        eprintln!(
            "confidence={:?} files={} rationale={}",
            result.confidence,
            result.patch.file_changes.len(),
            result.rationale
        );
        assert_eq!(result.patch.request_id, req.request_id);
    }

    // ─── Memory grounding in the prompt ──────────────────────────────────────

    fn grounded_request() -> SynthesisRequest {
        make_request().with_grounding(vec![kosmo_pse_bridge::MemoryGroundingEntry {
            crystal_id: "ab12cd34ef56ab12".into(),
            stability: 0.76,
            qtic_class: Some(5),
            tripolar_score: 0.4668,
            commit_index: 3,
            scale_tag: "il-refined".into(),
            question: "kosmo-promote:/ws/alpha".into(),
        }])
    }

    #[test]
    fn prompt_renders_anchored_knowledge_section() {
        let prompt = build_user_prompt(&grounded_request());
        assert!(prompt.contains("# Anchored knowledge"));
        assert!(prompt.contains(
            "1. D=0.4668 | Q5 | stability 0.76 | t=3 | il-refined \
             | ab12cd34ef56ab12 | kosmo-promote:/ws/alpha"
        ));
        // The section is advisory, the workspace stays authoritative.
        assert!(prompt.contains("always verify against the workspace"));
    }

    #[test]
    fn prompt_omits_memory_section_without_grounding() {
        let prompt = build_user_prompt(&make_request());
        assert!(!prompt.contains("# Anchored knowledge"));
    }
}
