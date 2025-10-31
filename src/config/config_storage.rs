use anyhow::{Context, Result};
use std::fs;

use crate::config::config::get_config_storage_path;
use crate::config::types::{ConfigStorage, Configuration};

impl ConfigStorage {
    /// Load configurations from disk
    ///
    /// Reads the JSON file from `~/.claude/cc_auto_switch_setting.json`
    /// Auto-migrates from old location `~/.cc-switch/configurations.json` if it exists
    /// Returns default empty storage if file doesn't exist
    ///
    /// # Errors
    /// Returns error if file exists but cannot be read or parsed
    pub fn load() -> Result<Self> {
        let new_path = get_config_storage_path()?;

        // Check if the new file already exists
        if new_path.exists() {
            let content = fs::read_to_string(&new_path).with_context(|| {
                format!(
                    "Failed to read configuration storage from {}",
                    new_path.display()
                )
            })?;

            let storage: ConfigStorage = serde_json::from_str(&content)
                .with_context(|| "Failed to parse configuration storage JSON")?;

            return Ok(storage);
        }

        // No configuration file exists at new path, return default empty storage
        Ok(ConfigStorage::default())
    }

    /// Save configurations to disk
    ///
    /// Writes the current state to `~/.claude/cc_auto_switch_setting.json`
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

    /// Migrate configurations from old path to new path
    ///
    /// Old path: `~/.cc-switch/configurations.json`
    /// New path: `~/.claude/cc_auto_switch_setting.json`
    ///
    /// Safe to run multiple times. If old path does not exist, returns Ok(()) and prints a note.
    pub fn migrate_from_old_path() -> Result<()> {
        let new_path = get_config_storage_path()?;
        let old_path = dirs::home_dir()
            .map(|home| home.join(".cc-switch").join("configurations.json"))
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

        if !old_path.exists() {
            println!("ℹ️ No old configuration found at {}", old_path.display());
            return Ok(());
        }

        println!("🔄 Migrating configuration from old location...");

        let content = fs::read_to_string(&old_path).with_context(|| {
            format!(
                "Failed to read old configuration from {}",
                old_path.display()
            )
        })?;

        let storage: ConfigStorage = serde_json::from_str(&content)
            .with_context(|| "Failed to parse old configuration storage JSON")?;

        // Save to new location
        storage
            .save()
            .with_context(|| "Failed to save migrated configuration to new location")?;

        // Remove old directory
        if let Some(parent) = old_path.parent() {
            fs::remove_dir_all(parent).with_context(|| {
                format!(
                    "Failed to remove old configuration directory {}",
                    parent.display()
                )
            })?;
        }

        println!(
            "✅ Configuration migrated successfully to {}",
            new_path.display()
        );

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
