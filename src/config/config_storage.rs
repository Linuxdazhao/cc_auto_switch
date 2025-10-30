use anyhow::{Context, Result};
use std::fs;

use crate::config::config::get_config_storage_path;
use crate::config::types::{ConfigStorage, Configuration};

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
    #[allow(dead_code)]
    pub fn set_claude_settings_dir(&mut self, directory: String) {
        self.claude_settings_dir = Some(directory);
    }

    /// Get the current Claude settings directory
    ///
    /// # Returns
    /// `Some(&String)` if custom directory is set, `None` if using default
    #[allow(dead_code)]
    pub fn get_claude_settings_dir(&self) -> Option<&String> {
        self.claude_settings_dir.as_ref()
    }

    /// Update an existing configuration
    ///
    /// This method handles updating a configuration, including potential alias renaming.
    /// If the new configuration has a different alias name than the old one, it removes
    /// the old entry and creates a new one.
    ///
    /// # Arguments
    /// * `old_alias` - Current alias name of the configuration to update
    /// * `new_config` - Updated configuration object
    ///
    /// # Returns
    /// `Ok(())` if update succeeds, `Err` if the old configuration doesn't exist
    ///
    /// # Errors
    /// Returns error if the configuration with `old_alias` doesn't exist
    pub fn update_configuration(
        &mut self,
        old_alias: &str,
        new_config: Configuration,
    ) -> Result<()> {
        // Check if the old configuration exists
        if !self.configurations.contains_key(old_alias) {
            return Err(anyhow::anyhow!("Configuration '{}' not found", old_alias));
        }

        // If alias changed, remove the old entry
        if old_alias != new_config.alias_name {
            self.configurations.remove(old_alias);
        }

        // Insert the updated configuration (this will overwrite if alias hasn't changed)
        self.configurations
            .insert(new_config.alias_name.clone(), new_config);

        Ok(())
    }
}
