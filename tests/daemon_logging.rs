//! Tests for daemon logging: level resolution and log cleanup.

mod daemon_logging {
    use cc_switch::daemon::logging::resolve_log_level;
    use tracing::level_filters::LevelFilter;

    #[test]
    fn default_level_is_info() {
        let level = resolve_log_level(None, 0, None);
        assert_eq!(level, LevelFilter::INFO);
    }

    #[test]
    fn verbose_1_is_info() {
        let level = resolve_log_level(None, 1, None);
        assert_eq!(level, LevelFilter::INFO);
    }

    #[test]
    fn verbose_2_is_debug() {
        let level = resolve_log_level(None, 2, None);
        assert_eq!(level, LevelFilter::DEBUG);
    }

    #[test]
    fn verbose_3_is_trace() {
        let level = resolve_log_level(None, 3, None);
        assert_eq!(level, LevelFilter::TRACE);
    }

    #[test]
    fn cli_flag_overrides_verbose() {
        let level = resolve_log_level(Some("error"), 3, None);
        assert_eq!(level, LevelFilter::ERROR);
    }

    #[test]
    fn env_overrides_cli_flag() {
        let level = resolve_log_level(Some("error"), 0, Some("trace"));
        assert_eq!(level, LevelFilter::TRACE);
    }

    #[test]
    fn env_invalid_falls_back_to_cli() {
        let level = resolve_log_level(Some("warn"), 0, Some("not_a_level"));
        assert_eq!(level, LevelFilter::WARN);
    }
}
