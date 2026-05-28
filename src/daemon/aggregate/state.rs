use crate::config::types::ConfigStorage;
use crate::daemon::aggregate::stream::TaggedCaptureEvent;
use ccs_proxy::store::FsStore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AliasMap {
    map: HashMap<String, Vec<String>>,
}

impl AliasMap {
    pub fn from_storage(storage: &ConfigStorage) -> Self {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for config in storage.configurations.values() {
            if !config.url.is_empty() {
                map.entry(config.url.clone())
                    .or_default()
                    .push(config.alias_name.clone());
            }
        }
        Self { map }
    }

    pub fn from_entries(entries: Vec<(String, Vec<String>)>) -> Self {
        Self {
            map: entries.into_iter().collect(),
        }
    }

    pub fn aliases_for(&self, upstream: &str) -> Vec<String> {
        self.map.get(upstream).cloned().unwrap_or_default()
    }
}

pub struct AggregateState {
    pub stores: Vec<(String, Arc<FsStore>)>,
    pub merged_events: broadcast::Sender<TaggedCaptureEvent>,
    pub alias_map: Arc<AliasMap>,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{ConfigStorage, Configuration};
    use std::collections::BTreeMap;

    fn make_storage(entries: &[(&str, &str)]) -> ConfigStorage {
        let mut configurations = BTreeMap::new();
        for (alias, url) in entries {
            configurations.insert(
                alias.to_string(),
                Configuration {
                    alias_name: alias.to_string(),
                    token: "sk-test".to_string(),
                    url: url.to_string(),
                    ..Default::default()
                },
            );
        }
        ConfigStorage {
            configurations,
            claude_settings_dir: None,
            default_storage_mode: None,
            codex_configurations: None,
        }
    }

    #[test]
    fn alias_map_groups_by_upstream() {
        let storage = make_storage(&[
            ("work", "https://api.anthropic.com"),
            ("personal", "https://api.anthropic.com"),
            ("other", "https://other.example.com"),
        ]);
        let map = AliasMap::from_storage(&storage);
        let mut aliases = map.aliases_for("https://api.anthropic.com");
        aliases.sort();
        assert_eq!(aliases, vec!["personal", "work"]);
        assert_eq!(map.aliases_for("https://other.example.com"), vec!["other"]);
    }

    #[test]
    fn alias_map_returns_empty_for_unknown() {
        let storage = make_storage(&[("work", "https://api.anthropic.com")]);
        let map = AliasMap::from_storage(&storage);
        assert!(map.aliases_for("https://unknown.example.com").is_empty());
    }

    #[test]
    fn alias_map_from_entries() {
        let map = AliasMap::from_entries(vec![(
            "https://a.example.com".to_string(),
            vec!["alias_a".to_string()],
        )]);
        assert_eq!(map.aliases_for("https://a.example.com"), vec!["alias_a"]);
        assert!(map.aliases_for("https://unknown.com").is_empty());
    }
}
