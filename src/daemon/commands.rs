use crate::config::ConfigStorage;
use crate::daemon::lifecycle::LifecycleConfig;
use crate::daemon::pidfile::{Pidfile, process_alive};
use crate::daemon::state::DaemonState;
use anyhow::{Context, Result};

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
    match action {
        DaemonAction::Start { foreground } => handle_start(foreground, storage),
        DaemonAction::Stop => handle_stop(),
        DaemonAction::Status { json } => handle_status(json, storage),
        DaemonAction::Restart { foreground } => {
            let _ = handle_stop();
            handle_start(foreground, storage)
        }
    }
}

#[cfg(unix)]
fn handle_start(foreground: bool, storage: &ConfigStorage) -> Result<()> {
    let cfg = LifecycleConfig::from_storage(storage)?;

    // Preflight: check for existing pidfile.
    let pidfile = Pidfile::new(cfg.pidfile_path.clone());
    if let Some(pid) = pidfile.read()? {
        if process_alive(pid)? {
            anyhow::bail!("daemon already running (PID {pid}). Use `cc-switch daemon stop` first.");
        }
        eprintln!("warning: stale pidfile for dead PID {pid} — removing");
        pidfile.release()?;
    }

    if !foreground {
        let home = dirs::home_dir().context("could not find home directory")?;
        let log_path = home.join(".cc-switch").join("daemon.log");
        crate::daemon::fork::double_fork_into_background(&log_path)?;
    }

    crate::daemon::lifecycle::run_daemon_blocking(cfg)
}

#[cfg(unix)]
fn handle_stop() -> Result<()> {
    let home = dirs::home_dir().context("could not find home directory")?;
    let pidfile_path = home.join(".cc-switch").join("daemon.pid");
    let pidfile = Pidfile::new(pidfile_path);

    let pid = match pidfile.read()? {
        Some(pid) => pid,
        None => {
            eprintln!("daemon not running (no pidfile)");
            return Ok(());
        }
    };

    if !process_alive(pid)? {
        eprintln!("daemon not running (stale pidfile for PID {pid}) — cleaning up");
        pidfile.release()?;
        return Ok(());
    }

    // Send SIGTERM.
    let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            eprintln!("daemon not running (PID {pid} gone) — cleaning up");
            pidfile.release()?;
            return Ok(());
        }
        return Err(err).with_context(|| format!("failed to send SIGTERM to PID {pid}"));
    }

    // Poll for exit (up to 5 seconds).
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !process_alive(pid)? {
            eprintln!("daemon stopped (PID {pid})");
            return Ok(());
        }
    }

    // Force kill after timeout.
    eprintln!("warning: daemon PID {pid} did not exit after 5s — sending SIGKILL");
    unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    std::thread::sleep(std::time::Duration::from_millis(200));
    pidfile.release()?;
    eprintln!("daemon killed");
    Ok(())
}

#[cfg(unix)]
fn handle_status(json: bool, storage: &ConfigStorage) -> Result<()> {
    let home = dirs::home_dir().context("could not find home directory")?;
    let cc_switch_dir = home.join(".cc-switch");
    let pidfile_path = cc_switch_dir.join("daemon.pid");
    let state_path = cc_switch_dir.join("daemon-state.json");

    let pidfile = Pidfile::new(pidfile_path);
    let pid = match pidfile.read()? {
        Some(pid) => pid,
        None => {
            if json {
                println!("{{\"status\":\"stopped\"}}");
            } else {
                println!("ccs-daemon: STOPPED (no pidfile)");
            }
            return Ok(());
        }
    };

    if !process_alive(pid)? {
        if json {
            println!("{{\"status\":\"stopped\",\"stale_pid\":{pid}}}");
        } else {
            println!("ccs-daemon: STOPPED (stale pidfile, PID {pid} is dead)");
        }
        return Ok(());
    }

    let state = match DaemonState::load(&state_path)? {
        Some(s) => s,
        None => {
            if json {
                println!("{{\"status\":\"running\",\"pid\":{pid},\"proxies\":[]}}");
            } else {
                println!("ccs-daemon: RUNNING (pid {pid}) — no state file");
            }
            return Ok(());
        }
    };

    let aliases_by_upstream = crate::daemon::status::build_aliases_by_upstream(storage);
    let statuses = crate::daemon::status::collect_status(&state);

    if json {
        let output = crate::daemon::status::format_status_json(&state, &statuses);
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let text =
            crate::daemon::status::format_status_text(&state, &statuses, &aliases_by_upstream);
        print!("{text}");
    }

    Ok(())
}
