use serde::{Deserialize, Serialize};

/// OmpConfiguration represents an OMP (omp.sh) configuration
///
/// Supports two authentication modes:
/// - "anthropic": Uses ANTHROPIC_API_KEY or ANTHROPIC_OAUTH_TOKEN
/// - "openai": Uses OPENAI_API_KEY
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OmpConfiguration {
    /// Unique alias name for this configuration
    pub alias_name: String,
    /// Authentication mode: "anthropic" or "openai"
    pub auth_mode: String,
    /// Anthropic API key (anthropic mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_api_key: Option<String>,
    /// Anthropic OAuth token (anthropic mode, takes precedence over API key)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_oauth_token: Option<String>,
    /// OpenAI API key (openai mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// Base URL for API endpoint (optional, e.g., ANTHROPIC_BASE_URL)
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
    fn test_serialize_anthropic_mode() {
        let config = OmpConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "anthropic".to_string(),
            anthropic_api_key: Some("sk-ant-test".to_string()),
            anthropic_oauth_token: None,
            openai_api_key: None,
            base_url: None,
            slow_model: Some("claude-3-5-sonnet-20241022".to_string()),
            smol_model: None,
            plan_model: None,
        };

        let json = serde_json::to_string(&config).expect("Should serialize");
        assert!(json.contains("\"auth_mode\":\"anthropic\""));
        assert!(json.contains("\"anthropic_api_key\":\"sk-ant-test\""));
        assert!(!json.contains("openai_api_key"));
    }

    #[test]
    fn test_deserialize_anthropic_mode() {
        let json = r#"{
            "alias_name": "test",
            "auth_mode": "anthropic",
            "anthropic_api_key": "sk-ant-test",
            "slow_model": "claude-3-5-sonnet-20241022"
        }"#;

        let config: OmpConfiguration = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(config.auth_mode, "anthropic");
        assert_eq!(config.anthropic_api_key, Some("sk-ant-test".to_string()));
        assert_eq!(
            config.slow_model,
            Some("claude-3-5-sonnet-20241022".to_string())
        );
        assert_eq!(config.openai_api_key, None);
    }

    #[test]
    fn test_serialize_openai_mode() {
        let config = OmpConfiguration {
            alias_name: "test".to_string(),
            auth_mode: "openai".to_string(),
            anthropic_api_key: None,
            anthropic_oauth_token: None,
            openai_api_key: Some("sk-test".to_string()),
            base_url: None,
            slow_model: Some("openai/gpt-5.3-codex:high".to_string()),
            smol_model: Some("openai/gpt-4o-mini".to_string()),
            plan_model: None,
        };

        let json = serde_json::to_string(&config).expect("Should serialize");
        assert!(json.contains("\"auth_mode\":\"openai\""));
        assert!(json.contains("\"openai_api_key\":\"sk-test\""));
        assert!(!json.contains("anthropic_api_key"));
    }

    #[test]
    fn test_deserialize_openai_mode() {
        let json = r#"{
            "alias_name": "test",
            "auth_mode": "openai",
            "openai_api_key": "sk-test",
            "slow_model": "openai/gpt-5.3-codex:high"
        }"#;

        let config: OmpConfiguration = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(config.auth_mode, "openai");
        assert_eq!(config.openai_api_key, Some("sk-test".to_string()));
        assert_eq!(
            config.slow_model,
            Some("openai/gpt-5.3-codex:high".to_string())
        );
        assert_eq!(config.anthropic_api_key, None);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = OmpConfiguration {
            alias_name: "work".to_string(),
            auth_mode: "anthropic".to_string(),
            anthropic_api_key: Some("sk-ant-work".to_string()),
            anthropic_oauth_token: None,
            openai_api_key: None,
            base_url: None,
            slow_model: Some("claude-3-5-sonnet-20241022".to_string()),
            smol_model: Some("claude-3-haiku-20240307".to_string()),
            plan_model: Some("claude-3-5-sonnet-20241022".to_string()),
        };

        let json = serde_json::to_string(&original).expect("Should serialize");
        let deserialized: OmpConfiguration =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(original, deserialized);
    }
}
