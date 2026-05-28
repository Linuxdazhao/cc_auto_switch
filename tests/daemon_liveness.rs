//! Liveness detection tests: pidfile alive vs stale vs corrupted, all branches in spec §8.

#[cfg(unix)]
mod daemon_liveness {
    use cc_switch::daemon::pidfile::{Pidfile, process_alive, process_name};
    use tempfile::TempDir;

    fn make_pidfile() -> (TempDir, Pidfile) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("daemon.pid");
        (dir, Pidfile::new(path))
    }

    // --- pidfile states ---

    #[test]
    fn pidfile_missing_means_daemon_not_running() {
        let (_dir, pidfile) = make_pidfile();
        assert!(pidfile.read().unwrap().is_none());
    }

    #[test]
    fn pidfile_present_with_our_pid_means_running() {
        let (_dir, pidfile) = make_pidfile();
        pidfile.acquire().unwrap();
        let pid = pidfile.read().unwrap().unwrap();
        assert_eq!(pid, std::process::id());
        assert!(process_alive(pid).unwrap());
    }

    #[test]
    fn pidfile_present_dead_pid_is_stale() {
        let (dir, _pidfile) = make_pidfile();
        let path = dir.path().join("daemon.pid");
        // Write a high PID that doesn't exist.
        std::fs::write(&path, "4294967294\n").unwrap();
        let pidfile = Pidfile::new(path);
        let pid = pidfile.read().unwrap().unwrap();
        assert_eq!(pid, 4_294_967_294);
        assert!(!process_alive(pid).unwrap());
    }

    #[test]
    fn pidfile_unparseable_is_corrupted() {
        let (dir, _pidfile) = make_pidfile();
        let path = dir.path().join("daemon.pid");
        std::fs::write(&path, "not-a-number\n").unwrap();
        let pidfile = Pidfile::new(path);
        assert!(pidfile.read().is_err());
    }

    #[test]
    fn pidfile_empty_is_corrupted() {
        let (dir, _pidfile) = make_pidfile();
        let path = dir.path().join("daemon.pid");
        std::fs::write(&path, "").unwrap();
        let pidfile = Pidfile::new(path);
        assert!(pidfile.read().is_err());
    }

    #[test]
    fn pidfile_negative_is_corrupted() {
        let (dir, _pidfile) = make_pidfile();
        let path = dir.path().join("daemon.pid");
        std::fs::write(&path, "-1\n").unwrap();
        let pidfile = Pidfile::new(path);
        assert!(pidfile.read().is_err());
    }

    // --- process_alive ---

    #[test]
    fn self_process_is_alive() {
        assert!(process_alive(std::process::id()).unwrap());
    }

    #[test]
    fn pid_1_is_alive_on_unix() {
        // PID 1 = init/launchd. kill(1, 0) returns EPERM from non-root.
        assert!(process_alive(1).unwrap());
    }

    #[test]
    fn impossibly_high_pid_is_not_alive() {
        assert!(!process_alive(0xFFFF_FFFE).unwrap());
    }

    // --- process_name ---

    #[test]
    fn process_name_of_self_is_not_empty() {
        let name = process_name(std::process::id());
        // In test harness this is typically "cc_switch-<hash>" or the test binary name.
        assert!(name.is_some(), "should be able to read own process name");
        assert!(!name.unwrap().is_empty());
    }

    #[test]
    fn process_name_of_pid_1_is_init_or_launchd() {
        let name = process_name(1);
        // On macOS it's "launchd", on Linux "systemd" or "init".
        // We just verify we get *something* back.
        assert!(name.is_some(), "PID 1 should have a readable process name");
    }

    #[test]
    fn process_name_of_dead_pid_is_none() {
        let name = process_name(0xFFFF_FFFE);
        assert!(name.is_none());
    }

    // --- acquire / release semantics ---

    #[test]
    fn acquire_then_release_allows_re_acquire() {
        let (_dir, pidfile) = make_pidfile();
        pidfile.acquire().unwrap();
        pidfile.release().unwrap();
        // Should be able to re-acquire after release.
        pidfile.acquire().unwrap();
    }

    #[test]
    fn double_acquire_fails() {
        let (_dir, pidfile) = make_pidfile();
        pidfile.acquire().unwrap();
        assert!(pidfile.acquire().is_err());
    }

    #[test]
    fn release_is_idempotent() {
        let (_dir, pidfile) = make_pidfile();
        pidfile.acquire().unwrap();
        pidfile.release().unwrap();
        pidfile.release().unwrap(); // second release is fine
    }

    #[cfg(unix)]
    #[test]
    fn pidfile_has_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, pidfile) = make_pidfile();
        pidfile.acquire().unwrap();
        let path = dir.path().join("daemon.pid");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "pidfile should be 0600, got {:o}", mode);
    }
}
