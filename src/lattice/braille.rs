use super::Lattice;
use crate::edge::Edge;

impl<'a> Lattice<'a> {
    pub fn prep_braille(&mut self) {
        if !self
            .contains_script
            .get("Braille")
            .copied()
            .unwrap_or(false)
        {
            return;
        }

        let dots6 = '\u{2820}';
        let braille_space = '\u{2800}';
        let mut all_caps = false;

        for i in 0..self.s_chars.len() {
            let c = self.s_chars[i];

            if i >= 1 && self.s_chars[i - 1] == dots6 && c == dots6 {
                all_caps = true;
            } else if all_caps {
                if c == braille_space {
                    all_caps = false;
                } else {
                    self.props.insert(("is-upper", i), Some(true));
                }
            }
        }
    }

    pub fn add_braille_numbers(&mut self) {
        if !self
            .contains_script
            .get("Braille")
            .copied()
            .unwrap_or(false)
        {
            return;
        }

        let mut found_numbers: Vec<(usize, usize, String)> = Vec::new();

        let mut num_s = String::new();
        let mut start: Option<usize> = None;

        for (i, &char) in self.s_chars.iter().enumerate() {
            if char == '\u{283C}' {
                // number mark
                if start.is_none() {
                    start = Some(i);
                }
            } else if start.is_some() {
                if let Some(digit_s) = self.braille_digit(char) {
                    num_s.push_str(&digit_s);
                } else if char == '\u{2832}' {
                    // period
                    num_s.push('.');
                } else if char == '\u{2802}' {
                    // comma
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
                    "number".to_string(),        // type
                    Some("Braille".to_string()), // script
                    None,                        // num_base
                    None,                        // n_decimals
                    txt,
                );
                self.add_edge(new_edge);
            }
        }
    }

    /// Convert Braille characters to their corresponding numeric string representations ("0"-"9").
    fn braille_digit(&self, char: char) -> Option<String> {
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
}
