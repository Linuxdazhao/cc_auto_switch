use crate::config::ConfigStorage;
use anyhow::Result;

pub enum DaemonAction {
    Start { foreground: bool },
    Stop,
    Status { json: bool },
    Restart { foreground: bool },
}

pub fn handle_daemon_command(action: DaemonAction, storage: &ConfigStorage) -> Result<()> {
    #[cfg(not(unix))]
    {
        let _ = (action, storage);
        anyhow::bail!("cc daemon is Unix-only in v1 — run `ccs-proxy serve` directly");
    }

    #[cfg(unix)]
    {
        let _ = storage;
        match action {
            DaemonAction::Start { .. } => unimplemented!("Task 9: cc daemon start"),
            DaemonAction::Stop => unimplemented!("Task 9: cc daemon stop"),
            DaemonAction::Status { .. } => unimplemented!("Task 10: cc daemon status"),
            DaemonAction::Restart { .. } => unimplemented!("Task 9: cc daemon restart"),
        }
    }
}
