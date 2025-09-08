use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
}

/// Storage manager for Claude API configurations
///
/// Handles persistence and retrieval of multiple API configurations
/// stored in `~/.cc_auto_switch/configurations.json`
#[derive(Serialize, Deserialize, Default)]
pub struct ConfigStorage {
    /// Map of alias names to configuration objects
    pub configurations: HashMap<String, Configuration>,
}

/// Environment variable manager for API configuration
///
/// Handles setting environment variables for the Claude CLI process
#[derive(Default, Clone)]
pub struct EnvironmentConfig {
    /// Environment variables to be set
    pub env_vars: HashMap<String, String>,
}

impl ConfigStorage {
    /// Load configurations from disk
    ///
    /// Reads the JSON file from `~/.cc_auto_switch/configurations.json`
    /// Returns default empty storage if file doesn't exist
    ///
    /// # Errors
    /// Returns error if file exists but cannot be read or parsed
    pub fn load() -> Result<Self> {
        let path = get_config_storage_path()?;

        if !path.exists() {
            return Ok(ConfigStorage::default());
        }

        let content = fs::read_to_string(&path).with_context(|| {
            format!(
                "Failed to read configuration storage from {}",
                path.display()
            )
        })?;

        let storage: ConfigStorage = serde_json::from_str(&content)
            .with_context(|| "Failed to parse configuration storage JSON")?;

        Ok(storage)
    }

    /// Save configurations to disk
    ///
    /// Writes the current state to `~/.cc_auto_switch/configurations.json`
    /// Creates the directory structure if it doesn't exist
    ///
    /// # Errors
    /// Returns error if directory cannot be created or file cannot be written
    pub fn save(&self) -> Result<()> {
        let path = get_config_storage_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let json = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration storage")?;

        fs::write(&path, json).with_context(|| format!("Failed to write to {}", path.display()))?;

        Ok(())
    }

    /// Add a new configuration to storage
    ///
    /// # Arguments
    /// * `config` - Configuration object to add
    ///
    /// Overwrites existing configuration with same alias
    pub fn add_configuration(&mut self, config: Configuration) {
        self.configurations
            .insert(config.alias_name.clone(), config);
    }

    /// Remove a configuration by alias name
    ///
    /// # Arguments
    /// * `alias_name` - Name of configuration to remove
    ///
    /// # Returns
    /// `true` if configuration was found and removed, `false` if not found
    pub fn remove_configuration(&mut self, alias_name: &str) -> bool {
        self.configurations.remove(alias_name).is_some()
    }

    /// Get a configuration by alias name
    ///
    /// # Arguments
    /// * `alias_name` - Name of configuration to retrieve
    ///
    /// # Returns
    /// `Some(&Configuration)` if found, `None` if not found
    pub fn get_configuration(&self, alias_name: &str) -> Option<&Configuration> {
        self.configurations.get(alias_name)
    }
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
        let mut env_vars = HashMap::new();

        // Set required environment variables
        env_vars.insert("ANTHROPIC_AUTH_TOKEN".to_string(), config.token.clone());
        env_vars.insert("ANTHROPIC_BASE_URL".to_string(), config.url.clone());

        // Set model configurations only if provided
        if let Some(model) = &config.model {
            if !model.is_empty() {
                env_vars.insert("ANTHROPIC_MODEL".to_string(), model.clone());
            }
        }

        if let Some(small_fast_model) = &config.small_fast_model {
            if !small_fast_model.is_empty() {
                env_vars.insert(
                    "ANTHROPIC_SMALL_FAST_MODEL".to_string(),
                    small_fast_model.clone(),
                );
            }
        }

        EnvironmentConfig { env_vars }
    }

    /// Create an empty environment configuration (for reset)
    pub fn empty() -> Self {
        EnvironmentConfig {
            env_vars: HashMap::new(),
        }
    }

    /// Get environment variables as a Vec of (key, value) tuples
    /// for use with Command::envs()
    pub fn as_env_tuples(&self) -> Vec<(String, String)> {
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
