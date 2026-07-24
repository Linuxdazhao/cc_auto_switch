pub mod codex_interactive;
pub mod grok_interactive;
#[allow(clippy::module_inception)]
pub mod interactive;
pub mod omp_interactive;

// Re-export functions for convenience
pub use crate::interactive::codex_interactive::handle_codex_interactive_selection;
pub use crate::interactive::grok_interactive::handle_grok_interactive_selection;
pub use crate::interactive::interactive::{
    handle_current_command, handle_interactive_selection, launch_claude_with_env, read_input,
    read_sensitive_input,
};
pub use crate::interactive::omp_interactive::handle_omp_interactive_selection;
