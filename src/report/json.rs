//! Canonical JSON output. Stable key order, pretty or compact.

use crate::model::Report;

pub fn to_json(r: &Report, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(r).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    } else {
        serde_json::to_string(r).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }
}
