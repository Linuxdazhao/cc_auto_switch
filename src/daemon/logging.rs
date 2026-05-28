use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub enum LogMode {
    Foreground,
    Background,
}

/// Resolve the effective log level from (highest priority first):
/// 1. `env_val` — CCS_LOG env var (full EnvFilter syntax)
/// 2. `cli_level` — --log-level flag (single level string)
/// 3. `verbose` — -v count: 0-1=info, 2=debug, 3+=trace
/// 4. Default: info
pub fn resolve_log_level(
    cli_level: Option<&str>,
    verbose: u8,
    env_val: Option<&str>,
) -> LevelFilter {
    if let Some(env) = env_val
        && let Ok(lf) = env.parse::<LevelFilter>()
    {
        return lf;
    }

    if let Some(flag) = cli_level
        && let Ok(lf) = flag.parse::<LevelFilter>()
    {
        return lf;
    }

    match verbose {
        0 | 1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

/// Initialize the global tracing subscriber. Returns a guard that must be held
/// for the daemon's lifetime to ensure the non-blocking writer flushes on drop.
pub fn init_tracing(mode: LogMode, level: LevelFilter) -> WorkerGuard {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "daemon");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    match mode {
        LogMode::Foreground => {
            let stderr_layer = tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .with_target(true);
            registry.with(stderr_layer).init();
        }
        LogMode::Background => {
            registry
                .with(None::<tracing_subscriber::fmt::Layer<_>>)
                .init();
        }
    }

    guard
}

/// Clean up log files older than `retention_days` in the log directory.
pub fn cleanup_old_logs(retention_days: u32) {
    let log_dir = log_directory();
    cleanup_old_logs_in_dir(&log_dir, retention_days);
}

fn cleanup_old_logs_in_dir(log_dir: &std::path::Path, retention_days: u32) {
    let entries = match std::fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let today = chrono::Utc::now().date_naive();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // tracing-appender daily files: "daemon.YYYY-MM-DD"
        let date_part = match name_str.strip_prefix("daemon.") {
            Some(rest) => rest,
            None => continue,
        };

        let file_date = match chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let age_days = (today - file_date).num_days();
        if age_days > retention_days as i64 {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn log_directory() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".cc-switch")
        .join("logs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_removes_old_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let old_file = dir.path().join("daemon.2020-01-01");
        let recent_file = dir.path().join("daemon.2026-05-27");
        let unrelated = dir.path().join("other.txt");

        std::fs::write(&old_file, "old").unwrap();
        std::fs::write(&recent_file, "recent").unwrap();
        std::fs::write(&unrelated, "keep").unwrap();

        cleanup_old_logs_in_dir(dir.path(), 7);

        assert!(!old_file.exists(), "old file should be deleted");
        assert!(recent_file.exists(), "recent file should be kept");
        assert!(unrelated.exists(), "unrelated file should be kept");
    }

    #[test]
    fn resolve_level_default_is_info() {
        assert_eq!(resolve_log_level(None, 0, None), LevelFilter::INFO);
    }

    #[test]
    fn resolve_level_verbose_2_is_debug() {
        assert_eq!(resolve_log_level(None, 2, None), LevelFilter::DEBUG);
    }

    #[test]
    fn resolve_level_verbose_3_is_trace() {
        assert_eq!(resolve_log_level(None, 3, None), LevelFilter::TRACE);
    }

    #[test]
    fn resolve_level_cli_overrides_verbose() {
        assert_eq!(
            resolve_log_level(Some("error"), 3, None),
            LevelFilter::ERROR
        );
    }

    #[test]
    fn resolve_level_env_overrides_all() {
        assert_eq!(
            resolve_log_level(Some("error"), 0, Some("trace")),
            LevelFilter::TRACE
        );
    }

    #[test]
    fn resolve_level_invalid_env_falls_back() {
        assert_eq!(
            resolve_log_level(Some("warn"), 0, Some("not_a_level")),
            LevelFilter::WARN
        );
    }
}
