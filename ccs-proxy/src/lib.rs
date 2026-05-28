//! ccs-proxy: a local logging reverse-proxy + dashboard that captures the
//! traffic between Claude Code / Codex and their upstream LLM APIs.

mod config;
mod error;
mod handle;
pub mod provider;

pub use config::ServeConfig;
pub use error::ServeError;
pub use handle::ProxyHandle;
pub use provider::ProviderKind;
