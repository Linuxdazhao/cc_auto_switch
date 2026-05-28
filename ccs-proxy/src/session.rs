use chrono::Utc;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub(crate) String);

impl SessionId {
    pub fn new() -> Self {
        let ts = Utc::now().format("%Y-%m-%dT%H-%M-%S-%3fZ").to_string();
        let suffix: String = uuid::Uuid::new_v4()
            .to_string()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(8)
            .collect();
        Self(format!("{ts}-{suffix}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid session id: {0}")]
pub struct InvalidSessionId(pub String);

impl FromStr for SessionId {
    type Err = InvalidSessionId;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty()
            || s.len() > 64
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == 'T' || c == 'Z')
        {
            return Err(InvalidSessionId(s.to_string()));
        }
        Ok(Self(s.to_string()))
    }
}
