//! ccs-proxy: a local logging reverse-proxy + dashboard that captures the
//! traffic between Claude Code / Codex and their upstream LLM APIs.

pub mod capture;
mod config;
mod error;
mod handle;
pub mod provider;
mod session;

pub use config::ServeConfig;
pub use error::ServeError;
pub use handle::ProxyHandle;
pub use provider::ProviderKind;
pub use session::SessionId;
