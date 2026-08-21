//! Text helpers for the `::slot value` line format used by the uroman data
//! files. Shared between the build-time dictionary compiler and the runtime.

/// Captures the value associated with a `::slot` in a line.
///
/// # Example
/// `slot_value_in_double_colon_del_list("::s1 of course ::s2 ::cost 0.3", "cost")`
/// returns `Some("0.3")`.
pub fn slot_value_in_double_colon_del_list<'a>(line: &'a str, slot: &str) -> Option<&'a str> {
    let search_str = format!("::{slot}");
    if let Some(start_index) = line.find(&search_str) {
        let remaining = &line[start_index + search_str.len()..];
        if let Some(end_index) = remaining.find("::") {
            Some(remaining[..end_index].trim())
        } else {
            Some(remaining.trim())
        }
    } else {
        None
    }
}

/// Checks if a slot exists in the line, even if it has no value.
pub fn has_value_in_double_colon_del_list(line: &str, slot: &str) -> bool {
    slot_value_in_double_colon_del_list(line, slot).is_some()
}

/// Removes matching quotes (`'…'`, `"…"`, `“…”`) from a string.
pub fn dequote_string(s: &str) -> &str {
    use regex::Regex;
    use std::sync::OnceLock;

    static DEQUOTE_RE: OnceLock<Regex> = OnceLock::new();
    let re = DEQUOTE_RE.get_or_init(|| Regex::new(r#"^\s*(['"“])(.*)(['"”])\s*$"#).unwrap());

    if let Some(m) = re.captures(s) {
        let open_quote = m.get(1).map_or("", |m| m.as_str());
        let content = m.get(2).map_or("", |m| m.as_str());
        let close_quote = m.get(3).map_or("", |m| m.as_str());

        if (open_quote == "'" && close_quote == "'")
            || (open_quote == "\"" && close_quote == "\"")
            || (open_quote == "“" && close_quote == "”")
        {
            return content;
        }
    }
    s
}
