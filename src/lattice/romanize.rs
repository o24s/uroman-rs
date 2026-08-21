use super::Lattice;
use crate::core::AbugidaRuleType;
use crate::decompositions::DECOMPOSITIONS;
use crate::edge::Edge;
use crate::{Uroman, rom_format};
use regex::Regex;
use std::sync::LazyLock;
use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

/// Python: regex.match(r'(ch|[bcdfghjklmnpqrstwz])', rom)
static DOUBLE_CONSONANT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(ch|[bcdfghjklmnpqrstwz])").unwrap());
/// Python: regex.match(r'[bcdfghjklmnpqrstvwxyz]', rom)
static THAI_CONSONANT_START_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxyz]").unwrap());
static THAI_CONSONANT_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxyz]+$").unwrap());
static THAI_CONSONANT_XZ_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[bcdfghjklmnpqrstvwxz]+$").unwrap());
static THAI_CONSONANT_XZ_ONLY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxz]+$").unwrap());
// Python: regex.match(r'[a-z]', rom)
static BRAILLE_LOWER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z]").unwrap());
/// Python: regex.search(r'([bcdfghjklmnpqrstvwxyz]i$)', rom)
static JP_Y_ENDING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([bcdfghjklmnpqrstvwxyz])i$").unwrap());
static ABUGIDA_CONSONANT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxyz]+$").unwrap());
// Python: regex.search('r[aeiou]', rom)
static CONTAINS_VOWEL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[aeiou]").unwrap());
static RE_AO: LazyLock<(Regex, Regex)> = LazyLock::new(|| {
    (
        Regex::new(r"([cfghkmnqrstxy]?y)(a+|o+)-?$").unwrap(),
        Regex::new(r"([bcdfghjklmnpqrstvwxyz]+)(a|o)-?$").unwrap(),
    )
});
static RE_A: LazyLock<(Regex, Regex)> = LazyLock::new(|| {
    (
        Regex::new(r"([cfghkmnqrstxy]?y)(a+)-?$").unwrap(),
        Regex::new(r"([bcdfghjklmnpqrstvwxyz]+)(a)-?$").unwrap(),
    )
});

impl<'a> Lattice<'a> {
    pub fn add_romanization(&mut self) {
        // Python: for start in range(self.max_vertex):
        for start in 0..self.max_vertex {
            // Python: for end in range(start+1, self.max_vertex+1):
            //         if not self.uroman.dict_bool[('s-prefix', self.s[start:end])]: break
            //         if (rom := self.simple_top_romanization_candidate_for_span(start, end)) is not None:
            //             ...
            for end in (start + 1)..(self.max_vertex + 1) {
                let sub: String = self.s_chars[start..end].iter().collect();
                if !self.uroman.dict_bool_get("s-prefix", &sub) {
                    break;
                }
                if let Some(mut rom) =
                    self.simple_top_romanization_candidate_for_span(start, end, false)
                {
                    if self
                        .contains_script
                        .get("Braille")
                        .copied()
                        .unwrap_or(false)
                        && (end - start == 1)
                        && self
                            .props
                            .get(&("is-upper", start))
                            .copied()
                            .flatten()
                            .unwrap_or(false)
                    {
                        rom = rom.to_uppercase();
                    }

                    // rom tail
                    let mut edge_annotation = "rom".to_string();
                    if let Some(stripped) = rom.strip_prefix('+')
                        && (stripped.len() == 1
                            || (stripped.len() == 2 && stripped.starts_with('n')))
                    {
                        rom = stripped.to_string();
                        edge_annotation = "rom tail".to_string();
                    }

                    // Add the default vowel for Abugida
                    let new_rom =
                        self.add_default_abugida_vowel(rom.clone(), start, end, &edge_annotation);
                    if new_rom.starts_with(&rom) {
                        let suffix = &new_rom[rom.len()..];
                        if !suffix.is_empty() && suffix.chars().all(|c| "aeiou".contains(c)) {
                            edge_annotation = format!("{edge_annotation} c:{rom} s:{suffix}");
                        }
                    }
                    rom = new_rom;

                    // Python: rom, start2, end2, exp_edge_annotation = self.expand_rom_with_special_chars(...)
                    // Python: self.add_edge(Edge(start2, end2, rom, edge_annotation))
                    let (expanded_rom, new_start, new_end, exp_edge_annotation) =
                        self.expand_rom_with_special_chars(rom, start, end);

                    let final_annotation = exp_edge_annotation.unwrap_or(edge_annotation);

                    self.add_edge(Edge::new_regular(
                        new_start,
                        new_end,
                        expanded_rom,
                        final_annotation,
                    ));
                }
            }

            // Python: if start < len(self.s): ...
            let char = self.s_chars[start];
            let cp = char as u32;

            // Python: if 0xAC00 <= cp <= 0xD7A3: ... self.add_edge(...)
            if (0xAC00..=0xD7A3).contains(&cp)
                && let Some(rom) = self.uroman.unicode_hangul_romanization(char)
            {
                self.add_edge(Edge::new_regular(start, start + 1, rom, "rom".to_string()));
            }

            // Python: if rom_decomp := self.decomp_rom(start): self.add_edge(...)
            if let Some(rom_decomp) = self.decomp_rom(start) {
                self.add_edge(Edge::new_regular(
                    start,
                    start + 1,
                    rom_decomp,
                    "rom decomp".to_string(),
                ));
            }
        }
    }

    pub fn add_rom_fall_back_singles(&mut self) {
        for start in 0..self.max_vertex {
            let end = start + 1;

            let is_covered = self
                .edge_lattice
                .get(&(start, end))
                .is_some_and(|edges| !edges.is_empty());
            if is_covered {
                continue;
            }

            let orig_char = self.s_chars[start];
            let (rom, edge_annotation) = self.get_fallback_rom_and_annot(orig_char, start, end);

            self.add_edge(Edge::new_regular(
                start,
                end,
                rom.clone(),
                edge_annotation.clone(),
            ));
        }
    }

    fn get_fallback_rom_and_annot(
        &mut self,
        orig_char: char,
        start: usize,
        end: usize,
    ) -> (String, String) {
        match orig_char.general_category() {
            // Mn (Nonspacing_Mark) -> empty string
            GeneralCategory::NonspacingMark => ("".to_string(), "Mn".to_string()),
            // Cf (Format) -> empty string
            GeneralCategory::Format => ("".to_string(), "Cf".to_string()),
            // Co (PrivateUse) -> empty string
            GeneralCategory::PrivateUse => ("".to_string(), "Co".to_string()),
            // Zs (SpaceSeparator) -> half-width space
            GeneralCategory::SpaceSeparator => (" ".to_string(), "Zs".to_string()),
            _ => {
                if let Some(rom) = self.simple_top_romanization_candidate_for_span(start, end, true)
                {
                    let final_rom = if rom.starts_with('+')
                        && rom.len() > 1
                        && "mngr".contains(rom.chars().nth(1).unwrap())
                    {
                        rom[1..].to_string()
                    } else {
                        rom
                    };
                    (final_rom, "rom single".to_string())
                } else {
                    (orig_char.to_string(), "orig".to_string())
                }
            }
        }
    }

    fn get_decomposition(&self, c: char) -> Option<(&'static str, &'static str)> {
        DECOMPOSITIONS.get(&c).cloned()
    }

    fn char_has_numeric_value(&self, c: char) -> bool {
        c.general_category_group() == GeneralCategoryGroup::Number
    }

    pub fn decomp_rom(&self, char_position: usize) -> Option<String> {
        let target_char = self.s_chars[char_position];

        if let Some((tag, decomp_s)) = self.get_decomposition(target_char) {
            let mut rom: Option<String> = None;

            // Python: `if (format_comps and (format_comps[0] not in ('<super>', '<sub>', '<noBreak>', '<compat>'))
            //         and (not other_comps) and decomp_s):``
            if !tag.is_empty()
                && !matches!(tag, "<super>" | "<sub>" | "<noBreak>" | "<compat>")
                && !decomp_s.is_empty()
            {
                rom = Some(
                    Uroman::new()
                        .romanize_string::<rom_format::Str>(decomp_s, None)
                        .to_string(),
                );
            }

            if rom.is_some() && self.char_has_numeric_value(target_char) {
                let mut r = rom.unwrap();
                r = r.replace('⁄', "/");

                let mut final_rom = String::new();

                if char_position > 0
                    && self
                        .s_chars
                        .get(char_position - 1)
                        .is_some_and(|c| c.is_numeric())
                {
                    final_rom.push(' ');
                }
                final_rom.push_str(&r);
                if char_position + 1 < self.s_chars.len()
                    && self
                        .s_chars
                        .get(char_position + 1)
                        .is_some_and(|c| c.is_numeric())
                {
                    final_rom.push(' ');
                }
                rom = Some(final_rom);
            }
            return rom;
        }

        None
    }

    fn add_default_abugida_vowel(
        &mut self,
        mut rom: String,
        start: usize,
        end: usize,
        annotation: &str,
    ) -> String {
        let Some(first_s_char) = self.s_chars.get(start).copied() else {
            return rom;
        };
        let Some(last_s_char) = self.s_chars.get(end - 1).copied() else {
            return rom;
        };

        let script_name = self.uroman.chr_script_name(first_s_char);
        if script_name.is_empty() {
            return rom;
        }

        let Some(script) = self.uroman.scripts.get(&script_name.to_lowercase()) else {
            return rom;
        };
        let Some(abugida_rule_type) = &script.abugida_rule_type else {
            return rom;
        };

        let abugida_regexes = match abugida_rule_type {
            AbugidaRuleType::A => &RE_A,
            AbugidaRuleType::AO => &RE_AO,
        };

        let mut base_rom: Option<String>;
        let mut base_rom_plus_vowel: Option<String>;
        let mut modified_rom = rom.clone();

        if let Some(caps) = abugida_regexes.0.captures(&rom) {
            base_rom = Some(caps[1].to_string());
            base_rom_plus_vowel = Some(format!("{}{}", &caps[1], &caps[2]));
        } else if let Some(caps) = abugida_regexes.1.captures(&rom) {
            base_rom = Some(caps[1].to_string());
            base_rom_plus_vowel = Some(format!("{}{}", &caps[1], &caps[2]));
            if rom.ends_with('-')
                && start + 1 == end
                && rom.chars().next().is_some_and(|c| c.is_alphabetic())
            {
                modified_rom.pop();
            }
        } else {
            base_rom = Some(rom.clone());
            base_rom_plus_vowel = Some(format!("{}{}", rom, script.abugida_default_vowels[0]));
        }

        if let Some(br) = &base_rom
            && !(ABUGIDA_CONSONANT_RE.is_match(br) || (script_name == "Tibetan" && br == "'"))
        {
            base_rom = None;
            base_rom_plus_vowel = None;
        }

        rom = modified_rom;
        let Some(base_rom) = base_rom else {
            return rom;
        };
        let Some(base_rom_plus_vowel) = base_rom_plus_vowel else {
            return rom;
        };

        if annotation.contains("tail") {
            return rom;
        }
        let prev_s_char = if start > 0 {
            self.s_chars.get(start - 1).copied()
        } else {
            None
        };
        let next_s_char = self.s_chars.get(end).copied();
        let next2_s_char = self.s_chars.get(end + 1).copied();

        if script_name == "Tibetan" {
            if self
                .props
                .get(&("edge-delete", start))
                .copied()
                .flatten()
                .unwrap_or(false)
            {
                return "".to_string();
            } else if self
                .props
                .get(&("edge-vowel", start))
                .copied()
                .flatten()
                .unwrap_or(false)
            {
                return base_rom_plus_vowel;
            } else {
                return base_rom;
            }
        }

        if let Some(nc) = next_s_char {
            let nc_string = &nc.to_string();
            let khmer_yo_condition = (base_rom == "b"
                || base_rom == "c"
                || base_rom == "d"
                || base_rom == "z"
                || base_rom == "ng")
                && nc == 'យ';
            if khmer_yo_condition {
                return base_rom;
            }
            if self.uroman.dict_bool_get("is-vowel-sign", nc_string) {
                return base_rom;
            }
            if self
                .uroman
                .dict_bool_get("is-medial-consonant-sign", nc_string)
            {
                return base_rom;
            }
            if self.char_is_subjoined_letter(nc) {
                return base_rom;
            }
            if self.uroman.char_is_nonspacing_mark(nc)
                && let Some(n2c) = next2_s_char
                && self.uroman.dict_bool_get("is-vowel-sign", &n2c.to_string())
            {
                return base_rom;
            }
            if self.uroman.dict_bool_get("is-virama", nc_string) {
                return base_rom;
            }
            if self.uroman.char_is_nonspacing_mark(nc)
                && let Some(n2c) = next2_s_char
                && self.uroman.dict_bool_get("is-virama", &n2c.to_string())
            {
                return base_rom;
            }
        }

        if let Some(pc) = prev_s_char
            && self.uroman.dict_bool_get("is-virama", &pc.to_string())
        {
            return base_rom_plus_vowel;
        }

        if self.is_at_start_of_word(start) && !CONTAINS_VOWEL_RE.is_match(&rom) {
            return base_rom_plus_vowel;
        }

        if self.is_at_end_of_word(end) {
            if script_name == "Devanagari" && self.lcode.as_deref() != Some("san") {
                return rom;
            } else if let Some(lc) = &self.lcode
                && ["asm", "ben", "guj", "kas", "pan"].contains(&lc.as_str())
            {
                return rom;
            }
            return base_rom_plus_vowel;
        }

        if prev_s_char.is_none_or(|pc| self.uroman.chr_script_name(pc) != script_name) {
            return base_rom_plus_vowel;
        }
        if self.uroman.chr_name(last_s_char).contains("VOCALIC") {
            return base_rom;
        }
        if next_s_char.is_some_and(|nc| self.uroman.chr_script_name(nc) == script_name) {
            return base_rom_plus_vowel;
        }

        rom
    }

    fn expand_rom_with_special_chars(
        &mut self,
        mut rom: String,
        mut start: usize,
        mut end: usize,
    ) -> (String, usize, usize, Option<String>) {
        // Python: orig_start = start
        // Python: annot = None
        let orig_start = start;
        let mut annot: Option<String> = None;

        // Python: if rom == '': return rom, start, end, None
        if rom.is_empty() {
            return (rom, start, end, None);
        }

        // Python: prev_char = (full_string[start-1] if start >= 1 else '')
        // Python: first_char = full_string[start]
        // Python: last_char = full_string[end-1]
        // Python: next_char = (full_string[end] if end < len(full_string) else '')
        let mut prev_char: Option<char> = if start > 0 {
            self.s_chars.get(start - 1).copied()
        } else {
            None
        };
        let first_char: char = self.s_chars[start];
        let mut last_char: char = self.s_chars[end - 1];
        let mut next_char: Option<char> = self.s_chars.get(end).copied();

        // Python: if (prev_char == '\u2820') and regex.match(r'[a-z]', rom):
        // Python:     return rom[0].upper() + rom[1:], start-1, end, 'rom exp'
        if prev_char == Some('\u{2820}') && BRAILLE_LOWER_RE.is_match(&rom) {
            let mut new_rom = rom[0..1].to_uppercase();
            new_rom.push_str(&rom[1..]);
            return (new_rom, start - 1, end, Some("rom exp".to_string()));
        }

        // Python: if start+1 == end and rom.isupper() and next_char.islower():
        // Python:     rom = rom.capitalize()
        let mut chars = rom.chars();
        if end - start == 1
            && chars.all(|c| c.is_uppercase())
            && next_char.is_some_and(|nc| nc.is_lowercase())
            && let Some(first) = chars.next()
        {
            rom = first.to_uppercase().to_string() + &chars.as_str().to_lowercase();
        }

        // Python: if (prev_char and prev_char in 'っッ\u0A71') ...
        if let Some(pc) = prev_char
            && "っッ\u{0A71}".contains(pc)
            && self.uroman.chr_script_name(pc) == self.uroman.chr_script_name(first_char)
            && let Some(caps) = DOUBLE_CONSONANT_RE.captures(&rom)
        {
            let consonant_to_double = caps.get(1).unwrap().as_str();
            let prefix = if "っッ".contains(pc) {
                consonant_to_double.replace("ch", "t")
            } else {
                consonant_to_double.replace("ch", "c")
            };
            rom.insert_str(0, &prefix);
            start -= 1;
            annot = Some("rom exp".to_string());
            prev_char = if start > 0 {
                self.s_chars.get(start - 1).copied()
            } else {
                None
            };
        }

        // Python: if uroman.chr_script_name(first_char) == 'Thai':
        if self.uroman.chr_script_name(first_char) == "Thai" {
            // Python: if (start+1 == end) and regex.match(r'[bcdfghjklmnpqrstvwxyz]+$', rom):
            if end - start == 1 && THAI_CONSONANT_END_RE.is_match(&rom) {
                // Python: if uroman.dict_str[('syllable-info', prev_char)] == 'written-pre-consonant-spoken-post-consonant':
                if let Some(pc) = prev_char
                    && self.uroman.dict_str_get("syllable-info", pc)
                        == "written-pre-consonant-spoken-post-consonant"
                {
                    // Python: for vowel_prefix_len in [1]:
                    for vowel_prefix_len in [1] {
                        if vowel_prefix_len <= start {
                            // Python: for vowel_suffix_len in [3, 2, 1]:
                            for vowel_suffix_len in [3, 2, 1] {
                                if end + vowel_suffix_len <= self.s_chars.len() {
                                    // Python: pattern = (full_string[start-vowel_prefix_len: start] + '–' + full_string[end:end+vowel_suffix_len])
                                    let pattern_chars = [
                                        &self.s_chars[start - vowel_prefix_len..start],
                                        &['–'],
                                        &self.s_chars[end..end + vowel_suffix_len],
                                    ]
                                    .concat();
                                    let pattern: String = pattern_chars.into_iter().collect();
                                    // Python: if uroman.rom_rules[pattern]:
                                    // Python: vowel_rom = vowel_rom_rule['t']
                                    if let Some(vowel_rom_rule) = self
                                        .uroman
                                        .rom_rules
                                        .get(&pattern)
                                        .and_then(|rules| rules.first())
                                        && let Some(vowel_rom) = &vowel_rom_rule.t
                                    {
                                        // Python: return rom + vowel_rom, start-vowel_prefix_len, end+vowel_suffix_len, 'rom exp'
                                        return (
                                            format!("{rom}{vowel_rom}"),
                                            start - vowel_prefix_len,
                                            end + vowel_suffix_len,
                                            Some("rom exp".to_string()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Python: if (uroman.chr_script_name(prev_char) == 'Thai')
            // and (uroman.dict_str[('syllable-info', prev_char)] == 'written-pre-consonant-spoken-post-consonant')
            // and regex.match(r'[bcdfghjklmnpqrstvwxyz]', rom) and (vowel_rom := self.romanization_by_first_rule(prev_char)):
            if let Some(pc) = prev_char
                && self.uroman.chr_script_name(pc) == "Thai"
                && self.uroman.dict_str_get("syllable-info", pc)
                    == "written-pre-consonant-spoken-post-consonant"
                && THAI_CONSONANT_START_RE.is_match(&rom)
                && let Some(vowel_rom) = self.romanization_by_first_rule(&pc.to_string())
            {
                // Python: return rom + vowel_rom, start-1, end, 'rom exp'
                rom.push_str(&vowel_rom);
                start -= 1;
                annot = Some("rom exp".to_string());
                prev_char = if start > 0 {
                    self.s_chars.get(start - 1).copied()
                } else {
                    None
                };
            }
            // Python: if (first_char == '\u0E2D') and (end - start == 1):
            if first_char == '\u{0E2D}' && end - start == 1 {
                // Python: prev_script = uroman.chr_script_name(prev_char)
                let prev_script =
                    prev_char.map_or("".to_string(), |c| self.uroman.chr_script_name(c));
                // Python: next_script = uroman.chr_script_name(next_char)
                let next_script =
                    next_char.map_or("".to_string(), |c| self.uroman.chr_script_name(c));
                // Python: prev_rom = self.find_rom_edge_path_backwards(0, start, 1, return_str=True)
                let prev_rom = if start > 0 {
                    self.find_rom_edge_path_backwards_str(0, start, Some(1), false)
                } else {
                    "".to_string()
                };
                // Python: next_rom = self.romanization_by_first_rule(next_char)
                let next_rom = next_char
                    .and_then(|nc| self.romanization_by_first_rule(&nc.to_string()))
                    .unwrap_or_default();
                // Python: if not ((prev_script == 'Thai') and (next_script == 'Thai') andregex.match(r'[bcdfghjklmnpqrstvwxz]+$', prev_rom) and regex.match(r'[bcdfghjklmnpqrstvwxz]+$', next_rom)):
                let is_between_consonants = prev_script == "Thai"
                    && next_script == "Thai"
                    && THAI_CONSONANT_XZ_END_RE.is_match(&prev_rom)
                    && THAI_CONSONANT_XZ_ONLY_RE.is_match(&next_rom);
                if !is_between_consonants {
                    // Python: return '', start, end, 'rom del'
                    return ("".to_string(), start, end, Some("rom del".to_string()));
                }
            }
        }

        // Python: if next_char and (next_char == "\u0300") and (uroman.chr_script_name(last_char) == "Coptic")...
        if next_char == Some('\u{0300}')
            && self.uroman.chr_script_name(last_char) == "Coptic"
            && self
                .simple_top_romanization_candidate_for_span(orig_start, end + 1, true)
                .is_none()
        {
            rom.insert(0, 'e');
            end += 1;
            annot = Some("rom exp".to_string());
            last_char = self.s_chars[end - 1];
            next_char = self.s_chars.get(end).copied();
        }

        // Python: if (next_char and next_char in 'ゃゅょャュョ') ...
        if let Some(nc) = next_char
            && "ゃゅょャュョ".contains(nc)
            && self.uroman.chr_script_name(last_char) == self.uroman.chr_script_name(nc)
            && JP_Y_ENDING_RE.is_match(&rom)
            && let Some(y_rom) = self.romanization_by_first_rule(&nc.to_string())
            && self
                .simple_top_romanization_candidate_for_span(orig_start, end + 1, true)
                .is_none()
            && self
                .simple_top_romanization_candidate_for_span(start, end + 1, true)
                .is_none()
        {
            rom.pop();
            rom.push_str(&y_rom);
            end += 1;
            annot = Some("rom exp".to_string());
            last_char = self.s_chars[end - 1];
            next_char = self.s_chars.get(end).copied();
        }

        // Python: if (next_char == 'ー') and (uroman.chr_script_name(last_char) in ('Hiragana', 'Katakana')) ...
        if next_char == Some('ー')
            && let Some(last_rom_char) = rom.chars().last()
        {
            let last_s_char_script = self.uroman.chr_script_name(last_char);
            if ("Hiragana" == last_s_char_script || "Katakana" == last_s_char_script)
                && "aeiou".contains(last_rom_char)
            {
                rom.push(last_rom_char);
                return (rom, start, end + 1, Some("rom exp".to_string()));
            }
        }

        // Python: if self.uroman.dict_bool[('is-virama', next_char)]:
        // Python:     return rom, start, end + 1, "rom exp"
        if next_char.is_some_and(|nc| self.uroman.dict_bool_get("is-virama", &nc.to_string())) {
            return (rom, start, end + 1, Some("rom exp".to_string()));
        }

        // Python: if rom.startswith(' ') and ((start == 0) or (prev_char == ' ')): rom = rom[1:]
        if rom.starts_with(' ') && (start == 0 || prev_char == Some(' ')) {
            rom.remove(0);
        }
        // Python: if rom.endswith(' ') and ((end == len(full_string)+1) or (next_char == ' ')): rom = rom[:-1]
        if rom.ends_with(' ') && next_char == Some(' ') {
            rom.pop();
        }

        (rom, start, end, annot)
    }
}
