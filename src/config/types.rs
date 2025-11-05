use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// Type alias for configuration map
type ConfigMap = BTreeMap<String, Configuration>;
/// Type alias for environment variable map
type EnvMap = BTreeMap<String, String>;
/// Type alias for JSON value map
type JsonMap = BTreeMap<String, serde_json::Value>;

/// Storage mode for how configuration should be written to settings.json
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum StorageMode {
    /// Write to env field with uppercase environment variable names (default)
    #[serde(rename = "env")]
    #[default]
    Env,
    /// Write to root level with camelCase field names
    #[serde(rename = "config")]
    Config,
}

/// Represents a Claude API configuration
///
/// Contains the components needed to configure Claude API access:
/// - alias_name: User-friendly identifier for the configuration
/// - token: API authentication token
/// - url: Base URL for the API endpoint
/// - model: Optional custom model name
/// - small_fast_model: Optional Haiku-class model for background tasks
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// User-friendly alias name for this configuration
    pub alias_name: String,
    /// ANTHROPIC_AUTH_TOKEN value (API authentication token)
    pub token: String,
    /// ANTHROPIC_BASE_URL value (API endpoint URL)
    pub url: String,
    /// ANTHROPIC_MODEL value (custom model name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ANTHROPIC_SMALL_FAST_MODEL value (Haiku-class model for background tasks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_fast_model: Option<String>,
    /// ANTHROPIC_MAX_THINKING_TOKENS value (Maximum thinking tokens limit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_thinking_tokens: Option<u32>,
    /// API timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_timeout_ms: Option<u32>,
    /// Disable non-essential traffic flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_code_disable_nonessential_traffic: Option<u32>,
    /// Default Sonnet model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_default_sonnet_model: Option<String>,
    /// Default Opus model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_default_opus_model: Option<String>,
    /// Default Haiku model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_default_haiku_model: Option<String>,
}

impl Configuration {
    /// Get all environment variable names that this configuration can set
    ///
    /// Returns a vector of all UPPERCASE environment variable names
    /// that can be set by this configuration, used for conflict detection
    /// in env mode.
    pub fn get_env_field_names() -> Vec<&'static str> {
        vec![
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_MODEL",
            "ANTHROPIC_SMALL_FAST_MODEL",
            "ANTHROPIC_MAX_THINKING_TOKENS",
            "API_TIMEOUT_MS",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_field_names() {
        let fields = Configuration::get_env_field_names();

        // Verify all expected fields are present
        let expected_fields = vec![
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_MODEL",
            "ANTHROPIC_SMALL_FAST_MODEL",
            "ANTHROPIC_MAX_THINKING_TOKENS",
            "API_TIMEOUT_MS",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        ];

        assert_eq!(
            fields.len(),
            expected_fields.len(),
            "Should have exactly 10 fields"
        );

        for expected_field in expected_fields {
            assert!(
                fields.contains(&expected_field),
                "Missing field: {}",
                expected_field
            );
        }

        // Verify all fields are uppercase
        for field in &fields {
            assert_eq!(
                field,
                &field.to_uppercase(),
                "Field {} should be uppercase",
                field
            );
        }
    }

    #[test]
    fn test_remove_anthropic_env_uses_dynamic_fields() {
        let mut settings = ClaudeSettings::default();

        // Add all possible environment variables
        let env_fields = Configuration::get_env_field_names();
        for field in &env_fields {
            settings
                .env
                .insert(field.to_string(), "test_value".to_string());
        }

        // Add some other env variables that shouldn't be removed
        settings
            .env
            .insert("OTHER_VAR".to_string(), "other_value".to_string());
        settings
            .env
            .insert("CLAUDE_THEME".to_string(), "dark".to_string());

        // Remove Anthropic environment variables
        settings.remove_anthropic_env();

        // Verify all Anthropic fields are removed
        for field in &env_fields {
            assert!(
                !settings.env.contains_key(*field),
                "Field {} should be removed",
                field
            );
        }

        // Verify other fields are preserved
        assert!(
            settings.env.contains_key("OTHER_VAR"),
            "Other variables should be preserved"
        );
        assert!(
            settings.env.contains_key("CLAUDE_THEME"),
            "Other variables should be preserved"
        );
        assert_eq!(
            settings.env.get("OTHER_VAR"),
            Some(&"other_value".to_string())
        );
        assert_eq!(settings.env.get("CLAUDE_THEME"), Some(&"dark".to_string()));
    }

    #[test]
    fn test_switch_to_config_uses_dynamic_fields() {
        let mut settings = ClaudeSettings::default();

        // Add all possible environment variables
        let env_fields = Configuration::get_env_field_names();
        for field in &env_fields {
            settings
                .env
                .insert(field.to_string(), "old_value".to_string());
        }

        // Create a test configuration
        let config = Configuration {
            alias_name: "test".to_string(),
            token: "new_token".to_string(),
            url: "https://api.new.com".to_string(),
            model: Some("new_model".to_string()),
            small_fast_model: Some("new_fast_model".to_string()),
            max_thinking_tokens: Some(50000),
            api_timeout_ms: Some(300000),
            claude_code_disable_nonessential_traffic: Some(1),
            anthropic_default_sonnet_model: Some("new_sonnet".to_string()),
            anthropic_default_opus_model: Some("new_opus".to_string()),
            anthropic_default_haiku_model: Some("new_haiku".to_string()),
        };

        // Switch to new configuration
        settings.switch_to_config(&config);

        // Verify the required fields are set correctly
        assert_eq!(
            settings.env.get("ANTHROPIC_AUTH_TOKEN"),
            Some(&"new_token".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_BASE_URL"),
            Some(&"https://api.new.com".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_MODEL"),
            Some(&"new_model".to_string())
        );
        assert_eq!(
            settings.env.get("ANTHROPIC_SMALL_FAST_MODEL"),
            Some(&"new_fast_model".to_string())
        );

        // Verify fields not set in the config are removed (not just left with old values)
        assert!(!settings.env.contains_key("ANTHROPIC_MAX_THINKING_TOKENS"));
        assert!(!settings.env.contains_key("API_TIMEOUT_MS"));
        assert!(
            !settings
                .env
                .contains_key("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC")
        );
        assert!(!settings.env.contains_key("ANTHROPIC_DEFAULT_SONNET_MODEL"));
        assert!(!settings.env.contains_key("ANTHROPIC_DEFAULT_OPUS_MODEL"));
        assert!(!settings.env.contains_key("ANTHROPIC_DEFAULT_HAIKU_MODEL"));
    }
}

/// Storage manager for Claude API configurations
///
/// Handles persistence and retrieval of multiple API configurations
/// stored in `~/.cc_auto_switch/configurations.json`
#[derive(Serialize, Deserialize, Default)]
pub struct ConfigStorage {
    /// Map of alias names to configuration objects
    pub configurations: ConfigMap,
    /// Custom directory for Claude settings (optional)
    pub claude_settings_dir: Option<String>,
    /// Default storage mode for writing configurations (None = use env mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_storage_mode: Option<StorageMode>,
}

/// Claude settings manager for API configuration
///
/// Manages the Claude settings.json file to control Claude's API configuration
/// Handles environment variables and preserves other settings
#[derive(Default, Clone)]
#[allow(dead_code)]
pub struct ClaudeSettings {
    /// Environment variables map (ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL, ANTHROPIC_MODEL, ANTHROPIC_SMALL_FAST_MODEL)
    pub env: EnvMap,
    /// Other settings to preserve when modifying API configuration
    pub other: JsonMap,
}

impl Serialize for ClaudeSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(
            self.other.len() + if self.env.is_empty() { 0 } else { 1 },
        ))?;

        // Serialize env field only if it has content
        if !self.env.is_empty() {
            map.serialize_entry("env", &self.env)?;
        }

        // Serialize other fields
        for (key, value) in &self.other {
            map.serialize_entry(key, value)?;
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for ClaudeSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ClaudeSettingsHelper {
            #[serde(default)]
            env: EnvMap,
            #[serde(flatten)]
            other: JsonMap,
        }

        let helper = ClaudeSettingsHelper::deserialize(deserializer)?;
        Ok(ClaudeSettings {
            env: helper.env,
            other: helper.other,
        })
    }
}

/// Parameters for adding a new configuration
#[allow(dead_code)]
pub struct AddCommandParams {
    pub alias_name: String,
    pub token: Option<String>,
    pub url: Option<String>,
    pub model: Option<String>,
    pub small_fast_model: Option<String>,
    pub max_thinking_tokens: Option<u32>,
    pub api_timeout_ms: Option<u32>,
    pub claude_code_disable_nonessential_traffic: Option<u32>,
    pub anthropic_default_sonnet_model: Option<String>,
    pub anthropic_default_opus_model: Option<String>,
    pub anthropic_default_haiku_model: Option<String>,
    pub force: bool,
    pub interactive: bool,
    pub token_arg: Option<String>,
    pub url_arg: Option<String>,
    pub from_file: Option<String>,
}
