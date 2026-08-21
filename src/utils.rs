//! Utility functions for parsing uroman data files.

use regex::Regex;
use std::sync::{LazyLock, OnceLock};

static HAS_ESCAPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\(x[0-9a-fA-F]{2}|u[0-9a-fA-F]{4}|U[0-9a-fA-F]{8})").unwrap());

/// Captures the value associated with a `::slot` in a line.
///
/// This function is a Rust port of the Python version's `slot_value_in_double_colon_del_list`.
/// It uses a dynamically generated regex to find the slot and extract its value.
///
/// # Example
/// `slot_value_in_double_colon_del_list("::s1 of course ::s2 ::cost 0.3", "cost")` returns `Some("0.3")`.
pub fn slot_value_in_double_colon_del_list<'a>(line: &'a str, slot: &'a str) -> Option<&'a str> {
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

/// Removes matching quotes from the start and end of a string.
///
/// Handles single quotes, double quotes, and curly double quotes.
pub fn dequote_string(s: &str) -> &str {
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

pub fn decode_unicode_escapes(s: &str) -> String {
    if !HAS_ESCAPE_RE.is_match(s) {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len());
    let mut last_end = 0;

    for m in HAS_ESCAPE_RE.find_iter(s) {
        result.push_str(&s[last_end..m.start()]);

        let full_escape_sequence = m.as_str();
        let hex_part = &full_escape_sequence[2..];

        let codepoint = u32::from_str_radix(hex_part, 16).unwrap();

        if codepoint > 0x80 {
            result.push(std::char::from_u32(codepoint).unwrap_or(std::char::REPLACEMENT_CHARACTER));
        } else {
            result.push_str(full_escape_sequence);
        }

        last_end = m.end();
    }

    result.push_str(&s[last_end..]);

    result
}
