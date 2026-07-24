use crate::config::ConfigStorage;
use crate::grok::GrokConfiguration;
use std::collections::BTreeMap;

impl ConfigStorage {
    /// Add a grok configuration, overwriting any existing entry with the same alias.
    pub fn add_grok_configuration(&mut self, config: GrokConfiguration) {
        self.grok_configurations
            .get_or_insert_with(BTreeMap::new)
            .insert(config.alias_name.clone(), config);
    }

    /// Get a grok configuration by alias name.
    pub fn get_grok_configuration(&self, alias_name: &str) -> Option<&GrokConfiguration> {
        self.grok_configurations.as_ref()?.get(alias_name)
    }

    /// Remove a grok configuration by alias name.
    ///
    /// Returns `true` if a configuration was found and removed.
    pub fn remove_grok_configuration(&mut self, alias_name: &str) -> bool {
        if let Some(ref mut map) = self.grok_configurations {
            map.remove(alias_name).is_some()
        } else {
            false
        }
    }

    /// Update a grok configuration by old alias name.
    ///
    /// If the alias changed, removes the old entry and inserts the new one.
    pub fn update_grok_configuration(
        &mut self,
        old_alias: &str,
        new_config: GrokConfiguration,
    ) -> anyhow::Result<()> {
        let map = self
            .grok_configurations
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No grok configurations in storage"))?;

        if !map.contains_key(old_alias) {
            anyhow::bail!("grok configuration '{}' not found", old_alias);
        }

        if old_alias != new_config.alias_name {
            map.remove(old_alias);
        }
        map.insert(new_config.alias_name.clone(), new_config);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(alias: &str) -> GrokConfiguration {
        GrokConfiguration {
            alias_name: alias.to_string(),
            model: "claude-opus-4-6".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            name: None,
            api_backend: Some("messages".to_string()),
            context_window: None,
            api_key: None,
            env_key: Some(vec!["ANTHROPIC_API_KEY".to_string()]),
            extra_headers: None,
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut storage = ConfigStorage::default();
        assert!(storage.grok_configurations.is_none());
        let config = make_config("opus");
        storage.add_grok_configuration(config.clone());
        assert_eq!(storage.get_grok_configuration("opus"), Some(&config));
    }

    #[test]
    fn test_add_overwrites() {
        let mut storage = ConfigStorage::default();
        storage.add_grok_configuration(make_config("opus"));
        let mut updated = make_config("opus");
        updated.model = "claude-sonnet".to_string();
        storage.add_grok_configuration(updated.clone());
        assert_eq!(storage.grok_configurations.as_ref().unwrap().len(), 1);
        assert_eq!(
            storage.get_grok_configuration("opus").unwrap().model,
            "claude-sonnet"
        );
    }

    #[test]
    fn test_remove() {
        let mut storage = ConfigStorage::default();
        storage.add_grok_configuration(make_config("opus"));
        assert!(storage.remove_grok_configuration("opus"));
        assert!(!storage.remove_grok_configuration("opus"));
    }

    #[test]
    fn test_update_rename() {
        let mut storage = ConfigStorage::default();
        storage.add_grok_configuration(make_config("old"));
        storage
            .update_grok_configuration("old", make_config("new"))
            .unwrap();
        assert_eq!(storage.get_grok_configuration("old"), None);
        assert!(storage.get_grok_configuration("new").is_some());
    }

    #[test]
    fn test_update_missing_errors() {
        let mut storage = ConfigStorage::default();
        storage.add_grok_configuration(make_config("a"));
        assert!(
            storage
                .update_grok_configuration("missing", make_config("b"))
                .is_err()
        );
    }
}
