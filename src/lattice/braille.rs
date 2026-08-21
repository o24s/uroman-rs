//! Braille-specific passes: capitalization markers and number sequences.

use super::Lattice;
use crate::edge::Edge;

const BRAILLE_NUMBER_MARK: char = '\u{283C}';
const BRAILLE_PERIOD: char = '\u{2832}';
const BRAILLE_COMMA: char = '\u{2802}';
const BRAILLE_DOTS6: char = '\u{2820}';
const BRAILLE_SPACE: char = '\u{2800}';

impl Lattice {
    /// Marks positions covered by a double-dots-6 all-caps prefix.
    pub(crate) fn prep_braille(&mut self) {
        if !self.has_script("Braille") {
            return;
        }

        let mut all_caps = false;

        for i in 0..self.s_chars.len() {
            let c = self.s_chars[i];

            if i >= 1 && self.s_chars[i - 1] == BRAILLE_DOTS6 && c == BRAILLE_DOTS6 {
                all_caps = true;
            } else if all_caps {
                if c == BRAILLE_SPACE {
                    all_caps = false;
                } else {
                    self.props.is_upper.insert(i);
                }
            }
        }
    }

    /// Collects digit sequences introduced by the Braille number mark into
    /// numeric edges.
    pub(crate) fn add_braille_numbers(&mut self) {
        if !self.has_script("Braille") {
            return;
        }

        let mut found_numbers: Vec<(usize, usize, String)> = Vec::new();

        let mut num_s = String::new();
        let mut start: Option<usize> = None;

        for (i, &char) in self.s_chars.iter().enumerate() {
            if char == BRAILLE_NUMBER_MARK {
                if start.is_none() {
                    start = Some(i);
                }
            } else if start.is_some() {
                if let Some(digit_s) = braille_digit(char) {
                    num_s.push_str(&digit_s);
                } else if char == BRAILLE_PERIOD {
                    num_s.push('.');
                } else if char == BRAILLE_COMMA {
                    num_s.push(',');
                } else {
                    if !num_s.is_empty() {
                        found_numbers.push((start.unwrap(), i, num_s.clone()));
                    }
                    num_s.clear();
                    start = None;
                }
            }
        }

        if let Some(s) = start
            && !num_s.is_empty()
        {
            found_numbers.push((s, self.s_chars.len(), num_s));
        }

        for (start_pos, end_pos, txt) in found_numbers {
            // Remove commas as they cause issues in numerical parsers
            let clean_txt = txt.replace(',', "");
            if let Ok(value) = clean_txt.parse::<f64>() {
                let new_edge = Edge::new_combined_numeric(
                    start_pos,
                    end_pos,
                    value,
                    "number".to_string(),
                    Some("Braille".to_string()),
                    None,
                    None,
                    txt,
                );
                self.add_edge(new_edge);
            }
        }
    }
}

/// Converts a Braille digit character to its numeric string ("0"-"9").
fn braille_digit(char: char) -> Option<String> {
    const BRAILLE_DIGITS: [char; 10] = [
        '\u{281A}', // 0
        '\u{2801}', // 1
        '\u{2803}', // 2
        '\u{2809}', // 3
        '\u{2819}', // 4
        '\u{2811}', // 5
        '\u{280B}', // 6
        '\u{281B}', // 7
        '\u{2813}', // 8
        '\u{280A}', // 9
    ];

    BRAILLE_DIGITS
        .iter()
        .position(|&d| d == char)
        .map(|pos| pos.to_string())
}
