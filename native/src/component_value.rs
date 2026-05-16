//! Default component values and field placeholders (schematic EDA conventions).

/// Default **Value** field when placing a library symbol (before user edit).
pub fn default_value_for_library_id(symbol_id: &str) -> String {
    if let Some(sym) = symbol_id.rsplit(':').next() {
        if let Some(from_lib) = default_from_short_name(sym) {
            return from_lib;
        }
    }
    default_from_short_name(symbol_id).unwrap_or_else(|| "???".to_string())
}

fn default_from_short_name(short: &str) -> Option<String> {
    let upper = short.to_ascii_uppercase();
    if upper == "R" || upper.starts_with("R_") {
        return Some("R".into());
    }
    if upper == "C" || upper.starts_with("C_") {
        return Some("C".into());
    }
    if upper == "L" || upper.starts_with("L_") {
        return Some("L".into());
    }
    if upper == "D" || upper.starts_with("LED") {
        return Some(if upper.contains("LED") {
            "LED".into()
        } else {
            "D".into()
        });
    }
    if upper.starts_with("SW_") || upper == "SW" {
        return Some("SW".into());
    }
    if upper.starts_with("CONN_") {
        return Some("Conn".into());
    }
    if upper.len() <= 16 && short.chars().any(|c| c.is_ascii_alphanumeric()) {
        return Some(short.to_string());
    }
    None
}

/// Inspector placeholder hint by refdes prefix letter.
pub fn value_placeholder_for_prefix(prefix: &str) -> &'static str {
    match prefix {
        "R" => "e.g. 10k, 4.7k",
        "C" => "e.g. 100n, 10u",
        "L" => "e.g. 10uH, 1mH",
        "D" => "e.g. 1N4148",
        "Q" => "e.g. 2N7002",
        "U" => "part value / IC name",
        "J" => "e.g. Conn_01x04",
        "SW" => "e.g. SW_SPST",
        _ => "component value",
    }
}
