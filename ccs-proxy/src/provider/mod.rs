use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ProviderKind {
    Claude,
    Codex,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown provider `{0}` (supported: claude, codex)")]
pub struct UnknownProvider(pub String);

impl FromStr for ProviderKind {
    type Err = UnknownProvider;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            other => Err(UnknownProvider(other.to_string())),
        }
    }
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}
