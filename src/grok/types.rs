use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Map of HTTP header name to value.
pub type HeaderMap = BTreeMap<String, String>;

/// GrokConfiguration represents a custom third-party model definition for the
/// grok CLI (xAI), stored under `[model.<alias_name>]` in `~/.grok/config.toml`.
///
/// The env-based auth path (`env_key`) is the primary way to supply credentials;
/// `api_key` is an alternative that writes the key directly to disk.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GrokConfiguration {
    /// Unique alias name; also used as the `[model.<alias_name>]` id.
    pub alias_name: String,
    /// Underlying model id (e.g. "claude-opus-4-6").
    pub model: String,
    /// API endpoint base URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Human-friendly display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// API backend: "chat_completions" | "responses" | "messages".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_backend: Option<String>,
    /// Context window size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// API key written directly into config (secondary to env_key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Environment variable name(s) to read the key from (primary path).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_key: Option<Vec<String>>,
    /// Extra HTTP headers (e.g. x-api-key, anthropic-version) for the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<HeaderMap>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> GrokConfiguration {
        let mut headers = BTreeMap::new();
        headers.insert("x-api-key".to_string(), "sk-ant-xxx".to_string());
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        GrokConfiguration {
            alias_name: "claude-opus".to_string(),
            model: "claude-opus-4-6".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            name: Some("Claude Opus 4.6".to_string()),
            api_backend: Some("messages".to_string()),
            context_window: Some(200000),
            api_key: None,
            env_key: Some(vec!["ANTHROPIC_API_KEY".to_string()]),
            extra_headers: Some(headers),
        }
    }

    #[test]
    fn test_serialize_skips_none() {
        let config = GrokConfiguration {
            alias_name: "gpt".to_string(),
            model: "gpt-4o".to_string(),
            base_url: None,
            name: None,
            api_backend: None,
            context_window: None,
            api_key: None,
            env_key: Some(vec!["OPENAI_API_KEY".to_string()]),
            extra_headers: None,
        };
        let json = serde_json::to_string(&config).expect("Should serialize");
        assert!(json.contains("\"model\":\"gpt-4o\""));
        assert!(json.contains("\"env_key\":[\"OPENAI_API_KEY\"]"));
        assert!(!json.contains("base_url"));
        assert!(!json.contains("api_key"));
        assert!(!json.contains("extra_headers"));
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = sample();
        let json = serde_json::to_string(&original).expect("Should serialize");
        let deserialized: GrokConfiguration =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(original, deserialized);
    }
}
