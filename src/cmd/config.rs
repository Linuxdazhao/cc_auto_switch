use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
    /// Custom directory for Claude settings (optional)
    pub claude_settings_dir: Option<String>,
}

/// Claude settings manager for API configuration
///
/// Manages the Claude settings.json file to control Claude's API configuration
/// Handles environment variables and preserves other settings
#[derive(Default, Clone)]
pub struct ClaudeSettings {
    /// Environment variables map (ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL, ANTHROPIC_MODEL, ANTHROPIC_SMALL_FAST_MODEL)
    pub env: HashMap<String, String>,
    /// Other settings to preserve when modifying API configuration
    pub other: HashMap<String, serde_json::Value>,
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
            env: HashMap<String, String>,
            #[serde(flatten)]
            other: HashMap<String, serde_json::Value>,
        }

        let helper = ClaudeSettingsHelper::deserialize(deserializer)?;
        Ok(ClaudeSettings {
            env: helper.env,
            other: helper.other,
        })
    }
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

    /// Set the default directory for Claude settings
    ///
    /// # Arguments
    /// * `directory` - Directory path for Claude settings
    pub fn set_claude_settings_dir(&mut self, directory: String) {
        self.claude_settings_dir = Some(directory);
    }

    /// Get the current Claude settings directory
    ///
    /// # Returns
    /// `Some(&String)` if custom directory is set, `None` if using default
    pub fn get_claude_settings_dir(&self) -> Option<&String> {
        self.claude_settings_dir.as_ref()
    }
}

impl ClaudeSettings {
    /// Load Claude settings from disk
    ///
    /// Reads the JSON file from the configured Claude settings directory
    /// Returns default empty settings if file doesn't exist
    /// Creates the file with default structure if it doesn't exist
    ///
    /// # Arguments
    /// * `custom_dir` - Optional custom directory for Claude settings
    ///
    /// # Errors
    /// Returns error if file exists but cannot be read or parsed
    pub fn load(custom_dir: Option<&str>) -> Result<Self> {
        let path = get_claude_settings_path(custom_dir)?;

        if !path.exists() {
            // Create default settings file if it doesn't exist
            let default_settings = ClaudeSettings::default();
            default_settings.save(custom_dir)?;
            return Ok(default_settings);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read Claude settings from {}", path.display()))?;

        // Parse with better error handling for missing env field
        let mut settings: ClaudeSettings = if content.trim().is_empty() {
            ClaudeSettings::default()
        } else {
            serde_json::from_str(&content)
                .with_context(|| "Failed to parse Claude settings JSON")?
        };

        // Ensure env field exists (handle case where it might be missing from JSON)
        if settings.env.is_empty() && !content.contains("\"env\"") {
            settings.env = HashMap::new();
        }

        Ok(settings)
    }

    /// Save Claude settings to disk
    ///
    /// Writes the current state to the configured Claude settings directory
    /// Creates the directory structure if it doesn't exist
    /// Ensures the env field is properly serialized
    ///
    /// # Arguments
    /// * `custom_dir` - Optional custom directory for Claude settings
    ///
    /// # Errors
    /// Returns error if directory cannot be created or file cannot be written
    pub fn save(&self, custom_dir: Option<&str>) -> Result<()> {
        let path = get_claude_settings_path(custom_dir)?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        // The custom Serialize implementation handles env field inclusion automatically
        let settings_to_save = self;

        let json = serde_json::to_string_pretty(&settings_to_save)
            .with_context(|| "Failed to serialize Claude settings")?;

        fs::write(&path, json).with_context(|| format!("Failed to write to {}", path.display()))?;

        Ok(())
    }

    /// Switch to a specific API configuration
    ///
    /// Updates the environment variables with the provided configuration
    /// Ensures env field exists before updating
    ///
    /// # Arguments
    /// * `config` - Configuration containing token, URL, and optional model settings to apply
    pub fn switch_to_config(&mut self, config: &Configuration) {
        // Ensure env field exists
        if self.env.is_empty() {
            self.env = HashMap::new();
        }

        // First remove all Anthropic environment variables to ensure clean state
        self.env.remove("ANTHROPIC_AUTH_TOKEN");
        self.env.remove("ANTHROPIC_BASE_URL");
        self.env.remove("ANTHROPIC_MODEL");
        self.env.remove("ANTHROPIC_SMALL_FAST_MODEL");

        // Set required environment variables
        self.env
            .insert("ANTHROPIC_AUTH_TOKEN".to_string(), config.token.clone());
        self.env
            .insert("ANTHROPIC_BASE_URL".to_string(), config.url.clone());

        // Set model configurations only if provided (don't set empty values)
        if let Some(model) = &config.model {
            if !model.is_empty() {
                self.env
                    .insert("ANTHROPIC_MODEL".to_string(), model.clone());
            }
        }

        if let Some(small_fast_model) = &config.small_fast_model {
            if !small_fast_model.is_empty() {
                self.env.insert(
                    "ANTHROPIC_SMALL_FAST_MODEL".to_string(),
                    small_fast_model.clone(),
                );
            }
        }
    }

    /// Remove Anthropic environment variables
    ///
    /// Clears all Anthropic-related environment variables from settings
    /// Used to reset to default Claude behavior
    pub fn remove_anthropic_env(&mut self) {
        // Ensure env field exists
        if self.env.is_empty() {
            self.env = HashMap::new();
        }

        self.env.remove("ANTHROPIC_AUTH_TOKEN");
        self.env.remove("ANTHROPIC_BASE_URL");
        self.env.remove("ANTHROPIC_MODEL");
        self.env.remove("ANTHROPIC_SMALL_FAST_MODEL");
    }
}

/// Get the path to the configuration storage file
///
/// Returns `~/.cc_auto_switch/configurations.json`
///
/// # Errors
/// Returns error if home directory cannot be found
pub fn get_config_storage_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    Ok(home_dir.join(".cc-switch").join("configurations.json"))
}

/// Get the path to the Claude settings file
///
/// Returns the path to settings.json, using custom directory if configured
/// Defaults to `~/.claude/settings.json`
///
/// # Errors
/// Returns error if home directory cannot be found or path is invalid
pub fn get_claude_settings_path(custom_dir: Option<&str>) -> Result<PathBuf> {
    if let Some(dir) = custom_dir {
        let custom_path = PathBuf::from(dir);
        if custom_path.is_absolute() {
            Ok(custom_path.join("settings.json"))
        } else {
            // If relative path, resolve from home directory
            let home_dir = dirs::home_dir().context("Could not find home directory")?;
            Ok(home_dir.join(custom_path).join("settings.json"))
        }
    } else {
        // Default path
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        Ok(home_dir.join(".claude").join("settings.json"))
    }
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
    if alias_name.contains(' ') {
        anyhow::bail!("Alias name cannot contain whitespace");
    }
    Ok(())
}