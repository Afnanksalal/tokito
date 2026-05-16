//! Net label sanitization (safe display / export strings).

const MAX_NET_NAME_LEN: usize = 128;

/// Sanitize a net name for storage and UI: trim, strip controls, cap length.
pub fn sanitize_net_name(raw: &str) -> String {
    let trimmed = raw.trim();
    let mut out = String::with_capacity(trimmed.len().min(MAX_NET_NAME_LEN));
    for ch in trimmed.chars() {
        if ch.is_control() {
            continue;
        }
        if out.len() >= MAX_NET_NAME_LEN {
            break;
        }
        out.push(ch);
    }
    if out.is_empty() {
        "NET".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_control_chars() {
        assert_eq!(sanitize_net_name("VCC\u{0000}"), "VCC");
    }

    #[test]
    fn empty_becomes_net() {
        assert_eq!(sanitize_net_name("   "), "NET");
    }
}
