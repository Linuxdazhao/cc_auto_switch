use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Type alias for environment variable map
type EnvVarMap = BTreeMap<String, String>;
/// Type alias for environment variable tuples
type EnvVarTuple = (String, String);
/// Type alias for environment variable tuples vector
type EnvVarTuples = Vec<EnvVarTuple>;

// Re-export types for backward compatibility
pub use crate::cmd::types::{ConfigStorage, Configuration};

/// Environment variable manager for API configuration
///
/// Handles setting environment variables for the Claude CLI process
#[derive(Default, Clone)]
pub struct EnvironmentConfig {
    /// Environment variables to be set
    pub env_vars: EnvVarMap,
}

impl EnvironmentConfig {
    /// Create a new environment configuration from a Claude configuration
    ///
    /// # Arguments
    /// * `config` - Configuration containing token, URL, and optional model settings
    ///
    /// # Returns
    /// EnvironmentConfig with the appropriate environment variables set
    pub fn from_config(config: &Configuration) -> Self {
        let mut env_vars = EnvVarMap::new();

        // Set required environment variables
        env_vars.insert("ANTHROPIC_AUTH_TOKEN".to_string(), config.token.clone());
        env_vars.insert("ANTHROPIC_BASE_URL".to_string(), config.url.clone());

        // Set model configurations only if provided
        if let Some(model) = &config.model
            && !model.is_empty()
        {
            env_vars.insert("ANTHROPIC_MODEL".to_string(), model.clone());
        }

        if let Some(small_fast_model) = &config.small_fast_model
            && !small_fast_model.is_empty()
        {
            env_vars.insert(
                "ANTHROPIC_SMALL_FAST_MODEL".to_string(),
                small_fast_model.clone(),
            );
        }

        // Set max thinking tokens only if provided
        if let Some(max_thinking_tokens) = config.max_thinking_tokens {
            env_vars.insert(
                "ANTHROPIC_MAX_THINKING_TOKENS".to_string(),
                max_thinking_tokens.to_string(),
            );
        }

        // Set API timeout only if provided
        if let Some(timeout) = config.api_timeout_ms {
            env_vars.insert("API_TIMEOUT_MS".to_string(), timeout.to_string());
        }

        // Set disable nonessential traffic flag only if provided
        if let Some(flag) = config.claude_code_disable_nonessential_traffic {
            env_vars.insert(
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
                flag.to_string(),
            );
        }

        // Set default Sonnet model only if provided
        if let Some(model) = &config.anthropic_default_sonnet_model
            && !model.is_empty()
        {
            env_vars.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), model.clone());
        }

        // Set default Opus model only if provided
        if let Some(model) = &config.anthropic_default_opus_model
            && !model.is_empty()
        {
            env_vars.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), model.clone());
        }

        // Set default Haiku model only if provided
        if let Some(model) = &config.anthropic_default_haiku_model
            && !model.is_empty()
        {
            env_vars.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), model.clone());
        }

        EnvironmentConfig { env_vars }
    }

    /// Create an empty environment configuration (for reset)
    pub fn empty() -> Self {
        EnvironmentConfig {
            env_vars: EnvVarMap::new(),
        }
    }

    /// Get environment variables as a Vec of (key, value) tuples
    /// for use with Command::envs()
    pub fn as_env_tuples(&self) -> EnvVarTuples {
        self.env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// Get the path to the configuration storage file
///
/// Returns `~/.cc-switch/configurations.json`
///
/// # Errors
/// Returns error if home directory cannot be found
pub fn get_config_storage_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    Ok(home_dir.join(".cc-switch").join("configurations.json"))
}

/// Validate alias name
///
/// # Arguments
/// * `alias_name` - The alias name to validate
///
/// # Returns
/// Ok(()) if valid, Err with message if invalid
pub fn validate_alias_name(alias_name: &str) -> Result<()> {
    if alias_name.is_empty() {
        anyhow::bail!("Alias name cannot be empty");
    }
    if alias_name == "cc" {
        anyhow::bail!("Alias name 'cc' is reserved and cannot be used");
    }
    if alias_name.chars().any(|c| c.is_whitespace()) {
        anyhow::bail!("Alias name cannot contain whitespace");
    }
    Ok(())
}
