use ccs_proxy::ProviderKind;

#[test]
fn provider_kind_serializes_to_lowercase_string() {
    assert_eq!(serde_json::to_string(&ProviderKind::Claude).unwrap(), "\"claude\"");
    assert_eq!(serde_json::to_string(&ProviderKind::Codex).unwrap(), "\"codex\"");
}

#[test]
fn provider_kind_parses_from_str() {
    assert_eq!("claude".parse::<ProviderKind>().unwrap(), ProviderKind::Claude);
    assert_eq!("codex".parse::<ProviderKind>().unwrap(), ProviderKind::Codex);
    assert!("kimi".parse::<ProviderKind>().is_err());
}
