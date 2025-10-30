#[allow(clippy::module_inception)]
pub mod interactive;

// Re-export functions for convenience
pub use crate::interactive::interactive::{
    handle_current_command, handle_interactive_selection, read_input, read_sensitive_input,
};
