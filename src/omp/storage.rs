use crate::config::ConfigStorage;
use crate::omp::OmpConfiguration;
use std::collections::BTreeMap;

impl ConfigStorage {
    /// Add an OMP configuration to storage
    ///
    /// Overwrites existing configuration with the same alias.
    pub fn add_omp_configuration(&mut self, config: OmpConfiguration) {
        self.omp_configurations
            .get_or_insert_with(BTreeMap::new)
            .insert(config.alias_name.clone(), config);
    }

    /// Get an OMP configuration by alias name
    pub fn get_omp_configuration(&self, alias_name: &str) -> Option<&OmpConfiguration> {
        self.omp_configurations.as_ref()?.get(alias_name)
    }

    /// Remove an OMP configuration by alias name
    ///
    /// Returns `true` if a configuration was found and removed, `false` otherwise.
    pub fn remove_omp_configuration(&mut self, alias_name: &str) -> bool {
        if let Some(ref mut map) = self.omp_configurations {
            map.remove(alias_name).is_some()
        } else {
            false
        }
    }

    /// Update an OMP configuration by old alias name
    ///
    /// If the alias changed, removes the old entry and inserts the new one.
    /// Returns error if the old configuration doesn't exist.
    pub fn update_omp_configuration(
        &mut self,
        old_alias: &str,
        new_config: OmpConfiguration,
    ) -> anyhow::Result<()> {
        let map = self
            .omp_configurations
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No OMP configurations in storage"))?;

        if !map.contains_key(old_alias) {
            anyhow::bail!("OMP configuration '{}' not found", old_alias);
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

    fn make_config(alias: &str) -> OmpConfiguration {
        OmpConfiguration {
            alias_name: alias.to_string(),
            openai_api_key: Some("sk-test".to_string()),
            base_url: None,
            slow_model: None,
            smol_model: None,
            plan_model: None,
        }
    }

    #[test]
    fn test_add_omp_configuration() {
        let mut storage = ConfigStorage::default();
        assert!(storage.omp_configurations.is_none());

        let config = make_config("test");
        storage.add_omp_configuration(config.clone());

        assert!(storage.omp_configurations.is_some());
        assert_eq!(storage.omp_configurations.as_ref().unwrap().len(), 1);
        assert_eq!(
            storage.omp_configurations.as_ref().unwrap().get("test"),
            Some(&config)
        );
    }

    #[test]
    fn test_add_overwrites_existing() {
        let mut storage = ConfigStorage::default();
        storage.add_omp_configuration(make_config("test"));

        let mut updated = make_config("test");
        updated.openai_api_key = Some("sk-updated".to_string());
        storage.add_omp_configuration(updated.clone());

        assert_eq!(storage.omp_configurations.as_ref().unwrap().len(), 1);
        assert_eq!(
            storage
                .get_omp_configuration("test")
                .unwrap()
                .openai_api_key,
            Some("sk-updated".to_string())
        );
    }

    #[test]
    fn test_get_omp_configuration() {
        let mut storage = ConfigStorage::default();
        let config = make_config("foo");
        storage.add_omp_configuration(config.clone());

        assert_eq!(storage.get_omp_configuration("foo"), Some(&config));
        assert_eq!(storage.get_omp_configuration("bar"), None);
    }

    #[test]
    fn test_get_omp_configuration_empty_storage() {
        let storage = ConfigStorage::default();
        assert_eq!(storage.get_omp_configuration("any"), None);
    }

    #[test]
    fn test_remove_omp_configuration() {
        let mut storage = ConfigStorage::default();
        storage.add_omp_configuration(make_config("test"));

        assert!(storage.remove_omp_configuration("test"));
        assert_eq!(storage.get_omp_configuration("test"), None);
        assert!(storage.omp_configurations.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_remove_nonexistent_omp_configuration() {
        let mut storage = ConfigStorage::default();
        storage.add_omp_configuration(make_config("test"));

        assert!(!storage.remove_omp_configuration("nonexistent"));
        assert_eq!(storage.omp_configurations.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_remove_from_empty_storage() {
        let mut storage = ConfigStorage::default();
        assert!(!storage.remove_omp_configuration("test"));
    }

    #[test]
    fn test_update_omp_configuration() {
        let mut storage = ConfigStorage::default();
        storage.add_omp_configuration(make_config("old"));

        let mut updated = make_config("new");
        updated.slow_model = Some("claude-3-5-sonnet".to_string());
        storage.update_omp_configuration("old", updated).unwrap();

        assert_eq!(storage.get_omp_configuration("old"), None);
        let new_config = storage.get_omp_configuration("new").unwrap();
        assert_eq!(new_config.slow_model, Some("claude-3-5-sonnet".to_string()));
    }

    #[test]
    fn test_update_nonexistent_omp_configuration() {
        let mut storage = ConfigStorage::default();
        storage.add_omp_configuration(make_config("test"));

        let result = storage.update_omp_configuration("nonexistent", make_config("new"));
        assert!(result.is_err());
    }
}
