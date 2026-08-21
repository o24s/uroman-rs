use super::Lattice;
use regex::Regex;
use std::sync::LazyLock;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

static P_L_OR_M: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{M}]").unwrap());
static P_L_OR_M_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{M}]$").unwrap());
static CONSONANT_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[bcdfghjklmnpqrstvxz]$").unwrap());
static VOWEL_START_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[aeiou]").unwrap());

impl<'a> Lattice<'a> {
    pub fn is_at_start_of_word(&mut self, position: usize) -> bool {
        let key = ("preceded_by_alpha", position);
        if let Some(cached_val) = self.props.get(&key) {
            return !cached_val.unwrap_or(false);
        }

        if position > 0 {
            let prev_char = self.s_chars[position - 1];
            // Python: `(\pL|\pM)`
            if matches!(
                prev_char.general_category_group(),
                GeneralCategoryGroup::Letter | GeneralCategoryGroup::Mark
            ) {
                self.props.insert(key, Some(true));
                return false;
            }
        }

        if let Some(starts) = self.left_links.get(&position) {
            for &start in starts {
                if let Some(edges) = self.edge_lattice.get(&(start, position)) {
                    for edge in edges {
                        if let Some(last_char) = edge.txt().chars().last() {
                            if matches!(
                                last_char.general_category_group(),
                                GeneralCategoryGroup::Letter | GeneralCategoryGroup::Mark
                            ) {
                                self.props.insert(key, Some(true));
                                return false;
                            }
                            // A special case of Braille
                            let first_char_is_braille =
                                ('\u{2800}'..='\u{28FF}').contains(&self.s_chars[position]);
                            if first_char_is_braille && last_char == '\'' {
                                self.props.insert(key, Some(true));
                                return false;
                            }
                        }
                    }
                }
            }
        }

        self.props.insert(key, Some(false));
        true
    }

    pub fn is_at_end_of_word(&mut self, position: usize) -> bool {
        let key = ("followed_by_alpha", position);
        if let Some(cached_val) = self.props.get(&key) {
            return !cached_val.unwrap_or(false);
        }

        let mut start = position;
        if start < self.max_vertex {
            let next_char = self.s_chars[start];
            if matches!(
                next_char.general_category_group(),
                GeneralCategoryGroup::Letter | GeneralCategoryGroup::Mark
            ) {
                self.props.insert(key, Some(true));
                return false;
            }
        }

        while start + 1 < self.max_vertex
            && self.uroman.char_is_nonspacing_mark(self.s_chars[start])
            && self.uroman.chr_name(self.s_chars[start]).contains("NUKTA")
        {
            start += 1;
        }

        for end in (start + 1)..=self.max_vertex {
            let sub: String = self.s_chars[start..end].iter().collect();
            if !self.uroman.dict_bool_get("s-prefix", &sub) {
                break;
            }
            if let Some(rom_rules) = self.uroman.rom_rules.get(&sub) {
                for rom_rule in rom_rules {
                    if !rom_rule.use_only_at_start_of_word
                        && let Some(rom) = &rom_rule.t
                        && P_L_OR_M.is_match(rom)
                    {
                        self.props.insert(key, Some(true));
                        return false;
                    }
                }
            }
        }

        self.props.insert(key, Some(false));
        true
    }

    pub(super) fn is_at_end_of_syllable(&mut self, position: usize) -> (bool, String) {
        let prev_char = if position >= 2 {
            self.s_chars.get(position - 2).copied()
        } else {
            None
        };
        let mut next_char = self.s_chars.get(position).copied();

        let mut adj_position = position;
        if let Some(nc) = next_char
            && !self.uroman.dict_str_get("tone-mark", nc).is_empty()
        {
            adj_position += 1;
            next_char = self.s_chars.get(adj_position).copied();
        }
        let next_char2 = self.s_chars.get(adj_position + 1).copied();

        if prev_char.is_none() {
            return (false, "start-of-string".to_string());
        }
        if let Some(pc) = prev_char {
            if !P_L_OR_M_END.is_match(&pc.to_string()) {
                return (false, "start-of-token".to_string());
            }
            if self.uroman.dict_str_get("syllable-info", pc)
                == "written-pre-consonant-spoken-post-consonant"
            {
                return (false, "pre-post-vowel-on-left".to_string());
            }
        }

        if let Some(nc) = next_char
            && self.uroman.dict_str_get("syllable-info", nc)
                == "written-pre-consonant-spoken-post-consonant"
        {
            return (true, "pre-post-vowel-on-right".to_string());
        }

        if adj_position >= self.max_vertex {
            return (true, "end-of-string".to_string());
        }
        if let Some(nc) = next_char {
            if !P_L_OR_M.is_match(&nc.to_string()) {
                return (true, "end-of-token".to_string());
            }
        } else {
            return (true, "end-of-string".to_string());
        }

        if position > 0
            && let Some(left_edge) = self.best_left_neighbor_edge(position - 1, false)
            && CONSONANT_END_RE.is_match(left_edge.txt())
        {
            return (false, "consonant-to-the-left".to_string());
        }

        let next_char_rom = self
            .simple_top_romanization_candidate_for_span(
                adj_position,
                (adj_position + 2).min(self.max_vertex),
                true,
            )
            .or_else(|| {
                self.simple_top_romanization_candidate_for_span(
                    adj_position,
                    (adj_position + 1).min(self.max_vertex),
                    true,
                )
            });

        let Some(next_char_rom) = next_char_rom else {
            return (true, "not-followed-by-vowel ?".to_string());
        };

        if !VOWEL_START_RE.is_match(&next_char_rom.to_lowercase()) {
            return (true, format!("not-followed-by-vowel {next_char_rom}"));
        }

        // Special rules for THAI CHARACTER O ANG (`\u0E2D`)
        if next_char == Some('\u{0E2D}') && next_char2.is_some() {
            let next_char2_rom = self
                .simple_top_romanization_candidate_for_span(
                    adj_position + 1,
                    (adj_position + 2).min(self.max_vertex),
                    true,
                )
                .unwrap_or_else(|| "?".to_string());
            if VOWEL_START_RE.is_match(&next_char2_rom.to_lowercase()) {
                return (true, "o-ang-followed-by-vowel".to_string());
            }
        }

        (false, "not-at-syllable-end-by-default".to_string())
    }
}
