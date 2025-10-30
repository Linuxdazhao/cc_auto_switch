#[allow(clippy::module_inception)]
pub mod config;
pub mod config_storage;
pub mod types;

// Re-export types for convenience
pub use crate::config::config::{EnvironmentConfig, get_config_storage_path, validate_alias_name};
pub use crate::config::types::{AddCommandParams, ClaudeSettings, ConfigStorage, Configuration};
