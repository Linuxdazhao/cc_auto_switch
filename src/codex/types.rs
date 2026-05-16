use serde::{Deserialize, Serialize};

/// CodexConfiguration represents a Codex (OpenAI) API configuration
///
/// Supports two authentication modes:
/// - "chatgpt": Uses OAuth tokens (id_token, access_token, refresh_token, account_id)
/// - "apikey": Uses a direct OpenAI API key
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CodexConfiguration {
    /// Unique alias name for this configuration
    pub alias_name: String,
    /// Authentication mode: "chatgpt" or "apikey"
    pub auth_mode: String,
    /// OpenAI API key (apikey mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// OAuth ID token (chatgpt mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    /// OAuth access token (chatgpt mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// OAuth refresh token (chatgpt mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Account ID (chatgpt mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Last token refresh timestamp (chatgpt mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_apikey_mode() {
        let config = CodexConfiguration {
            alias_name: "my-apikey".to_string(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some("sk-abc123".to_string()),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"auth_mode\":\"apikey\""));
        assert!(json.contains("\"openai_api_key\":\"sk-abc123\""));
        // None fields should not appear
        assert!(!json.contains("id_token"));
        assert!(!json.contains("access_token"));
    }

    #[test]
    fn test_deserialize_apikey_mode() {
        let json = r#"{
            "alias_name": "my-apikey",
            "auth_mode": "apikey",
            "openai_api_key": "sk-abc123"
        }"#;

        let config: CodexConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.alias_name, "my-apikey");
        assert_eq!(config.auth_mode, "apikey");
        assert_eq!(config.openai_api_key, Some("sk-abc123".to_string()));
        assert_eq!(config.id_token, None);
        assert_eq!(config.access_token, None);
    }

    #[test]
    fn test_serialize_chatgpt_mode() {
        let config = CodexConfiguration {
            alias_name: "my-chatgpt".to_string(),
            auth_mode: "chatgpt".to_string(),
            openai_api_key: None,
            id_token: Some("id-xyz".to_string()),
            access_token: Some("at-xyz".to_string()),
            refresh_token: Some("rt-xyz".to_string()),
            account_id: Some("acc-123".to_string()),
            last_refresh: Some("2026-05-16T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"auth_mode\":\"chatgpt\""));
        assert!(json.contains("\"id_token\":\"id-xyz\""));
        assert!(json.contains("\"access_token\":\"at-xyz\""));
        assert!(json.contains("\"refresh_token\":\"rt-xyz\""));
        assert!(json.contains("\"account_id\":\"acc-123\""));
        assert!(json.contains("\"last_refresh\""));
        assert!(!json.contains("openai_api_key"));
    }

    #[test]
    fn test_deserialize_chatgpt_mode() {
        let json = r#"{
            "alias_name": "my-chatgpt",
            "auth_mode": "chatgpt",
            "id_token": "id-xyz",
            "access_token": "at-xyz",
            "refresh_token": "rt-xyz",
            "account_id": "acc-123",
            "last_refresh": "2026-05-16T00:00:00Z"
        }"#;

        let config: CodexConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.alias_name, "my-chatgpt");
        assert_eq!(config.auth_mode, "chatgpt");
        assert_eq!(config.id_token, Some("id-xyz".to_string()));
        assert_eq!(config.access_token, Some("at-xyz".to_string()));
        assert_eq!(config.refresh_token, Some("rt-xyz".to_string()));
        assert_eq!(config.account_id, Some("acc-123".to_string()));
        assert_eq!(
            config.last_refresh,
            Some("2026-05-16T00:00:00Z".to_string())
        );
        assert_eq!(config.openai_api_key, None);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let config = CodexConfiguration {
            alias_name: "roundtrip".to_string(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some("sk-roundtrip".to_string()),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: CodexConfiguration = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
