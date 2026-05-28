use crate::config::ConfigStorage;
use crate::daemon::state::{DaemonState, ProxyEntry};
use std::collections::BTreeMap;

pub type AliasesByUpstream = BTreeMap<String, Vec<String>>;

pub struct ProxyStatus {
    pub entry: ProxyEntry,
    pub reachable: bool,
    pub request_count: Option<u64>,
    pub store_degraded: bool,
}

pub fn build_aliases_by_upstream(storage: &ConfigStorage) -> AliasesByUpstream {
    let mut map: AliasesByUpstream = BTreeMap::new();
    for config in storage.configurations.values() {
        if !config.url.is_empty() {
            map.entry(config.url.clone())
                .or_default()
                .push(config.alias_name.clone());
        }
    }
    map
}

struct HealthProbe {
    reachable: bool,
    request_count: Option<u64>,
    store_degraded: bool,
}

pub fn collect_status(state: &DaemonState) -> Vec<ProxyStatus> {
    state
        .proxies
        .iter()
        .map(|entry| {
            let probe = probe_health(entry.api_port);
            ProxyStatus {
                entry: entry.clone(),
                reachable: probe.reachable,
                request_count: probe.request_count,
                store_degraded: probe.store_degraded,
            }
        })
        .collect()
}

fn probe_health(api_port: Option<u16>) -> HealthProbe {
    let Some(port) = api_port else {
        return HealthProbe {
            reachable: false,
            request_count: None,
            store_degraded: false,
        };
    };
    let url = format!("http://127.0.0.1:{port}/api/health");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(_) => {
            return HealthProbe {
                reachable: false,
                request_count: None,
                store_degraded: false,
            };
        }
    };
    match client.get(&url).send() {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().unwrap_or_default();
            let request_count = json.get("request_count").and_then(|v| v.as_u64());
            let store_degraded = json
                .get("store_degraded")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            HealthProbe {
                reachable: true,
                request_count,
                store_degraded,
            }
        }
        _ => HealthProbe {
            reachable: false,
            request_count: None,
            store_degraded: false,
        },
    }
}

pub fn format_status_text(
    state: &DaemonState,
    statuses: &[ProxyStatus],
    aliases_per_upstream: &AliasesByUpstream,
) -> String {
    let mut out = String::new();

    let uptime = compute_uptime(&state.started_at);
    out.push_str(&format!(
        "ccs-daemon: RUNNING (pid {}, uptime {})\n",
        state.pid, uptime
    ));
    out.push_str(&format!(
        "  state: {}\n",
        state.data_root.parent().map_or_else(
            || state.data_root.display().to_string(),
            |p| p.join("daemon-state.json").display().to_string(),
        )
    ));
    out.push_str(&format!(
        "  pidfile: {}\n",
        state.data_root.parent().map_or_else(
            || "~/.cc-switch/daemon.pid".to_string(),
            |p| p.join("daemon.pid").display().to_string(),
        )
    ));
    if let Some(agg_port) = state.agg_port {
        out.push_str(&format!(
            "  dashboard: http://127.0.0.1:{agg_port}\n"
        ));
    }
    out.push('\n');

    if statuses.is_empty() {
        out.push_str("No proxies running.\n");
        return out;
    }

    out.push_str(&format!("Proxies ({}):\n", statuses.len()));

    // Table header
    out.push_str("  upstream                             proxy_port  dashboard  requests\n");
    out.push_str("  -----------------------------------  ----------  ---------  --------\n");

    for status in statuses {
        let upstream = &status.entry.upstream;
        let upstream_display = if upstream.len() > 35 {
            format!("{}...", &upstream[..32])
        } else {
            upstream.clone()
        };
        let req_str = match status.request_count {
            Some(n) => format!("{n:>8}"),
            None if !status.reachable => "(unreachable)".to_string(),
            None => "       ?".to_string(),
        };
        let dashboard_str = match status.entry.api_port {
            Some(port) => format!(":{port}"),
            None => "—".to_string(),
        };
        out.push_str(&format!(
            "  {:<35}  {:>10}  {:>9}  {}\n",
            upstream_display, status.entry.proxy_port, dashboard_str, req_str,
        ));
    }

    // Aliases section
    if !aliases_per_upstream.is_empty() {
        out.push_str("\nAliases routed through daemon:\n");
        for status in statuses {
            if let Some(aliases) = aliases_per_upstream.get(&status.entry.upstream) {
                let alias_list = aliases.join(", ");
                let upstream_short = if status.entry.upstream.len() > 40 {
                    format!("{}...", &status.entry.upstream[..37])
                } else {
                    status.entry.upstream.clone()
                };
                out.push_str(&format!("  {:<20} → {}\n", alias_list, upstream_short));
            }
        }
    }

    out
}

pub fn format_status_json(state: &DaemonState, statuses: &[ProxyStatus]) -> serde_json::Value {
    let proxies: Vec<serde_json::Value> = statuses
        .iter()
        .map(|s| {
            serde_json::json!({
                "provider": s.entry.provider,
                "upstream": s.entry.upstream,
                "proxy_port": s.entry.proxy_port,
                "api_port": s.entry.api_port,
                "data_dir": s.entry.data_dir.display().to_string(),
                "reachable": s.reachable,
                "request_count": s.request_count,
                "store_degraded": s.store_degraded,
                "restart_count": s.entry.restart_count,
            })
        })
        .collect();

    serde_json::json!({
        "status": "running",
        "pid": state.pid,
        "started_at": state.started_at,
        "data_root": state.data_root.display().to_string(),
        "agg_port": state.agg_port,
        "proxies": proxies,
    })
}

fn compute_uptime(started_at: &str) -> String {
    let started = match chrono::DateTime::parse_from_rfc3339(started_at) {
        Ok(dt) => dt,
        Err(_) => return "?".to_string(),
    };
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(started);
    let secs = duration.num_seconds();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{hours}h {mins:02}m")
    }
}
