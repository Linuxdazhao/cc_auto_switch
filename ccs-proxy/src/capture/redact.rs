//! Default redaction of secret-bearing headers and JSON body keys.

use serde_json::Value;
use std::collections::BTreeMap;

const SECRET_HEADER_NAMES: &[&str] = &[
    "authorization",
    "x-api-key",
    "anthropic-api-key",
    "cookie",
    "set-cookie",
];

const SECRET_BODY_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "auth_token",
    "authtoken",
    "access_token",
    "accesstoken",
    "token",
    "secret",
    "password",
];

const PLACEHOLDER: &str = "<redacted>";

pub fn redact_headers(headers: &mut BTreeMap<String, String>) {
    for (k, v) in headers.iter_mut() {
        if SECRET_HEADER_NAMES
            .iter()
            .any(|name| k.eq_ignore_ascii_case(name))
        {
            *v = PLACEHOLDER.to_string();
        }
    }
}

pub fn redact_body(body: &mut Value) {
    if let Value::Object(map) = body {
        for (k, v) in map.iter_mut() {
            if SECRET_BODY_KEYS
                .iter()
                .any(|secret| k.eq_ignore_ascii_case(secret))
            {
                *v = Value::String(PLACEHOLDER.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_known_headers_case_insensitive() {
        let mut h: BTreeMap<String, String> = [
            ("Authorization".into(), "Bearer sk-abc".into()),
            ("X-Api-Key".into(), "sk-xyz".into()),
            ("content-type".into(), "application/json".into()),
        ]
        .into();
        redact_headers(&mut h);
        assert_eq!(h["Authorization"], "<redacted>");
        assert_eq!(h["X-Api-Key"], "<redacted>");
        assert_eq!(h["content-type"], "application/json");
    }

    #[test]
    fn redacts_top_level_body_keys() {
        let mut body: Value = serde_json::json!({
            "api_key": "secret",
            "model": "claude-sonnet-4-6",
            "nested": { "api_key": "should_not_be_touched" }
        });
        redact_body(&mut body);
        assert_eq!(body["api_key"], "<redacted>");
        assert_eq!(body["model"], "claude-sonnet-4-6");
        assert_eq!(body["nested"]["api_key"], "should_not_be_touched");
    }
}
