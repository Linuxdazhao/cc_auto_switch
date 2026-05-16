use crate::codex::CodexConfiguration;
use crate::config::ConfigStorage;
use std::collections::BTreeMap;

impl ConfigStorage {
    /// Add a Codex configuration to storage
    ///
    /// Overwrites existing configuration with the same alias.
    pub fn add_codex_configuration(&mut self, config: CodexConfiguration) {
        self.codex_configurations
            .get_or_insert_with(BTreeMap::new)
            .insert(config.alias_name.clone(), config);
    }

    /// Get a Codex configuration by alias name
    pub fn get_codex_configuration(&self, alias_name: &str) -> Option<&CodexConfiguration> {
        self.codex_configurations.as_ref()?.get(alias_name)
    }

    /// Remove a Codex configuration by alias name
    ///
    /// Returns `true` if a configuration was found and removed, `false` otherwise.
    pub fn remove_codex_configuration(&mut self, alias_name: &str) -> bool {
        if let Some(ref mut map) = self.codex_configurations {
            map.remove(alias_name).is_some()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_apikey_config(alias: &str) -> CodexConfiguration {
        CodexConfiguration {
            alias_name: alias.to_string(),
            auth_mode: "apikey".to_string(),
            openai_api_key: Some(format!("sk-{alias}")),
            id_token: None,
            access_token: None,
            refresh_token: None,
            account_id: None,
            last_refresh: None,
        }
    }

    #[test]
    fn test_add_codex_configuration() {
        let mut storage = ConfigStorage::default();
        assert!(storage.codex_configurations.is_none());

        let config = make_apikey_config("test");
        storage.add_codex_configuration(config.clone());

        assert!(storage.codex_configurations.is_some());
        assert_eq!(storage.codex_configurations.as_ref().unwrap().len(), 1);
        assert_eq!(
            storage.codex_configurations.as_ref().unwrap().get("test"),
            Some(&config)
        );
    }

    #[test]
    fn test_add_overwrites_existing() {
        let mut storage = ConfigStorage::default();
        storage.add_codex_configuration(make_apikey_config("test"));

        let mut updated = make_apikey_config("test");
        updated.openai_api_key = Some("sk-updated".to_string());
        storage.add_codex_configuration(updated.clone());

        assert_eq!(storage.codex_configurations.as_ref().unwrap().len(), 1);
        assert_eq!(
            storage
                .get_codex_configuration("test")
                .unwrap()
                .openai_api_key,
            Some("sk-updated".to_string())
        );
    }

    #[test]
    fn test_get_codex_configuration() {
        let mut storage = ConfigStorage::default();
        let config = make_apikey_config("foo");
        storage.add_codex_configuration(config.clone());

        assert_eq!(storage.get_codex_configuration("foo"), Some(&config));
        assert_eq!(storage.get_codex_configuration("missing"), None);
    }

    #[test]
    fn test_get_codex_configuration_empty_storage() {
        let storage = ConfigStorage::default();
        assert_eq!(storage.get_codex_configuration("any"), None);
    }

    #[test]
    fn test_remove_codex_configuration() {
        let mut storage = ConfigStorage::default();
        storage.add_codex_configuration(make_apikey_config("test"));

        assert!(storage.remove_codex_configuration("test"));
        assert_eq!(storage.get_codex_configuration("test"), None);
    }

    #[test]
    fn test_remove_nonexistent_codex_configuration() {
        let mut storage = ConfigStorage::default();
        storage.add_codex_configuration(make_apikey_config("test"));

        assert!(!storage.remove_codex_configuration("missing"));
        assert_eq!(storage.codex_configurations.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_remove_from_empty_storage() {
        let mut storage = ConfigStorage::default();
        assert!(!storage.remove_codex_configuration("any"));
    }
}
