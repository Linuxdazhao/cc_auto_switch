use ccs_proxy::SessionId;

#[test]
fn session_id_round_trips_to_string() {
    let id = SessionId::new();
    let s = id.to_string();
    let parsed: SessionId = s.parse().unwrap();
    assert_eq!(id, parsed);
}

#[test]
fn session_id_format_is_iso_plus_hash() {
    // Example: 2026-05-28T17-12-34-000Z-a1b2c3d4
    let s = SessionId::new().to_string();
    assert!(s.len() > 25, "got: {s}");
    let parts: Vec<&str> = s.rsplitn(2, '-').collect();
    assert_eq!(parts[0].len(), 8, "8-char random suffix expected, got: {s}");
}
