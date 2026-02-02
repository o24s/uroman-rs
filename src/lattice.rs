use crate::decompositions::DECOMPOSITIONS;
use crate::edge::{Edge, EdgeData, NumData, NumDataUpdates};
use crate::rom_rule::RomRule;
use crate::{Uroman, rom_format};
use crate::core::{AbugidaRuleType, UromanInner};
use num_rational::Ratio;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

static P_L_OR_M: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{M}]").unwrap());
static P_L_OR_M_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{M}]$").unwrap());
/// Python: regex.match(r'(ch|[bcdfghjklmnpqrstwz])', rom)
static DOUBLE_CONSONANT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(ch|[bcdfghjklmnpqrstwz])").unwrap());
/// Python: regex.match(r'[bcdfghjklmnpqrstvwxyz]', rom)
static THAI_CONSONANT_START_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxyz]").unwrap());
static THAI_CONSONANT_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxyz]+$").unwrap());

// Python: regex.match(r'[a-z]', rom)
static BRAILLE_LOWER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z]").unwrap());

/// Python: regex.search(r'([bcdfghjklmnpqrstvwxyz]i$)', rom)
static JP_Y_ENDING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([bcdfghjklmnpqrstvwxyz])i$").unwrap());
static ABUGIDA_CONSONANT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxyz]+$").unwrap());
static CONSONANT_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[bcdfghjklmnpqrstvxz]$").unwrap());
static VOWEL_START_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[aeiou]").unwrap());
// Python: regex.search('r[aeiou]', rom)
static CONTAINS_VOWEL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[aeiou]").unwrap());
static STARTS_WITH_DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d").unwrap());
static ENDS_WITH_DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d$").unwrap());

static GOOD_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^'?(?:.|bd|br|brg|brgy|bs|bsh|bst|bt|bts|by|bz|bzh|ch|db|dby|dk|dm|dp|dpy|dr|gl|gn|gr|gs|gt|gy|gzh|kh|khr|khy|kr|ky|ld|lh|lt|mkh|mny|mth|mtsh|ny|ph|phr|phy|rgy|rk|el|rn|rny|rt|rts|sk|skr|sky|sl|sm|sn|sny|sp|spy|sr|st|th|ts|tsh)$").unwrap()
});
static GOOD_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:|[bcdfghjklmnpqrstvwxz]|bh|bs|ch|cs|dd|ddh|dh|dz|dzh|gh|gr|gs|kh|khs|kss|n|nn|nt|ms|ng|ngs|ns|ph|rm|sh|ss|th|ts|tsh|tt|tth|zh|zhs)'?$").unwrap()
});
static ROM_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bc:([a-zA-Z]+)\s+s:([a-zA-Z]+)\b").unwrap());
static ROMS_CONSONANT_A_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([bcdfghjklmnpqrstvwxyz].*)a$").unwrap());
static ROM_VOWEL_END_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[aeiou]+$").unwrap());

static THAI_CONSONANT_XZ_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[bcdfghjklmnpqrstvwxz]+$").unwrap());
static THAI_CONSONANT_XZ_ONLY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[bcdfghjklmnpqrstvwxz]+$").unwrap());

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


pub(super) struct Lattice<'a> {
    pub s: String,
    pub s_chars: Vec<char>,
    pub lcode: Option<String>,
    pub uroman: &'a UromanInner,

    // self.lattice[(edge.start, edge.end)]
    pub edge_lattice: HashMap<(usize, usize), HashSet<Edge>>,
    // self.lattice[(edge.start, 'right')]
    pub right_links: HashMap<usize, HashSet<usize>>,
    // self.lattice[(edge.end, 'left')]
    pub left_links: HashMap<usize, HashSet<usize>>,

    pub max_vertex: usize,

    // self.props: dict
    pub props: HashMap<(String, usize), Option<bool>>,

    // self.simple_top_rom_cache: dict
    pub simple_top_rom_cache: HashMap<(usize, usize), Option<String>>,
    // self.contains_script: defaultdict(bool)
    pub contains_script: HashMap<String, bool>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum BackwardsPathResult {
    Str(String),
    Edges(Vec<Edge>),
}

impl<'a> Lattice<'a> {
    pub fn new(s: &'a str, uroman: &'a UromanInner, lcode: Option<&str>) -> Self {
        let s_chars: Vec<char> = s.chars().collect();
        let max_vertex = s_chars.len();

        let mut lattice = Self {
            s_chars,
            s: s.to_string(),
            lcode: lcode.map(String::from),
            uroman,
            edge_lattice: HashMap::new(),
            right_links: HashMap::new(),
            left_links: HashMap::new(),
            max_vertex,
            props: HashMap::new(),
            simple_top_rom_cache: HashMap::new(),
            contains_script: HashMap::new(),
        };

        lattice.check_for_scripts();

        lattice
    }

    fn check_for_scripts(&mut self) {
        for &c in &self.s_chars {
            let script_name = self.uroman.chr_script_name(c);
            if !script_name.is_empty() {
                self.contains_script.insert(script_name, true);
            }
        }
        if self
            .s
            .contains(|c: char| ('\u{2800}'..='\u{28FF}').contains(&c))
        {
            self.contains_script.insert("Braille".to_string(), true);
        }
    }

    pub fn add_edge(&mut self, edge: Edge) {
        let (start, end) = (edge.start(), edge.end());

        self.edge_lattice
            .entry((start, end))
            .or_default()
            .insert(edge);

        self.right_links.entry(start).or_default().insert(end);

        self.left_links.entry(end).or_default().insert(start);
    }

    fn cand_is_valid(&mut self, rule: &RomRule, start: usize, end: usize) -> bool {
        let lcode_ok = rule.lcodes.is_empty()
            || self
                .lcode
                .as_ref()
                .is_some_and(|lc| rule.lcodes.contains(lc));
        let start_ok = (!rule.use_only_at_start_of_word || self.is_at_start_of_word(start))
            && (!rule.dont_use_at_start_of_word || !self.is_at_start_of_word(start));
        let end_ok = (!rule.use_only_at_end_of_word || self.is_at_end_of_word(end))
            && (!rule.dont_use_at_end_of_word || !self.is_at_end_of_word(end));
        let whole_word_ok = !rule.use_only_for_whole_word
            || (self.is_at_start_of_word(start) && self.is_at_end_of_word(end));
        lcode_ok && start_ok && end_ok && whole_word_ok
    }

    pub fn is_at_start_of_word(&mut self, position: usize) -> bool {
        let key = ("preceded_by_alpha".to_string(), position);
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
        let key = ("followed_by_alpha".to_string(), position);
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

    fn is_at_end_of_syllable(&mut self, position: usize) -> (bool, String) {
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
            })
            .unwrap_or_else(|| "?".to_string());

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

    fn is_gap_null_edge(edge: &Edge) -> bool {
        matches!(edge, Edge::Numeric { num_data, .. } if num_data.orig_txt == "零" || num_data.orig_txt == "〇")
    }

    fn edge_is_digit(edge: &Edge) -> bool {
        if let Edge::Numeric { data, num_data } = edge
            && data.r#type == "digit"
            && data.end - data.start == 1
            && let Some(val) = num_data.value
        {
            return val.fract() == 0.0 && (0.0..=9.0).contains(&val);
        }
        false
    }

    fn char_is_letter_or_vowel_sign(&self, c: char) -> bool {
        let is_vowel_sign = self.uroman.dict_bool_get("is-vowel-sign", &c.to_string());
        self.uroman.chr_name(c).contains("LETTER") || is_vowel_sign
    }

    fn char_is_vowel_sign(&self, c: char) -> bool {
        self.uroman.dict_bool_get("is-vowel-sign", &c.to_string())
    }

    fn char_is_subjoined_letter(&self, c: char) -> bool {
        self.uroman.chr_name(c).contains("SUBJOINED LETTER")
    }

    pub fn simple_top_romanization_candidate_for_span(
        &mut self,
        start: usize,
        end: usize,
        simple_search: bool,
    ) -> Option<String> {
        if end > self.max_vertex { return None; }
        let span_range = (start, end);
        if !simple_search
            && let Some(cached_result) = self.simple_top_rom_cache.get(&span_range) {
                return cached_result.clone();
            }

        let sub: String = self.s_chars[start..end].iter().collect();
        let Some(rules) = self.uroman.rom_rules.get(&sub) else {
            if !simple_search { self.simple_top_rom_cache.insert(span_range, None); }
            return None;
        };

        let mut best_rule_with_t: Option<&RomRule> = None;
        let mut best_rule_without_t: Option<&RomRule> = None;

        for rule in rules.iter() {
            if self.cand_is_valid(rule, start, end) {
                if rule.t.is_some() {
                    if best_rule_with_t.is_none() || rule.n_restr > best_rule_with_t.unwrap().n_restr {
                        best_rule_with_t = Some(rule);
                    }
                } else if best_rule_without_t.is_none() || rule.n_restr > best_rule_without_t.unwrap().n_restr {
                    best_rule_without_t = Some(rule);
                }
            }
        }

        let best_rule = best_rule_with_t.or(best_rule_without_t);

        let best_cand = best_rule.and_then(|r| r.t.clone());

        if simple_search {
            return best_cand;
        }

        let Some(rule) = best_rule else {
            self.simple_top_rom_cache.insert(span_range, None);
            return None;
        };

        let mut final_cand = rule.t.clone();
        if let Some(t_at_end) = &rule.t_at_end_of_syllable {
            let (is_end, _rationale) = self.is_at_end_of_syllable(end);
            if is_end {
                final_cand = Some(t_at_end.clone());
            }
        }

        self.simple_top_rom_cache
            .insert(span_range, final_cand.clone());
        final_cand
    }

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
                    self.props.insert(("is-upper".to_string(), i), Some(true));
                }
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

    pub fn all_edges(&self, start: usize, end: usize) -> Vec<Edge> {
        let mut result = Vec::new();

        // Python: for start2 in range(start, end):
        for start2 in start..end {
            if let Some(end_positions) = self.right_links.get(&start2) {
                let mut sorted_ends: Vec<usize> = end_positions.iter().cloned().collect();
                sorted_ends.sort_unstable_by(|a, b| b.cmp(a));

                // Python: for end2 in ...
                for &end2 in &sorted_ends {
                    // Python: if end2 <= end:
                    if end2 <= end
                        && let Some(edges_for_span) = self.edge_lattice.get(&(start2, end2))
                    {
                        result.extend(edges_for_span.iter().cloned());
                    } else {
                        break;
                    }
                }
            }
        }

        result
    }

    pub fn best_edge_in_span(&self, start: usize, end: usize, skip_num_edge: bool) -> Option<Edge> {
        let edges = self.edge_lattice.get(&(start, end))?;

        let mut active_num_edge = None;
        let mut inactive_num_edge = None;
        let mut rom_edge = None;
        let mut decomp_edge = None;
        let mut other_edge = None;

        for edge in edges {
            match edge {
                Edge::Numeric { .. } if !skip_num_edge => {
                    if edge.is_active() {
                        if active_num_edge.is_none() {
                            active_num_edge = Some(edge.clone());
                        }
                    } else if inactive_num_edge.is_none() {
                        inactive_num_edge = Some(edge.clone());
                    }
                }
                Edge::Regular(data) => {
                    if data.r#type.starts_with("rom decomp") {
                        if decomp_edge.is_none() {
                            decomp_edge = Some(edge.clone());
                        }
                    } else if data.r#type.starts_with("rom") || data.r#type.starts_with("num") {
                        if rom_edge.is_none() {
                            rom_edge = Some(edge.clone());
                        }
                    } else if other_edge.is_none() {
                        other_edge = Some(edge.clone());
                    }
                }
                _ => {}
            }
        }

        // Priority: Active Numeric > rom > decomp > other > Inactive Numeric
        active_num_edge
            .or(rom_edge)
            .or(decomp_edge)
            .or(other_edge)
            .or(inactive_num_edge)
    }

    // fn find_first_rom_candidate_with_target(&self, start: usize, end: usize) -> Option<String> {
    //     let s = self.s_chars.get(start..end).map(|chars| chars.iter().collect::<String>())?;
    //     let rules = self.uroman.rom_rules.get(&s)?;
    //     for rule in rules {
    //         if let Some(t) = &rule.t
    //             && !t.is_empty() {
    //                 return Some(t.clone());
    //             }
    //     }
    //     None
    // }

    pub fn best_right_neighbor_edge(&self, start: usize, skip_num_edge: bool) -> Option<Edge> {
        if let Some(ends) = self.right_links.get(&start) {
            let mut sorted_ends: Vec<_> = ends.iter().collect();
            sorted_ends.sort_by(|a, b| b.cmp(a));

            for &end in sorted_ends {
                if let Some(best_edge) = self.best_edge_in_span(start, end, skip_num_edge) {
                    return Some(best_edge);
                }
            }
        }
        None
    }

    pub fn best_left_neighbor_edge(&self, end: usize, skip_num_edge: bool) -> Option<Edge> {
        if let Some(starts) = self.left_links.get(&end) {
            let mut sorted_starts: Vec<_> = starts.iter().collect();
            sorted_starts.sort();

            for &start in sorted_starts {
                if let Some(best_edge) = self.best_edge_in_span(start, end, skip_num_edge) {
                    return Some(best_edge);
                }
            }
        }
        None
    }

    fn find_rom_edge_path_backwards(
        &self,
        start: usize,
        end: usize,
        min_char_len: Option<usize>,
        return_str: bool,
        skip_num_edge: bool,
    ) -> BackwardsPathResult {
        let mut current_pos = end;

        if return_str {
            let mut rom_s = String::new();
            while current_pos > start {
                let old_pos = current_pos;
                if let Some(new_edge) = self.best_left_neighbor_edge(current_pos, skip_num_edge) {
                    rom_s.insert_str(0, new_edge.txt());
                    current_pos = new_edge.start();
                }

                if let Some(min_len) = min_char_len
                    && rom_s.len() >= min_len
                {
                    break;
                }
                if current_pos == old_pos {
                    current_pos -= 1;
                }
            }
            BackwardsPathResult::Str(rom_s)
        } else {
            let mut result_edges = Vec::new();
            while current_pos > start {
                let old_pos = current_pos;
                if let Some(new_edge) = self.best_left_neighbor_edge(current_pos, skip_num_edge) {
                    result_edges.insert(0, new_edge.clone());
                    current_pos = new_edge.start();
                }

                if let Some(min_len) = min_char_len {
                    let current_len: usize = result_edges.iter().map(|e| e.txt().len()).sum();
                    if current_len >= min_len {
                        break;
                    }
                }
                if current_pos == old_pos {
                    current_pos -= 1;
                }
            }
            BackwardsPathResult::Edges(result_edges)
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

    /// Parses Tibetan syllables to determine which characters contain an implicit vowel.
    /// The results from this method are stored in `self.props` and utilized by `add_default_abugida_vowel` later.
    pub fn pick_tibetan_vowel_edge(&mut self) {
        if !self
            .contains_script
            .get("Tibetan")
            .copied()
            .unwrap_or(false)
        {
            return;
        }

        let mut tibetan_syllables: Vec<Vec<usize>> = Vec::new();
        let mut current_syllable_positions: Vec<usize> = Vec::new();

        for (i, &c) in self.s_chars.iter().enumerate() {
            if self.uroman.chr_script_name(c) == "Tibetan" && self.char_is_letter_or_vowel_sign(c) {
                current_syllable_positions.push(i);
            } else if !current_syllable_positions.is_empty() {
                tibetan_syllables.push(current_syllable_positions);
                current_syllable_positions = Vec::new();
            }
        }
        if !current_syllable_positions.is_empty() {
            tibetan_syllables.push(current_syllable_positions);
        }

        for positions in tibetan_syllables {
            let mut vowel_pos: Option<usize> = None;
            let mut roms: Vec<String> = Vec::new();
            let mut subjoined_letter_positions = HashSet::new();
            let first_letter_position = *positions.first().unwrap_or(&0);

            for &i in &positions {
                let c = self.s_chars[i];
                let mut rom = self
                    .simple_top_romanization_candidate_for_span(i, i + 1, true)
                    .unwrap_or("?".to_string());
                self.props.insert(("edge-vowel".to_string(), i), None);

                if self.char_is_vowel_sign(c) {
                    vowel_pos = Some(i);
                    self.props.insert(("edge-vowel".to_string(), i), Some(true));
                    if roms.len() == 1 && roms[0] == "'" {
                        self.props
                            .insert(("edge-delete".to_string(), i - 1), Some(true));
                    }
                } else if self.char_is_subjoined_letter(c) {
                    subjoined_letter_positions.insert(i);
                    if i > first_letter_position {
                        // The special index `\u0FB0` (SUBJOINED -A) converts the previous character into a vowel
                        if c == '\u{0FB0}' {
                            vowel_pos = Some(i - 1);
                            self.props
                                .insert(("edge-vowel".to_string(), i - 1), Some(true));
                        } else {
                            // Other indices suppress the vowel of the previous character
                            self.props
                                .insert(("edge-vowel".to_string(), i - 1), Some(false));
                        }
                    }
                    rom = ROMS_CONSONANT_A_END_RE.replace(&rom, "$1").to_string();
                } else if c == '\u{0F60}' {
                    // TIBETAN LETTER -A
                    self.props
                        .insert(("edge-vowel".to_string(), i), Some(false));
                    if i > first_letter_position {
                        vowel_pos = Some(i - 1);
                        self.props
                            .insert(("edge-vowel".to_string(), i - 1), Some(true));
                        if i == *positions.last().unwrap() {
                            self.props
                                .insert(("edge-delete".to_string(), i), Some(true));
                        }
                    }
                    // If the previous character was a consonant, use "a'"; if a vowel, use "'"
                    rom = if roms.last().is_none_or(|r| !ROM_VOWEL_END_RE.is_match(r)) {
                        "a'".to_string()
                    } else {
                        "'".to_string()
                    };
                } else {
                    rom = ROMS_CONSONANT_A_END_RE.replace(&rom, "$1").to_string();
                }
                roms.push(rom);
            }

            if vowel_pos.is_some() {
                // If an explicit vowel is found, all other undetermined characters should be non-vowels
                for &i in &positions {
                    if self
                        .props
                        .get(&("edge-vowel".to_string(), i))
                        .and_then(|&v| v)
                        .is_none()
                    {
                        self.props
                            .insert(("edge-vowel".to_string(), i), Some(false));
                    }
                }
            } else {
                let mut best_cost = f64::INFINITY;
                let mut best_vowel_pos: Option<usize> = None;

                for (idx, &i) in positions.iter().enumerate() {
                    let pre: String = roms[..=idx].join("");
                    let post: String = roms[idx + 1..].join("");

                    let cost =
                        if self.props.get(&("edge-vowel".to_string(), i)) == Some(&Some(false)) {
                            20.0
                        } else if positions.len() == 1 {
                            0.0
                        } else if positions.len() == 2 {
                            if i == positions[0] { 0.0 } else { 0.1 }
                        } else {
                            let good_prefix = GOOD_PREFIX_RE.is_match(&pre);
                            let good_suffix = GOOD_SUFFIX_RE.is_match(&post);
                            let suffix_slice = positions.get(idx + 2..).unwrap_or(&[]);
                            let subjoined_suffix = suffix_slice
                                .iter()
                                .all(|p| subjoined_letter_positions.contains(p));

                            if good_prefix && good_suffix {
                                pre.len() as f64 * 0.1
                            } else if good_suffix {
                                pre.len() as f64
                            } else if subjoined_suffix && good_prefix {
                                pre.len() as f64 * 0.3
                            } else if subjoined_suffix {
                                pre.len() as f64 * 0.5
                            } else {
                                f64::INFINITY
                            }
                        };

                    if cost < best_cost {
                        best_cost = cost;
                        best_vowel_pos = Some(i);
                    }
                }

                if let Some(pos) = best_vowel_pos {
                    for &i in &positions {
                        self.props
                            .insert(("edge-vowel".to_string(), i), Some(i == pos));
                    }
                }
            }
        }
    }

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
                            .get(&("is-upper".to_string(), start))
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

    fn initialize_num_edges(&mut self) -> Vec<Edge> {
        let mut num_edges = Vec::new();
        for start in 0..self.s_chars.len() {
            if let Some(edge) =
                Edge::new_numeric(start, start + 1, self.s_chars[start], self.uroman)
            {
                self.add_edge(edge.clone());
                num_edges.push(edge);
            }
        }
        num_edges
    }

    // Implements numeric processing by chaining rule-based passes. This layered approach
    // safely replicates the dynamic logic of the original Python implementation in Rust,
    // avoiding complex borrowing and mutation issues.
    pub fn add_numbers(&mut self) {
        let mut active_edges = self.initialize_num_edges();

        active_edges = self.apply_d1_digits(active_edges);
        active_edges = self.apply_g1_multiplication(active_edges);
        active_edges = self.apply_g2_addition(active_edges);
        active_edges = self.apply_g3_large_power_multiplication(active_edges);
        active_edges = self.apply_g4_large_block_addition(active_edges);

        self.apply_fraction_and_percentage_patterns();

        self.apply_g6_plus_minus_signs(&active_edges);
        self.apply_f1_final_adjustments();
        self.deactivate_exceptional_singles(&mut active_edges);
        self.add_fallback_unicode_numbers();
    }

    #[inline]
    fn apply_d1_digits(&mut self, prev_pass_edges: Vec<Edge>) -> Vec<Edge> {
        let mut next_pass_edges = Vec::new();
        let mut i = 0;
        while i < prev_pass_edges.len() {
            let start_edge = &prev_pass_edges[i];

            if !Self::edge_is_digit(start_edge) {
                next_pass_edges.push(start_edge.clone());
                i += 1;
                continue;
            }

            let mut sub_edges = vec![start_edge.clone()];
            let mut combined_orig_txt = start_edge.orig_txt().to_string();
            let mut current_val_s = start_edge.txt().to_string();
            let mut n_decimals = None;
            let mut last_edge_end = start_edge.end();
            let mut j = i + 1;

            'seq: while j < prev_pass_edges.len() {
                let next_edge = &prev_pass_edges[j];
                if next_edge.start() != last_edge_end {
                    break 'seq;
                }

                if Self::edge_is_digit(next_edge) {
                    sub_edges.push(next_edge.clone());
                    combined_orig_txt.push_str(next_edge.orig_txt());
                    current_val_s.push_str(next_edge.txt());
                    if n_decimals.is_some() {
                        n_decimals = n_decimals.map(|n| n + 1);
                    }
                    last_edge_end = next_edge.end();
                } else if n_decimals.is_none()
                    && self.s_chars.get(last_edge_end..last_edge_end + 1) == Some(&['.'])
                {
                    if let Some(digit_after_dot) = prev_pass_edges
                        .get(j)
                        .filter(|e| e.start() == last_edge_end + 1 && Self::edge_is_digit(e))
                    {
                        sub_edges.push(digit_after_dot.clone());
                        combined_orig_txt.push('.');
                        combined_orig_txt.push_str(digit_after_dot.orig_txt());
                        current_val_s.push('.');
                        current_val_s.push_str(digit_after_dot.txt());
                        n_decimals = Some(1);
                        last_edge_end = digit_after_dot.end();
                        j += 1;
                    } else {
                        break 'seq;
                    }
                } else {
                    break 'seq;
                }
                j += 1;
            }

            if sub_edges.len() > 1 {
                let new_value = current_val_s
                    .parse::<f64>()
                    .expect("D1 value parsing failed");
                let mut new_edge = Edge::new_combined_numeric(
                    start_edge.start(),
                    last_edge_end,
                    new_value,
                    "D1".to_string(),
                    sub_edges.last().unwrap().get_script(),
                    Some(1),
                    n_decimals,
                    combined_orig_txt,
                );

                let updates = NumDataUpdates {
                    value: Some(new_value),
                    value_s: Some(current_val_s),
                    ..Default::default()
                };
                new_edge.update(updates);

                self.add_edge(new_edge.clone());
                next_pass_edges.push(new_edge);
                i += sub_edges.len();
            } else {
                next_pass_edges.push(start_edge.clone());
                i += 1;
            }
        }
        next_pass_edges
    }

    #[inline]
    fn apply_g1_multiplication(&mut self, prev_pass_edges: Vec<Edge>) -> Vec<Edge> {
        let mut next_pass_edges = Vec::new();
        let mut i = 0;
        while i < prev_pass_edges.len() {
            if i + 1 < prev_pass_edges.len() {
                let left = &prev_pass_edges[i];
                let right = &prev_pass_edges[i + 1];

                let (left_val, is_single_digit) = left.get_num_data().map_or((0.0, false), |d| {
                    (
                        d.value.unwrap_or(0.0),
                        d.num_base == Some(1) && d.value.is_some_and(|v| v >= 1.0),
                    )
                });
                let (right_val, right_base, is_base_number) =
                    right.get_num_data().map_or((0.0, None, false), |d| {
                        (
                            d.value.unwrap_or(0.0),
                            d.num_base,
                            d.num_base.is_some_and(|b| b > 1) && !d.is_large_power,
                        )
                    });

                if left.end() == right.start() && is_single_digit && is_base_number {
                    let new_value = left_val * right_val;
                    let new_edge = Edge::new_combined_numeric(
                        left.start(),
                        right.end(),
                        new_value,
                        "G1".to_string(),
                        right.get_script(),
                        right_base,
                        None,
                        format!("{}{}", left.orig_txt(), right.orig_txt()),
                    );
                    self.add_edge(new_edge.clone());
                    next_pass_edges.push(new_edge);
                    i += 2;
                    continue;
                }
            }
            next_pass_edges.push(prev_pass_edges[i].clone());
            i += 1;
        }
        next_pass_edges
    }

    #[inline]
    fn apply_g2_addition(&mut self, prev_pass_edges: Vec<Edge>) -> Vec<Edge> {
        let mut next_pass_edges = Vec::new();
        let mut i = 0;
        while i < prev_pass_edges.len() {
            let start_edge = &prev_pass_edges[i];
            if start_edge.is_large_power() || start_edge.value().is_none() {
                next_pass_edges.push(start_edge.clone());
                i += 1;
                continue;
            }

            let mut sub_edges = vec![start_edge.clone()];
            let mut last_edge_end = start_edge.end();
            let mut j = i + 1;

            while let Some(right_edge) = prev_pass_edges.get(j) {
                if right_edge.start() != last_edge_end
                    || !right_edge.is_active()
                    || right_edge.is_large_power()
                {
                    break;
                }

                let prev_non_null = sub_edges.iter().rev().find(|e| !Self::is_gap_null_edge(e));
                let can_combine = if prev_non_null.is_some_and(|p| p.is_large_power()) {
                    false
                } else if let (Some(prev), Some(right_val), Some(right_base)) =
                    (prev_non_null, right_edge.value(), right_edge.get_num_base())
                {
                    prev.get_num_base().unwrap_or(0) > right_val as i64
                        && prev.get_num_base().unwrap_or(0) > right_base
                } else {
                    Self::is_gap_null_edge(right_edge)
                };

                if can_combine {
                    sub_edges.push(right_edge.clone());
                    last_edge_end = right_edge.end();
                    j += 1;
                } else {
                    break;
                }
            }

            if sub_edges.len() > 1 {
                let new_value: f64 = sub_edges.iter().map(|e| e.value().unwrap_or(0.0)).sum();
                let last = sub_edges.last().unwrap();
                let new_edge = Edge::new_combined_numeric(
                    start_edge.start(),
                    last.end(),
                    new_value,
                    "G2".to_string(),
                    last.get_script(),
                    last.get_num_base(),
                    None,
                    sub_edges.iter().map(|e| e.orig_txt()).collect::<String>(),
                );
                self.add_edge(new_edge.clone());
                next_pass_edges.push(new_edge);
                i += sub_edges.len();
            } else {
                next_pass_edges.push(start_edge.clone());
                i += 1;
            }
        }
        next_pass_edges
    }

    #[inline]
    fn apply_g3_large_power_multiplication(&mut self, prev_pass_edges: Vec<Edge>) -> Vec<Edge> {
        let mut next_pass_edges = Vec::new();
        let mut i = 0;
        while i < prev_pass_edges.len() {
            if i + 1 < prev_pass_edges.len() {
                let left = &prev_pass_edges[i];
                let right = &prev_pass_edges[i + 1];

                if left.end() == right.start()
                    && left.is_active()
                    && !left.is_large_power()
                    && left.value().is_some()
                    && right.is_active()
                    && right.is_large_power()
                    && right.value().is_some()
                {
                    let new_value = left.value().unwrap() * right.value().unwrap();
                    let mut new_edge = Edge::new_combined_numeric(
                        left.start(),
                        right.end(),
                        new_value,
                        "G3".to_string(),
                        right.get_script(),
                        right.get_num_base(),
                        None,
                        format!("{}{}", left.orig_txt(), right.orig_txt()),
                    );
                    if let Some(nd) = new_edge.get_num_data_mut() {
                        nd.is_large_power = true;
                    }
                    self.add_edge(new_edge.clone());
                    next_pass_edges.push(new_edge);
                    i += 2;
                    continue;
                }
            }
            next_pass_edges.push(prev_pass_edges[i].clone());
            i += 1;
        }
        next_pass_edges
    }

    #[inline]
    fn apply_g4_large_block_addition(&mut self, prev_pass_edges: Vec<Edge>) -> Vec<Edge> {
        let mut next_pass_edges = Vec::new();
        let mut i = 0;
        while i < prev_pass_edges.len() {
            let start_edge = &prev_pass_edges[i];
            if start_edge.value().is_none() {
                next_pass_edges.push(start_edge.clone());
                i += 1;
                continue;
            }

            let mut sub_edges = vec![start_edge.clone()];
            let mut last_edge_end = start_edge.end();
            let mut j = i + 1;

            while let Some(raw_right_edge) = prev_pass_edges.get(j) {
                let mut right_edge = raw_right_edge.clone();
                if right_edge.start() != last_edge_end
                    || !right_edge.is_active()
                    || right_edge.value().is_none()
                {
                    break;
                }

                let prev_edge = sub_edges.last().unwrap();
                let prev_base = prev_edge.get_num_base().unwrap_or(0);

                if prev_edge.get_script().as_deref() == Some("CJK")
                    && prev_base >= 1000
                    && is_power_of_10(prev_base)
                    && right_edge.end() - right_edge.start() == 1
                    && (1.0..=9.0).contains(&right_edge.value().unwrap())
                    && let Some(num_data) = right_edge.get_num_data_mut()
                {
                    let new_num_base = prev_base / 10;
                    num_data.value = Some(new_num_base as f64 * num_data.value.unwrap());
                    num_data.num_base = Some(new_num_base);
                    right_edge.get_data_mut().r#type = "G4tag".to_string();
                }

                let can_combine = if let (Some(right_val), Some(right_base)) =
                    (right_edge.value(), right_edge.get_num_base())
                {
                    prev_base > right_val as i64 && prev_base > right_base
                } else {
                    false
                };

                if can_combine {
                    sub_edges.push(right_edge);
                    last_edge_end = sub_edges.last().unwrap().end();
                    j += 1;
                } else {
                    break;
                }
            }

            if sub_edges.len() > 1 {
                let new_value: f64 = sub_edges.iter().map(|e| e.value().unwrap_or(0.0)).sum();
                let last = sub_edges.last().unwrap();
                let new_edge = Edge::new_combined_numeric(
                    start_edge.start(),
                    last.end(),
                    new_value,
                    "G4".to_string(),
                    last.get_script(),
                    last.get_num_base(),
                    None,
                    sub_edges.iter().map(|e| e.orig_txt()).collect::<String>(),
                );
                self.add_edge(new_edge.clone());
                next_pass_edges.push(new_edge);
                i += sub_edges.len();
            } else {
                next_pass_edges.push(start_edge.clone());
                i += 1;
            }
        }
        next_pass_edges
    }

    #[inline]
    fn apply_fraction_and_percentage_patterns(&mut self) {
        let mut new_edges: Vec<Edge> = Vec::new();
        let mut edges_to_deactivate: Vec<Edge> = Vec::new();

        // Combine all markers and sort them by length descending.
        // This ensures that longer markers (like "百分之") are matched before shorter ones.
        let mut markers: Vec<_> = self.uroman.percentage_markers.iter()
            .map(|m| (m, "percentage"))
            .chain(self.uroman.fraction_connectors.iter().map(|c| (c, "fraction")))
            .collect();
        markers.sort_by_key(|(b, _)| std::cmp::Reverse(b.len()));

        // Use a label to efficiently skip to the next start position once a match is found.
        'outer: for start in 0..self.s_chars.len() {
            if let Some((marker_str, marker_type)) = markers.iter().find(|(m, _)| self.s_chars[start..].starts_with(&m.chars().collect::<Vec<_>>())) {

                let marker_end = start + marker_str.chars().count();

                // --- (Number) + Marker/Connector + (Number) ---
                // Handles cases like "10 / 1", "十分之一", and the special case "百分之一".
                // The `false` argument is crucial: it means "do NOT skip numeric edges".
                if let Some(left_edge) = self.best_left_neighbor_edge(start, false)
                    && let Some(right_edge) = self.best_right_neighbor_edge(marker_end, false) {
                        // Ensure the pattern is contiguous and connected to the marker.
                        if left_edge.end() != start || right_edge.start() != marker_end { continue; }

                        if let (Some(left_val), Some(right_val)) = (
                            left_edge.value().and_then(|v| if v.fract() == 0.0 { Some(v as i64) } else { None }),
                            right_edge.value().and_then(|v| if v.fract() == 0.0 { Some(v as i64) } else { None })
                        ) {
                            let combined_start = left_edge.start();
                            let combined_end = right_edge.end();
                            let mut consumed = false;

                            // if the left value is 100 and the connector is a fraction type (like "分之"),
                            // treat it as a percentage. This correctly handles "百分之一".
                            // This also handles explicit percentage markers like "100 % 1".
                            if left_val == 100 && (*marker_type == "fraction" || *marker_type == "percentage") {
                                new_edges.push(Edge::new_regular(combined_start, combined_end, format!("{right_val}%"), "percentage".to_string()));
                                consumed = true;
                            } else if *marker_type == "fraction" && left_val != 0 {
                                // Standard fraction case like "十分之一".
                                let fraction = Ratio::new(right_val, left_val);
                                new_edges.push(Edge::Numeric {
                                    data: EdgeData {
                                        start: combined_start,
                                        end: combined_end,
                                        txt: format!("{right_val}/{left_val}"),
                                        r#type: "fraction".to_string(),
                                    },
                                    num_data: NumData {
                                        orig_txt: format!("{}/{}", right_val, left_val),
                                        value: None,
                                        fraction: Some(fraction),
                                        script: right_edge.get_script(),
                                        active: true,
                                        ..Default::default()
                                    },
                                });
                                consumed = true;
                            }

                            if consumed {
                                edges_to_deactivate.push(left_edge.clone());
                                edges_to_deactivate.push(right_edge.clone());
                                // Since we found a match for this `start` position, continue to the next `start`.
                                continue 'outer;
                            }
                        }
                    }

                // --- Marker + (Number) ---
                // Handles cases like "百分之" + "一". This is the primary path for `percentage-marker`.
                // The `false` argument is crucial: it means "do NOT skip numeric edges".
                if *marker_type == "percentage"
                    && let Some(right_edge) = self.best_right_neighbor_edge(marker_end, false) {
                        if right_edge.start() != marker_end || !right_edge.is_numeric() {
                            continue 'outer;
                        }

                        new_edges.push(Edge::new_regular(
                            start,
                            right_edge.end(),
                            format!("{}%", right_edge.txt()),
                            "percentage".to_string(),
                        ));
                        edges_to_deactivate.push(right_edge.clone());
                    }
                }
        }

        // Apply the collected changes to the lattice.
        for edge in edges_to_deactivate {
            if let Some(edges) = self.edge_lattice.get_mut(&(edge.start(), edge.end()))
                && let Some(mut e) = edges.take(&edge) {
                    e.set_active(false);
                    edges.insert(e);
                }
        }
        for edge in new_edges {
            self.add_edge(edge);
        }
    }

    #[inline]
    fn apply_g6_plus_minus_signs(&mut self, active_edges: &[Edge]) {
        for edge in active_edges.iter().filter(|e| e.value().is_some()) {
            let edge_start_char_idx = edge.start();

            for minus_sign in &self.uroman.minus_signs {
                let sign_char_len = minus_sign.chars().count();
                let sign_as_chars: Vec<char> = minus_sign.chars().collect();
                if let Some(start_pos_char_idx) = edge_start_char_idx.checked_sub(sign_char_len)
                    && self.s_chars.get(start_pos_char_idx..edge_start_char_idx)
                        == Some(sign_as_chars.as_slice())
                {
                    let new_edge = Edge::new_regular(
                        start_pos_char_idx,
                        edge.end(),
                        format!("-{}", edge.txt()),
                        format!("{} -", edge.r#type()),
                    );
                    self.add_edge(new_edge);
                }
            }

            for plus_sign in &self.uroman.plus_signs {
                let sign_char_len = plus_sign.chars().count();
                let sign_as_chars: Vec<char> = plus_sign.chars().collect();
                if let Some(start_pos_char_idx) = edge_start_char_idx.checked_sub(sign_char_len)
                    && self.s_chars.get(start_pos_char_idx..edge_start_char_idx)
                        == Some(sign_as_chars.as_slice())
                {
                    let new_edge = Edge::new_regular(
                        start_pos_char_idx,
                        edge.end(),
                        format!("+{}", edge.txt()),
                        format!("{} +", edge.r#type()),
                    );
                    self.add_edge(new_edge);
                }
            }
        }
    }

    #[inline]
    fn apply_f1_final_adjustments(&mut self) {
        let mut edges_to_add: Vec<Edge> = Vec::new();
        let mut edges_to_deactivate: Vec<Edge> = Vec::new();

        for (_, left_edges) in self.edge_lattice.iter() {
            for left_edge in left_edges.iter() {
                if !ENDS_WITH_DIGIT_RE.is_match(left_edge.txt()) {
                    continue;
                }

                if let Some(right_link_ends) = self.right_links.get(&left_edge.end()) {
                    for right_end in right_link_ends {
                        if let Some(right_edges) = self.edge_lattice.get(&(left_edge.end(), *right_end)) {
                            for right_edge in right_edges {
                                if right_edge.is_numeric() && STARTS_WITH_DIGIT_RE.is_match(right_edge.txt()) {
                                    let mut new_edge = right_edge.clone();
                                    let has_fraction = new_edge.get_num_data().is_some_and(|d| d.fraction.is_some());
                                    let separator = if has_fraction { " " } else { "·" };
                                    new_edge.get_data_mut().txt.insert_str(0, separator);

                                    edges_to_add.push(new_edge);
                                    edges_to_deactivate.push(right_edge.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        for edge_to_deactivate in edges_to_deactivate {
            if let Some(edges) = self.edge_lattice.get_mut(&(edge_to_deactivate.start(), edge_to_deactivate.end()))
                && let Some(mut e) = edges.take(&edge_to_deactivate) {
                    e.set_active(false);
                    edges.insert(e);
                }
        }

        for new_edge in edges_to_add {
            self.add_edge(new_edge);
        }
    }

    #[inline]
    fn deactivate_exceptional_singles(&mut self, num_edges: &mut [Edge]) {
        let exceptional_chars: HashSet<&str> = ["兩", "参", "參", "伍", "陆", "陸", "什", "京兆"]
            .into_iter()
            .collect();

        for edge in num_edges.iter_mut() {
            if !edge.is_active() {
                continue;
            }
            let mut should_deactivate = false;
            if let Some(num_data) = edge.get_num_data() {
                if edge.end() - edge.start() == 1 && num_data.value.is_some_and(|v| v > 1000.0) {
                    should_deactivate = true;
                }
                if exceptional_chars.contains(num_data.orig_txt.as_str()) {
                    should_deactivate = true;
                }
            }
            if should_deactivate {
                edge.set_active(false);
            }
        }
    }

    #[inline]
    fn add_fallback_unicode_numbers(&mut self) {
        let mut new_edges_to_add = Vec::new();
        for (start, &char) in self.s_chars.iter().enumerate() {
            let end = start + 1;
            if self
                .edge_lattice
                .get(&(start, end))
                .is_some_and(|edges| edges.iter().any(|e| e.is_numeric()))
            {
                continue;
            }
            if char.general_category() == GeneralCategory::DecimalNumber
                && let Some(digit_val) = char.to_digit(10)
            {
                new_edges_to_add.push(Edge::new_regular(
                    start,
                    end,
                    digit_val.to_string(),
                    "num".to_string(),
                ));
            }
        }
        for edge in new_edges_to_add {
            self.add_edge(edge);
        }
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
                        base_rom_plus_vowel =
                            Some(format!("{}{}", rom, &script.abugida_default_vowels[0]));
                    }

                    if let Some(br) = &base_rom
                        && !(ABUGIDA_CONSONANT_RE.is_match(br)
                            || (script_name == "Tibetan" && br == "'"))
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
                .get(&("edge-delete".to_string(), start))
                .copied()
                .flatten()
                .unwrap_or(false)
            {
                return "".to_string();
            } else if self
                .props
                .get(&("edge-vowel".to_string(), start))
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
        if end - start == 1
            && rom.chars().all(|c| c.is_uppercase())
            && next_char.is_some_and(|nc| nc.is_lowercase())
            && let Some(first) = rom.chars().next()
        {
            rom = first.to_uppercase().to_string() + &rom[1..].to_lowercase();
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
                    match self.find_rom_edge_path_backwards(0, start, Some(1), true, false) {
                        BackwardsPathResult::Str(s) => s,
                        _ => "".to_string(),
                    }
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

    fn romanization_by_first_rule(&self, s: &str) -> Option<String> {
        self.uroman
            .rom_rules
            .get(s)
            .and_then(|rules| rules.first())
            .and_then(|rule| rule.t.clone())
    }

    pub fn add_alternatives(&mut self, edges: &mut Vec<Edge>) {
        let mut existing_edges: HashSet<(usize, usize, String)> = edges
            .iter()
            .map(|e| (e.start(), e.end(), e.txt().to_string()))
            .collect();

        let mut new_edges_to_add = Vec::new();

        for base_edge in edges.clone() {
            // Python: if old_edge.type.startswith('rom-alt'): continue
            if base_edge.r#type().starts_with("rom-alt") {
                continue;
            }

            let start = base_edge.start();
            let end = base_edge.end();

            let orig_s: String = self.s_chars[start..end].iter().collect();
            let old_rom = base_edge.txt();

            let (old_rom_core, old_rom_suffix) =
                if let Some(caps) = ROM_SUFFIX_RE.captures(base_edge.r#type()) {
                    (
                        caps.get(1).map(|m| m.as_str()),
                        caps.get(2).map(|m| m.as_str()),
                    )
                } else {
                    (None, None)
                };

            if let Some(rom_rules) = self.uroman.rom_rules.get(&orig_s) {
                for rom_rule in rom_rules {
                    if !self.cand_is_valid(rom_rule, start, end) {
                        continue;
                    }

                    let Some(rom_t) = &rom_rule.t else {
                        continue;
                    };

                    if (rom_t == old_rom || old_rom_core == Some(rom_t.as_str()))
                        && !rom_rule.t_alts.is_empty()
                    {
                        for rom_alt_base in &rom_rule.t_alts {
                            let mut rom_alt = rom_alt_base.clone();
                            if let (Some(core), Some(suffix)) = (old_rom_core, old_rom_suffix)
                                && rom_t == core
                            {
                                rom_alt.push_str(suffix);
                            }

                            if existing_edges.insert((start, end, rom_alt.clone())) {
                                new_edges_to_add.push(Edge::new_regular(
                                    start,
                                    end,
                                    rom_alt,
                                    "rom-alt".to_string(),
                                ));
                            }
                        }
                    }

                    if let Some(rom_end_of_syllable) = &rom_rule.t_at_end_of_syllable {
                        if rom_t == old_rom && existing_edges.insert((start, end, rom_t.clone())) {
                            new_edges_to_add.push(Edge::new_regular(
                                start,
                                end,
                                rom_t.clone(),
                                "rom-alt2".to_string(),
                            ));
                        }

                        if rom_end_of_syllable == old_rom
                            && existing_edges.insert((start, end, rom_t.clone()))
                        {
                            new_edges_to_add.push(Edge::new_regular(
                                start,
                                end,
                                rom_t.clone(),
                                "rom-alt3".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        edges.extend(new_edges_to_add);
    }

    pub fn best_rom_edge_path(&mut self, start: usize, end: usize, skip_num_edge: bool) -> Vec<Edge> {
        let mut result = Vec::new();
        let mut current_pos = start;
        while current_pos < end {
            if let Some(best_edge) = self.best_right_neighbor_edge(current_pos, skip_num_edge) {
                current_pos = best_edge.end();
                result.push(best_edge);
            } else {
                // let skipped_char = self.s_chars.get(current_pos).unwrap();
                // println!(
                //     "DEBUG: No edge found at position {}. Skipping character: '{}' (U+{:X})",
                //     current_pos,
                //     skipped_char,
                //     *skipped_char as u32
                // );
                current_pos += 1;
            }
        }
        result
    }

    pub fn _print_all_edges_for_debug(&self, step_name: &str) {
        println!("\n--- Rust: After {step_name} ---");

        let mut all_edges: Vec<Edge> = self.edge_lattice.values().flatten().cloned().collect();

        all_edges.sort_by(|a, b| {
            let this = &a;
            {
                let this = &this;
                let data = this.get_data();
                (data.start, data.end, &data.txt, &data.r#type)
            }.cmp(&{
                let this = &b;
                let data = this.get_data();
                (data.start, data.end, &data.txt, &data.r#type)
            })
        });

        if all_edges.is_empty() {
            println!("(No edges in lattice)");
            return;
        }

        for edge in all_edges {
            println!("{edge:?}");
        }
    }
}

fn is_power_of_10(mut n: i64) -> bool {
    if n <= 0 {
        return false;
    }
    while n % 10 == 0 {
        n /= 10;
    }
    n == 1
}
