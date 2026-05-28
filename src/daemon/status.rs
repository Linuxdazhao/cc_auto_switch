use crate::daemon::state::{DaemonState, ProxyEntry};
use std::collections::BTreeMap;

/// Map from upstream URL → list of alias names that resolve to that upstream.
pub type AliasesByUpstream = BTreeMap<String, Vec<String>>;

pub struct ProxyStatus {
    pub entry: ProxyEntry,
    pub reachable: bool,
    pub request_count: Option<u64>,
    pub store_degraded: bool,
}

pub fn collect_status(_state: &DaemonState) -> Vec<ProxyStatus> {
    unimplemented!("Task 10")
}
pub fn format_status_text(
    _state: &DaemonState,
    _statuses: &[ProxyStatus],
    _aliases_per_upstream: &AliasesByUpstream,
) -> String {
    unimplemented!("Task 10")
}
pub fn format_status_json(_state: &DaemonState, _statuses: &[ProxyStatus]) -> serde_json::Value {
    unimplemented!("Task 10")
}
