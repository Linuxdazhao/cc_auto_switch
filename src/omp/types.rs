use serde::{Deserialize, Serialize};

/// OmpConfiguration represents an OMP (omp.sh) configuration.
///
/// omp is configured as an OpenAI-standard-protocol provider: the key is written
/// as `OPENAI_API_KEY` and custom endpoints are registered in `models.yml` with
/// `api: openai-completions`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OmpConfiguration {
    /// Unique alias name for this configuration
    pub alias_name: String,
    /// OpenAI API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// Base URL for API endpoint (optional, custom OpenAI-compatible gateway)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// PI_SLOW_MODEL - slow/reasoning model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_model: Option<String>,
    /// PI_SMOL_MODEL - fast/smaller model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smol_model: Option<String>,
    /// PI_PLAN_MODEL - planning model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_model: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_openai() {
        let config = OmpConfiguration {
            alias_name: "test".to_string(),
            openai_api_key: Some("sk-test".to_string()),
            base_url: None,
            slow_model: Some("openai/gpt-5.3-codex:high".to_string()),
            smol_model: Some("openai/gpt-4o-mini".to_string()),
            plan_model: None,
        };

        let json = serde_json::to_string(&config).expect("Should serialize");
        assert!(json.contains("\"openai_api_key\":\"sk-test\""));
        assert!(json.contains("\"slow_model\":\"openai/gpt-5.3-codex:high\""));
        assert!(!json.contains("plan_model"));
        assert!(!json.contains("auth_mode"));
        assert!(!json.contains("anthropic"));
    }

    #[test]
    fn test_deserialize_openai() {
        let json = r#"{
            "alias_name": "test",
            "openai_api_key": "sk-test",
            "slow_model": "openai/gpt-5.3-codex:high"
        }"#;

        let config: OmpConfiguration = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(config.openai_api_key, Some("sk-test".to_string()));
        assert_eq!(
            config.slow_model,
            Some("openai/gpt-5.3-codex:high".to_string())
        );
        assert_eq!(config.base_url, None);
    }

    #[test]
    fn test_deserialize_ignores_legacy_anthropic_fields() {
        // Configs stored before Anthropic support was removed still deserialize;
        // the obsolete fields are simply ignored.
        let json = r#"{
            "alias_name": "legacy",
            "auth_mode": "anthropic",
            "anthropic_api_key": "sk-ant-old",
            "anthropic_oauth_token": "oauth-old"
        }"#;

        let config: OmpConfiguration = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(config.alias_name, "legacy");
        assert_eq!(config.openai_api_key, None);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = OmpConfiguration {
            alias_name: "work".to_string(),
            openai_api_key: Some("sk-work".to_string()),
            base_url: Some("https://api.example.com/v1".to_string()),
            slow_model: Some("gpt-4o".to_string()),
            smol_model: Some("gpt-4o-mini".to_string()),
            plan_model: Some("gpt-4o".to_string()),
        };

        let json = serde_json::to_string(&original).expect("Should serialize");
        let deserialized: OmpConfiguration =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(original, deserialized);
    }
}
