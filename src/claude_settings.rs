use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;

use crate::config::types::{ClaudeSettings, Configuration, StorageMode};
use crate::utils::get_claude_settings_path;

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
            settings.env = BTreeMap::new();
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
            self.env = BTreeMap::new();
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
        if let Some(model) = &config.model
            && !model.is_empty()
        {
            self.env
                .insert("ANTHROPIC_MODEL".to_string(), model.clone());
        }

        if let Some(small_fast_model) = &config.small_fast_model
            && !small_fast_model.is_empty()
        {
            self.env.insert(
                "ANTHROPIC_SMALL_FAST_MODEL".to_string(),
                small_fast_model.clone(),
            );
        }
    }

    /// Remove Anthropic environment variables
    ///
    /// Clears all Anthropic-related environment variables from settings
    /// Used to reset to default Claude behavior
    pub fn remove_anthropic_env(&mut self) {
        // Ensure env field exists
        if self.env.is_empty() {
            self.env = BTreeMap::new();
        }

        self.env.remove("ANTHROPIC_AUTH_TOKEN");
        self.env.remove("ANTHROPIC_BASE_URL");
        self.env.remove("ANTHROPIC_MODEL");
        self.env.remove("ANTHROPIC_SMALL_FAST_MODEL");
    }

    /// Switch to a specific API configuration with specified storage mode
    ///
    /// Updates the settings.json file based on the storage mode:
    /// - Env mode: Launch Claude with environment variables (cleans settings.json)
    /// - Config mode: Write to env field in settings.json (settings file persistence)
    ///
    /// # Arguments
    /// * `config` - Configuration containing token, URL, and optional model settings to apply
    /// * `mode` - Storage mode to use (Env or Config)
    /// * `custom_dir` - Optional custom directory for Claude settings
    ///
    /// # Errors
    /// Returns error if settings cannot be saved
    pub fn switch_to_config_with_mode(
        &mut self,
        config: &Configuration,
        mode: StorageMode,
        custom_dir: Option<&str>,
    ) -> Result<()> {
        match mode {
            StorageMode::Env => {
                // Env mode: Check for Anthropic fields written by cc-switch
                let has_anthropic_env = self.env.contains_key("ANTHROPIC_AUTH_TOKEN")
                    || self.env.contains_key("ANTHROPIC_BASE_URL")
                    || self.env.contains_key("ANTHROPIC_MODEL")
                    || self.env.contains_key("ANTHROPIC_SMALL_FAST_MODEL");

                let has_anthropic_config = self.other.contains_key("anthropicAuthToken")
                    || self.other.contains_key("anthropicBaseUrl")
                    || self.other.contains_key("anthropicModel")
                    || self.other.contains_key("anthropicSmallFastModel");

                // Standard Claude settings fields that should be preserved
                // These are legitimate Claude settings that should not be removed
                let standard_claude_fields = [
                    "$schema",
                    "alwaysThinkingEnabled",
                    "claudeCodeDisableNonessentialTraffic",
                    "feedbackSurveyState",
                    "mcpServers",
                    "statusLine",
                    "ui",
                    "theme",
                    "featureFlags",
                    "experiments",
                ];

                // Check for truly unexpected fields (not Anthropic and not standard Claude)
                let mut unexpected_fields = Vec::new();
                for key in self.other.keys() {
                    let is_anthropic_field = key == "anthropicAuthToken"
                        || key == "anthropicBaseUrl"
                        || key == "anthropicModel"
                        || key == "anthropicSmallFastModel"
                        || key == "anthropicDefaultSonnerModel"
                        || key == "anthropicDefaultOpusModel"
                        || key == "anthropicDefaultHaikuModel"
                        || key == "anthropicDefaultSonnetModel"; // handle both spellings

                    let is_standard_claude_field = standard_claude_fields.contains(&key.as_str());

                    if !is_anthropic_field && !is_standard_claude_field {
                        unexpected_fields.push(key.clone());
                    }
                }

                if has_anthropic_env || has_anthropic_config || !unexpected_fields.is_empty() {
                    eprintln!("⚠️  Cleaning up settings.json:");
                    if has_anthropic_env || has_anthropic_config {
                        eprintln!("  - Removing Anthropic settings");
                    }
                    if !unexpected_fields.is_empty() {
                        eprintln!(
                            "  - Removing unexpected fields: {}",
                            unexpected_fields.join(", ")
                        );
                    }
                }

                self.remove_anthropic_env();
                self.remove_anthropic_config_mode();

                // Remove only truly unexpected fields (preserve standard Claude settings)
                for field in unexpected_fields {
                    self.other.remove(&field);
                }

                // Apply the new configuration to env field
                self.switch_to_config(config);

                self.save(custom_dir)?;
            }
            StorageMode::Config => {
                // Config mode: Write Anthropic settings to root level with camelCase names
                // First clean up any old Anthropic fields from 'other' map
                self.remove_anthropic_config_mode();

                // Set Anthropic settings at root level with camelCase names
                self.other.insert(
                    "anthropicAuthToken".to_string(),
                    serde_json::Value::String(config.token.clone()),
                );
                self.other.insert(
                    "anthropicBaseUrl".to_string(),
                    serde_json::Value::String(config.url.clone()),
                );

                // Set model configurations only if provided (don't set empty values)
                if let Some(model) = &config.model
                    && !model.is_empty()
                {
                    self.other.insert(
                        "anthropicModel".to_string(),
                        serde_json::Value::String(model.clone()),
                    );
                }

                if let Some(small_fast_model) = &config.small_fast_model
                    && !small_fast_model.is_empty()
                {
                    self.other.insert(
                        "anthropicSmallFastModel".to_string(),
                        serde_json::Value::String(small_fast_model.clone()),
                    );
                }

                self.save(custom_dir)?;
            }
        }

        Ok(())
    }

    /// Remove Anthropic settings in config mode
    ///
    /// Clears all Anthropic-related settings from env field and other map
    /// Used to reset to default Claude behavior
    pub fn remove_anthropic_config_mode(&mut self) {
        self.remove_anthropic_env();

        // Also remove Anthropic settings from 'other' map
        self.other.remove("anthropicAuthToken");
        self.other.remove("anthropicBaseUrl");
        self.other.remove("anthropicModel");
        self.other.remove("anthropicSmallFastModel");
        self.other.remove("anthropicDefaultSonnetModel");
        self.other.remove("anthropicDefaultOpusModel");
        self.other.remove("anthropicDefaultHaikuModel");
    }
}
