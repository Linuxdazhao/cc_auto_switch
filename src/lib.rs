//! cc-switch: A CLI tool for managing multiple Claude API configurations
//!
//! This library provides functionality to store, manage, and switch between
//! multiple Claude API configurations.

pub mod cli;
pub mod config;
pub mod interactive;

pub mod claude_settings;
pub mod utils;

// Re-export commonly used types and functions for easier importing
pub use crate::cli::completion::{
    generate_aliases, generate_completion, list_aliases_for_completion,
};
pub use crate::cli::main::handle_switch_command;
pub use crate::cli::main::run;
