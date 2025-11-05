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

        // Remove all Anthropic environment variables to ensure clean state
        let env_fields = Configuration::get_env_field_names();
        for field in &env_fields {
            self.env.remove(*field);
        }

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

        // Remove all environment variables that can be set by configurations
        let env_fields = Configuration::get_env_field_names();
        for field in &env_fields {
            self.env.remove(*field);
        }
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
                // Env mode: Check for conflicts with existing configurable fields
                // Automatically remove them from settings.json if found

                // Get all environment variable names that can be set by configurations
                let anthropic_env_fields = Configuration::get_env_field_names();

                let mut removed_fields = Vec::new();

                // Check and remove Anthropic variables from env field
                for field in &anthropic_env_fields {
                    if self.env.remove(*field).is_some() {
                        removed_fields.push(field.to_string());
                    }
                }

                // If fields were removed, report what was cleaned and save
                if !removed_fields.is_empty() {
                    eprintln!("🧹 Cleaning settings.json for env mode:");
                    eprintln!("   Removed configurable fields:");
                    for field in &removed_fields {
                        eprintln!("   - {}", field);
                    }
                    eprintln!();
                    eprintln!(
                        "   Settings.json cleaned. Environment variables will be used instead."
                    );

                    // Save the cleaned settings
                    self.save(custom_dir)?;
                }

                // Env mode: Environment variables will be set directly when launching Claude
            }
            StorageMode::Config => {
                // Config mode: Write Anthropic settings to env field with UPPERCASE names
                // Check for conflicts with system environment variables and settings.json
                // Report errors and exit if configurable fields are found

                // Get all environment variable names that can be set by configurations
                let anthropic_env_fields = Configuration::get_env_field_names();

                let mut conflicts = Vec::new();

                // Check system environment variables for Anthropic variables
                for field in &anthropic_env_fields {
                    if std::env::var(field).is_ok() {
                        conflicts.push(format!("system env: {}", field));
                    }
                }

                // Check env field in settings.json for Anthropic variables
                for field in &anthropic_env_fields {
                    if self.env.contains_key(*field) {
                        conflicts.push(format!("settings.json env field: {}", field));
                    }
                }

                // If conflicts found, report error and exit
                if !conflicts.is_empty() {
                    eprintln!("❌ Conflict detected in config mode:");
                    eprintln!("   Found existing Anthropic configuration:");
                    for conflict in &conflicts {
                        eprintln!("   - {}", conflict);
                    }
                    eprintln!();
                    eprintln!(
                        "   Config mode cannot work when Anthropic environment variables are already set."
                    );
                    eprintln!("   Please:");
                    eprintln!("   1. Unset system environment variables, or");
                    eprintln!("   2. Use 'env' mode instead, or");
                    eprintln!("   3. Manually clean settings.json");
                    return Err(anyhow::anyhow!(
                        "Config mode conflict: Anthropic environment variables already exist"
                    ));
                }

                // Apply the new configuration to env field
                self.switch_to_config(config);

                // Add the additional fields that switch_to_config doesn't handle
                if let Some(max_thinking_tokens) = config.max_thinking_tokens {
                    self.env.insert(
                        "ANTHROPIC_MAX_THINKING_TOKENS".to_string(),
                        max_thinking_tokens.to_string(),
                    );
                }

                if let Some(timeout) = config.api_timeout_ms {
                    self.env
                        .insert("API_TIMEOUT_MS".to_string(), timeout.to_string());
                }

                if let Some(flag) = config.claude_code_disable_nonessential_traffic {
                    self.env.insert(
                        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
                        flag.to_string(),
                    );
                }

                if let Some(model) = &config.anthropic_default_sonnet_model
                    && !model.is_empty()
                {
                    self.env
                        .insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), model.clone());
                }

                if let Some(model) = &config.anthropic_default_opus_model
                    && !model.is_empty()
                {
                    self.env
                        .insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), model.clone());
                }

                if let Some(model) = &config.anthropic_default_haiku_model
                    && !model.is_empty()
                {
                    self.env
                        .insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), model.clone());
                }

                self.save(custom_dir)?;
            }
        }

        Ok(())
    }
}
