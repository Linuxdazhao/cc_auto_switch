#[allow(clippy::module_inception)]
pub mod cli;
pub mod completion;
pub mod display_utils;
pub mod main;

// Re-export types for convenience
pub use crate::cli::cli::{Cli, Commands};
