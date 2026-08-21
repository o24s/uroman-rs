//! Runtime utility functions.

use regex::Regex;
use std::sync::LazyLock;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

static HAS_ESCAPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\(x[0-9a-fA-F]{2}|u[0-9a-fA-F]{4}|U[0-9a-fA-F]{8})").unwrap());

/// Checks if a character is a non-spacing mark.
pub fn char_is_nonspacing_mark(c: char) -> bool {
    matches!(c.general_category(), GeneralCategory::NonspacingMark)
}

/// Decodes `\xNN`, `\uNNNN` and `\UNNNNNNNN` escapes above U+0080.
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
