//! Pidfile primitives and process-liveness checks for the daemon supervisor.
//!
//! The pidfile is an atomic single-writer lock. `acquire` uses `O_CREAT|O_EXCL`
//! so two concurrent daemons racing for the same upstream cannot both win. The
//! caller is responsible for preflight: if a stale pidfile exists for a dead
//! PID, it must be removed *before* `acquire` is called.

use anyhow::{Context, Result};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;

pub struct Pidfile {
    path: PathBuf,
}

impl Pidfile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Atomically create the pidfile and write our PID into it. Fails if the
    /// file already exists (intentional — caller must preflight stale files).
    pub fn acquire(&self) -> Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = options
            .open(&self.path)
            .with_context(|| format!("failed to create pidfile at {}", self.path.display()))?;
        let pid = std::process::id();
        file.write_all(format!("{pid}\n").as_bytes())
            .with_context(|| format!("failed to write pid to {}", self.path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to fsync pidfile {}", self.path.display()))?;
        Ok(())
    }

    /// Best-effort removal. Returns Ok if the file was already missing.
    pub fn release(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err)
                .with_context(|| format!("failed to remove pidfile {}", self.path.display())),
        }
    }

    /// Returns the PID stored in the pidfile, or `Ok(None)` if the file is
    /// missing. Unparseable contents return `Err`.
    pub fn read(&self) -> Result<Option<u32>> {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to read pidfile {}", self.path.display()));
            }
        };
        let trimmed = raw.trim();
        trimmed.parse::<u32>().map(Some).map_err(|err| {
            anyhow::anyhow!(
                "pidfile {} contains unparseable content {:?}: {err}",
                self.path.display(),
                trimmed
            )
        })
    }
}

/// Returns true if a process with this PID is running and visible to us.
///
/// Unix: `kill(pid, 0)` — 0 means alive, ESRCH means gone, EPERM means alive
/// but owned by another uid (still "alive" for our purposes).
///
/// Non-Unix builds always return `Ok(false)` — the daemon command path is
/// gated behind `#[cfg(unix)]`, so this branch is unreachable in practice but
/// keeps the module compilable on Windows.
#[cfg(unix)]
pub fn process_alive(pid: u32) -> Result<bool> {
    // SAFETY: kill(pid, 0) is signal-free; it only performs the permission /
    // existence check and never delivers a signal.
    let ret = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if ret == 0 {
        return Ok(true);
    }
    let err = std::io::Error::last_os_error();
    match err.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err(err).with_context(|| format!("kill({pid}, 0) failed")),
    }
}

#[cfg(not(unix))]
pub fn process_alive(_pid: u32) -> Result<bool> {
    Ok(false)
}

/// Returns the executable name for the PID if it can be determined cheaply.
///
/// Linux: reads `/proc/<pid>/comm`.
/// Other Unix (macOS, BSDs): shells out to `ps -p <pid> -o comm=`, then takes
/// the basename — macOS `ps` returns the full path of the executable.
/// Non-Unix: returns `None`.
///
/// Used by the daemon preflight to tell "our prior daemon process" apart from
/// "PID was recycled and now belongs to /usr/bin/grep" or similar.
#[cfg(target_os = "linux")]
pub fn process_name(pid: u32) -> Option<String> {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
pub fn process_name(pid: u32) -> Option<String> {
    use std::process::Command;
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }
    let basename = std::path::Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed)
        .to_string();
    Some(basename)
}

#[cfg(not(unix))]
pub fn process_name(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::{Pidfile, process_alive};
    use tempfile::TempDir;

    fn make_path() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("ccs-proxy.pid");
        (dir, path)
    }

    #[test]
    fn acquire_writes_pid_to_file() {
        let (_dir, path) = make_path();
        let pidfile = Pidfile::new(path.clone());
        pidfile.acquire().expect("acquire");

        let raw = std::fs::read_to_string(&path).expect("read pidfile");
        let parsed: u32 = raw.trim().parse().expect("parse pid");
        assert_eq!(parsed, std::process::id());
    }

    #[test]
    fn acquire_errors_when_file_already_exists() {
        let (_dir, path) = make_path();
        let pidfile = Pidfile::new(path);
        pidfile.acquire().expect("first acquire");
        let second = pidfile.acquire();
        assert!(second.is_err(), "second acquire must fail");
    }

    #[test]
    fn release_removes_file() {
        let (_dir, path) = make_path();
        let pidfile = Pidfile::new(path.clone());
        pidfile.acquire().expect("acquire");
        pidfile.release().expect("release");
        assert!(!path.exists(), "pidfile should be gone after release");
        pidfile.release().expect("release is idempotent");
    }

    #[test]
    fn read_missing_file_returns_none() {
        let (_dir, path) = make_path();
        let pidfile = Pidfile::new(path);
        assert!(pidfile.read().expect("read").is_none());
    }

    #[test]
    fn read_unparseable_returns_err() {
        let (_dir, path) = make_path();
        std::fs::write(&path, "hello\n").expect("write garbage");
        let pidfile = Pidfile::new(path);
        assert!(pidfile.read().is_err());
    }

    #[test]
    fn read_returns_pid_for_valid_file() {
        let (_dir, path) = make_path();
        let pidfile = Pidfile::new(path);
        pidfile.acquire().expect("acquire");
        let pid = pidfile.read().expect("read");
        assert_eq!(pid, Some(std::process::id()));
    }

    #[cfg(unix)]
    #[test]
    fn process_alive_for_self() {
        let alive = process_alive(std::process::id()).expect("query self");
        assert!(alive);
    }

    #[cfg(unix)]
    #[test]
    fn process_alive_for_pid_1() {
        // PID 1 is init/launchd; it always exists on Unix and is owned by
        // root, so kill(1, 0) from a non-root caller returns EPERM. That
        // still counts as "alive" — exercises the EPERM branch.
        let alive = process_alive(1).expect("query pid 1");
        assert!(alive, "PID 1 must be reported alive on Unix");
    }

    #[test]
    fn process_alive_for_high_unused_pid() {
        let alive = process_alive(0xFFFF_FFFE).expect("query unused pid");
        assert!(!alive);
    }

    #[cfg(unix)]
    #[test]
    fn acquire_sets_unix_0600_perms() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, path) = make_path();
        let pidfile = Pidfile::new(path.clone());
        pidfile.acquire().expect("acquire");
        let meta = std::fs::metadata(&path).expect("stat");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "pidfile must be 0600, got {mode:o}");
    }
}
