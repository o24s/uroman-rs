//! Parses the bundled uroman data files into [`Dictionaries`].
//!
//! This is a faithful port of the loading logic of the original `uroman.py`
//! (and of the earlier `core.rs`): file order, rule-overwrite semantics and
//! per-line side effects are preserved exactly. It runs at build time (from
//! `build.rs`) to compile the dictionaries into an archive.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value as JsonValue;
use unicode_normalization::UnicodeNormalization;
use unicode_properties::UnicodeGeneralCategory;

use super::model::{
    AbugidaRuleType, CharRangeMap, CharRanges, Dictionaries, NumProp, RomRule, ScriptInfo,
};
use super::strutil::{
    dequote_string, has_value_in_double_colon_del_list, slot_value_in_double_colon_del_list,
};

static KAYAH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"kayah\s+(\S+)\s*$").unwrap());
static MENDE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"m\d+\s+(\S+)\s*$").unwrap());
static SPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\S\s+\S").unwrap());

/// The raw contents of the data files, in the layout of the `data/` directory.
pub struct DataFiles<'a> {
    pub romanization_auto_table: &'a str,
    pub unicode_data_overwrite: &'a str,
    pub romanization_table: &'a str,
    pub chinese_to_pinyin: &'a str,
    pub scripts: &'a str,
    pub unicode_data_props: [&'a str; 3],
    pub num_props: &'a str,
    pub unicode_data: &'a str,
}

/// Builds the complete dictionary set from the data files.
pub fn build(files: &DataFiles) -> Dictionaries {
    let mut b = Builder::default();
    b.load_rom_file(files.romanization_auto_table, "ud", "rom");
    b.load_rom_file(files.unicode_data_overwrite, "ow", "u2r");
    b.load_rom_file(files.romanization_table, "man", "rom");
    b.load_chinese_pinyin_file(files.chinese_to_pinyin);
    b.load_script_file(files.scripts);
    for props in files.unicode_data_props {
        b.load_unicode_data_props(props);
    }
    b.load_num_props(files.num_props);
    b.add_thai_cancellation_rules();
    b.load_decompositions(files.unicode_data);
    b.finalize()
}

/// A fully parsed rule line, including load-time-only attributes that are
/// folded into other collections by [`Builder::finalize`].
struct ParsedRule {
    s: String,
    t: Option<String>,
    prov: String,
    lcodes: Vec<String>,
    use_only_at_start_of_word: bool,
    dont_use_at_start_of_word: bool,
    use_only_at_end_of_word: bool,
    dont_use_at_end_of_word: bool,
    use_only_for_whole_word: bool,
    n_restr: u32,
    t_alts: Vec<String>,
    is_minus_sign: bool,
    is_plus_sign: bool,
    fraction_connector: bool,
    percentage_marker: bool,
    t_at_end_of_syllable: Option<String>,
}

impl ParsedRule {
    fn new_simple(s: String, t: &str, provenance: &str) -> Self {
        Self {
            s,
            t: Some(t.to_string()),
            prov: provenance.to_string(),
            lcodes: Vec::new(),
            use_only_at_start_of_word: false,
            dont_use_at_start_of_word: false,
            use_only_at_end_of_word: false,
            dont_use_at_end_of_word: false,
            use_only_for_whole_word: false,
            n_restr: 0,
            t_alts: Vec::new(),
            is_minus_sign: false,
            is_plus_sign: false,
            fraction_connector: false,
            percentage_marker: false,
            t_at_end_of_syllable: None,
        }
    }

    /// No contextual restrictions; crucial for the overwrite logic in
    /// [`Builder::add_rom_rule`].
    fn is_unconditional(&self) -> bool {
        self.lcodes.is_empty()
            && !self.use_only_at_start_of_word
            && !self.dont_use_at_start_of_word
            && !self.use_only_at_end_of_word
            && !self.dont_use_at_end_of_word
            && !self.use_only_for_whole_word
    }

    fn to_runtime(&self) -> RomRule {
        RomRule {
            t: self.t.clone(),
            lcodes: self.lcodes.clone(),
            use_only_at_start_of_word: self.use_only_at_start_of_word,
            dont_use_at_start_of_word: self.dont_use_at_start_of_word,
            use_only_at_end_of_word: self.use_only_at_end_of_word,
            dont_use_at_end_of_word: self.dont_use_at_end_of_word,
            use_only_for_whole_word: self.use_only_for_whole_word,
            n_restr: self.n_restr,
            t_alts: self.t_alts.clone(),
            t_at_end_of_syllable: self.t_at_end_of_syllable.clone(),
        }
    }
}

/// Full script record as parsed from `Scripts.txt`; only abugida-relevant
/// parts survive into [`Dictionaries`].
#[derive(Clone)]
struct FullScript {
    abugida_default_vowels: Vec<String>,
    abugida_rule_type: Option<AbugidaRuleType>,
}

#[derive(Default)]
struct Builder {
    rom_rules: HashMap<String, Vec<ParsedRule>>,
    s_prefixes: HashSet<String>,
    scripts: HashMap<String, FullScript>,
    char_scripts: HashMap<char, String>,
    vowel_signs: HashSet<char>,
    medial_consonant_signs: HashSet<char>,
    viramas: HashSet<char>,
    tone_marks: HashMap<char, String>,
    syllable_infos: HashMap<char, String>,
    char_names: HashMap<char, String>,
    num_props: HashMap<String, NumProp>,
    decompositions: HashMap<char, (String, String)>,
    percentage_markers: HashSet<String>,
    fraction_connectors: HashSet<String>,
    plus_signs: HashSet<String>,
    minus_signs: HashSet<String>,
}

impl Builder {
    /// Registers all prefixes of a string `s` for incremental span search.
    fn register_s_prefix(&mut self, s: &str) {
        let mut prefix = String::with_capacity(s.len());
        for c in s.chars() {
            prefix.push(c);
            if !self.s_prefixes.contains(prefix.as_str()) {
                self.s_prefixes.insert(prefix.clone());
            }
        }
    }

    fn chr_name(&self, c: char) -> String {
        if let Some(name) = self.char_names.get(&c) {
            return name.clone();
        }
        unicode_names2::name(c)
            .map(|n| n.to_string())
            .unwrap_or_default()
    }

    /// Post-processes a romanization target: extracts the essential part of
    /// verbose Kayah/Mende names, and rejects multi-word romanizations.
    fn second_rom_filter(&self, c: &str, rom: Option<&str>) -> Option<String> {
        if c.is_empty() {
            return rom.map(|s| s.to_string());
        }

        let rom_str = match rom {
            Some(r) if r.contains(' ') => r,
            _ => return rom.map(|s| s.to_string()),
        };

        let name = self.chr_name(c.chars().next().unwrap());

        if name.contains("MYANMAR VOWEL SIGN KAYAH")
            && let Some(cap) = KAYAH_RE.captures(rom_str)
        {
            return Some(cap.get(1).unwrap().as_str().to_string());
        }
        if name.contains("MENDE KIKAKUI SYLLABLE")
            && let Some(cap) = MENDE_RE.captures(rom_str)
        {
            return Some(cap.get(1).unwrap().as_str().to_string());
        }
        if SPACE_RE.is_match(rom_str) {
            return Some(c.to_string());
        }

        rom.map(|s| s.to_string())
    }

    fn parse_rule_line(
        &self,
        line: &str,
        provenance: &str,
        file_format: &str,
    ) -> Option<ParsedRule> {
        let (s, t) = if file_format == "u2r" {
            let u_str = slot_value_in_double_colon_del_list(line, "u")?;
            let cp = u32::from_str_radix(u_str, 16).ok()?;
            let s = std::char::from_u32(cp)?.to_string();
            let t = slot_value_in_double_colon_del_list(line, "r")
                .map(|s_val| dequote_string(s_val).to_string());
            (s, t)
        } else {
            let s = slot_value_in_double_colon_del_list(line, "s")
                .map(|s_val| dequote_string(s_val).to_string())?;
            let t = slot_value_in_double_colon_del_list(line, "t")
                .map(|s_val| dequote_string(s_val).to_string());
            (s, t)
        };

        let t = self.second_rom_filter(&s, t.as_deref());

        let lcodes: Vec<String> = slot_value_in_double_colon_del_list(line, "lcode")
            .map(|s| {
                s.split([',', ';'])
                    .map(|part| part.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();

        let use_only_at_start_of_word =
            has_value_in_double_colon_del_list(line, "use-only-at-start-of-word");
        let dont_use_at_start_of_word =
            has_value_in_double_colon_del_list(line, "dont-use-at-start-of-word");
        let use_only_at_end_of_word =
            has_value_in_double_colon_del_list(line, "use-only-at-end-of-word");
        let dont_use_at_end_of_word =
            has_value_in_double_colon_del_list(line, "dont-use-at-end-of-word");
        let use_only_for_whole_word =
            has_value_in_double_colon_del_list(line, "use-only-for-whole-word");

        let t_alts: Vec<String> = slot_value_in_double_colon_del_list(line, "t-alt")
            .map(|s| {
                s.split([',', ';'])
                    .map(|part| dequote_string(part).to_string())
                    .collect()
            })
            .unwrap_or_default();

        let is_minus_sign = has_value_in_double_colon_del_list(line, "is-minus-sign");
        let is_plus_sign = has_value_in_double_colon_del_list(line, "is-plus-sign");
        let fraction_connector = has_value_in_double_colon_del_list(line, "fraction-connector");
        let percentage_marker = has_value_in_double_colon_del_list(line, "percentage-marker");
        let t_at_end_of_syllable = slot_value_in_double_colon_del_list(line, "t-end-of-syllable")
            .map(|s_val| dequote_string(s_val).to_string());

        let n_restr = [
            !lcodes.is_empty(),
            use_only_at_start_of_word,
            dont_use_at_start_of_word,
            use_only_at_end_of_word,
            dont_use_at_end_of_word,
            use_only_for_whole_word,
        ]
        .iter()
        .filter(|&&is_restr| is_restr)
        .count() as u32;

        Some(ParsedRule {
            s,
            t,
            prov: provenance.to_string(),
            lcodes,
            use_only_at_start_of_word,
            dont_use_at_start_of_word,
            use_only_at_end_of_word,
            dont_use_at_end_of_word,
            use_only_for_whole_word,
            n_restr,
            t_alts,
            is_minus_sign,
            is_plus_sign,
            fraction_connector,
            percentage_marker,
            t_at_end_of_syllable,
        })
    }

    fn load_rom_file(&mut self, file: &str, provenance: &str, file_format: &str) {
        for line in file.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            if file_format == "u2r" {
                let u_str = match slot_value_in_double_colon_del_list(line, "u") {
                    Some(s) => s,
                    None => continue,
                };

                let c = match u32::from_str_radix(u_str, 16)
                    .ok()
                    .and_then(std::char::from_u32)
                {
                    Some(c) => c,
                    None => continue,
                };

                if let Some(tone_mark) = slot_value_in_double_colon_del_list(line, "tone-mark") {
                    self.tone_marks.insert(c, tone_mark.to_string());
                }
                if let Some(syllable_info) =
                    slot_value_in_double_colon_del_list(line, "syllable-info")
                {
                    self.syllable_infos.insert(c, syllable_info.to_string());
                }
                if let Some(name) = slot_value_in_double_colon_del_list(line, "name") {
                    self.char_names.insert(c, name.to_string());
                }

                if let Some(rule) = self.parse_rule_line(line, provenance, file_format) {
                    self.add_rom_rule(rule);
                }
            } else if let Some(rule) = self.parse_rule_line(line, provenance, file_format) {
                self.add_rom_rule(rule);
            }
        }
    }

    fn add_rom_rule(&mut self, rule: ParsedRule) {
        if rule.is_minus_sign {
            self.minus_signs.insert(rule.s.clone());
        }
        if rule.is_plus_sign {
            self.plus_signs.insert(rule.s.clone());
        }
        if rule.fraction_connector {
            self.fraction_connectors.insert(rule.s.clone());
        }
        if rule.percentage_marker {
            self.percentage_markers.insert(rule.s.clone());
        }

        self.register_s_prefix(&rule.s);

        let old_rules = self.rom_rules.entry(rule.s.clone()).or_default();

        // A single unrestricted rule from a hand-curated table replaces a
        // single auto-derived ("ud") or overwrite-table ("ow") rule.
        let is_unconditional = rule.is_unconditional();
        let should_overwrite = old_rules.len() == 1 && {
            let old_rule = &old_rules[0];
            (old_rule.prov == "ud" || old_rule.prov == "ow") && is_unconditional
        };

        if should_overwrite {
            *old_rules = vec![rule];
        } else {
            old_rules.push(rule);
        }
    }

    fn load_chinese_pinyin_file(&mut self, file: &str) {
        for line in file.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            if let Some((chinese, pinyin_with_accent)) = line.split_once(char::is_whitespace) {
                // `de_accent_pinyin`: NFD decomposition, strip combining marks.
                let rom: String = pinyin_with_accent
                    .nfd()
                    .filter(|c| {
                        !matches!(
                            c.general_category_group(),
                            unicode_properties::GeneralCategoryGroup::Mark
                        )
                    })
                    .collect::<String>()
                    .replace('ü', "u");

                let rule = ParsedRule::new_simple(chinese.to_string(), &rom, "rom pinyin");
                self.rom_rules
                    .entry(chinese.to_string())
                    .or_default()
                    .push(rule);
                self.register_s_prefix(chinese);
            }
        }
    }

    fn load_script_file(&mut self, file: &str) {
        for line in file.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            let Some(script_name) = slot_value_in_double_colon_del_list(line, "script-name") else {
                continue;
            };

            let lc_script_name = script_name.to_lowercase();
            if self.scripts.contains_key(&lc_script_name) {
                // Duplicate script names: first entry wins (matches Python).
                continue;
            }

            let abugida_default_vowel_s =
                slot_value_in_double_colon_del_list(line, "abugida-default-vowel").unwrap_or("");
            let abugida_default_vowels: Vec<String> = if abugida_default_vowel_s.is_empty() {
                vec![]
            } else {
                abugida_default_vowel_s
                    .split([',', ';'])
                    .map(|s| s.trim().to_string())
                    .collect()
            };
            let alt_script_name_s =
                slot_value_in_double_colon_del_list(line, "alt-script-name").unwrap_or("");
            let alt_script_names: Vec<String> = if alt_script_name_s.is_empty() {
                vec![]
            } else {
                alt_script_name_s
                    .split([',', ';'])
                    .map(|s| s.trim().to_string())
                    .collect()
            };

            let abugida_rule_type = if !abugida_default_vowels.is_empty() {
                let vowels_regex1 = abugida_default_vowels.join("|");
                let vowels_regex2 = abugida_default_vowels
                    .iter()
                    .map(|v| format!("{v}+"))
                    .collect::<Vec<_>>()
                    .join("|");

                Some(match (vowels_regex1.as_str(), vowels_regex2.as_str()) {
                    ("a|o", "a+|o+") => AbugidaRuleType::AO,
                    ("a", "a+") => AbugidaRuleType::A,
                    other => panic!("unsupported abugida default vowels: {other:?}"),
                })
            } else {
                None
            };

            let new_script = FullScript {
                abugida_default_vowels,
                abugida_rule_type,
            };

            self.scripts.insert(lc_script_name, new_script.clone());
            for alt_script_name in alt_script_names {
                self.scripts
                    .insert(alt_script_name.to_lowercase(), new_script.clone());
            }
        }
    }

    fn load_unicode_data_props(&mut self, file: &str) {
        for line in file.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            let Some(script_name) = slot_value_in_double_colon_del_list(line, "script-name") else {
                continue;
            };

            if let Some(chars_str) = slot_value_in_double_colon_del_list(line, "char") {
                for c in chars_str.chars() {
                    self.char_scripts.insert(c, script_name.to_string());
                }
            }
            if let Some(vowel_sign_str) = slot_value_in_double_colon_del_list(line, "vowel-sign") {
                self.vowel_signs.extend(vowel_sign_str.chars());
            }
            if let Some(medial_str) =
                slot_value_in_double_colon_del_list(line, "medial-consonant-sign")
            {
                self.medial_consonant_signs.extend(medial_str.chars());
            }
            if let Some(virama_str) = slot_value_in_double_colon_del_list(line, "sign-virama") {
                self.viramas.extend(virama_str.chars());
            }
        }
    }

    fn load_num_props(&mut self, file_content: &str) {
        for line in file_content.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            let json: JsonValue = serde_json::from_str(line).unwrap();
            let Some(obj) = json.as_object() else {
                continue;
            };
            let Some(txt) = obj.get("txt").and_then(|v| v.as_str()) else {
                continue;
            };

            // The conversions below replicate the original generic JSON->Value
            // mapping (numbers -> int/float, bools -> 0/1, arrays keep only
            // integer items) applied to each field `Edge::new_numeric` reads.
            let as_int = |v: &JsonValue| -> Option<i64> {
                v.as_i64().or_else(|| v.as_bool().map(i64::from))
            };
            let as_float = |v: &JsonValue| -> Option<f64> {
                if let Some(i) = as_int(v) {
                    Some(i as f64)
                } else {
                    v.as_f64()
                }
            };

            let fraction = obj.get("fraction").and_then(|v| match v {
                JsonValue::String(s) => s
                    .split_once('/')
                    .and_then(|(num, den)| Some((num.parse().ok()?, den.parse().ok()?))),
                JsonValue::Array(arr) => {
                    let ints: Vec<i64> = arr.iter().filter_map(|item| item.as_i64()).collect();
                    match ints.as_slice() {
                        [num, den] if *den != 0 => Some((*num, *den)),
                        _ => None,
                    }
                }
                _ => None,
            });

            let prop = NumProp {
                rom: obj.get("rom").and_then(|v| v.as_str()).map(str::to_string),
                value: obj.get("value").and_then(as_float),
                fraction,
                num_type: obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                base: obj.get("base").and_then(as_int),
                mult: obj.get("mult").and_then(as_float),
                script: obj
                    .get("script")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                is_large_power: obj
                    .get("is-large-power")
                    .is_some_and(|v| as_int(v) == Some(1)),
            };

            self.num_props.insert(txt.to_string(), prop);
        }
    }

    /// Adds automatic cancellation rules for the Thai script: THANTHAKHAT
    /// (`\u{0E4C}`) marks the preceding letter(s) as silent.
    fn add_thai_cancellation_rules(&mut self) {
        let thai_cancellation_mark = '\u{0E4C}';
        for cp in 0x0E01..0x0E4C {
            if let Some(c) = std::char::from_u32(cp) {
                let s = format!("{c}{thai_cancellation_mark}");

                let rules_for_s = self.rom_rules.entry(s.clone()).or_default();
                if rules_for_s.is_empty() {
                    rules_for_s.push(ParsedRule::new_simple(s.clone(), "", "auto cancel letter"));
                    self.register_s_prefix(&s);
                }
            }
        }

        let thai_consonants = (0x0E01..0x0E2F).filter_map(std::char::from_u32);
        let thai_vowel_modifiers = ['\u{0E31}', '\u{0E47}']
            .into_iter()
            .chain((0x0E33..=0x0E3B).filter_map(std::char::from_u32));

        for c1 in thai_consonants.clone() {
            for v in thai_vowel_modifiers.clone() {
                let s = format!("{c1}{v}{thai_cancellation_mark}");

                let rules_for_s = self.rom_rules.entry(s.clone()).or_default();
                if rules_for_s.is_empty() {
                    rules_for_s.push(ParsedRule::new_simple(
                        s.clone(),
                        "",
                        "auto cancel syllable",
                    ));
                    self.register_s_prefix(&s);
                }
            }
        }
    }

    /// Extracts the actionable compatibility decompositions from
    /// `UnicodeData.txt`. Canonical (untagged) and `<super>`/`<sub>`/
    /// `<noBreak>`/`<compat>` decompositions are excluded here because the
    /// romanizer never acts on them (see `Lattice::decomp_rom`).
    fn load_decompositions(&mut self, unicode_data: &str) {
        for line in unicode_data.lines() {
            let fields: Vec<&str> = line.split(';').collect();
            if fields.len() > 5 && !fields[5].is_empty() {
                let Some(codepoint) = u32::from_str_radix(fields[0], 16)
                    .ok()
                    .and_then(char::from_u32)
                else {
                    continue;
                };

                let decomp_field = fields[5];
                if !decomp_field.starts_with('<') {
                    continue;
                }
                let mut parts = decomp_field.splitn(2, ' ');
                let tag = parts.next().unwrap_or("");
                let decomp_codes = parts.next().unwrap_or("");

                if matches!(tag, "<super>" | "<sub>" | "<noBreak>" | "<compat>") {
                    continue;
                }

                let mut decomp_str = String::new();
                for code_hex in decomp_codes.split(' ') {
                    if !code_hex.is_empty()
                        && let Ok(code) = u32::from_str_radix(code_hex, 16)
                        && let Some(ch) = char::from_u32(code)
                    {
                        decomp_str.push(ch);
                    }
                }

                if !decomp_str.is_empty() {
                    self.decompositions
                        .insert(codepoint, (tag.to_string(), decomp_str));
                }
            }
        }
    }

    /// Builds the character classes the romanizer used to test by substring
    /// search over a character's Unicode name.
    ///
    /// The names come from the same source the runtime used:
    /// the `::name` overrides first, then `unicode_names2`.
    /// The classes are exactly the set of characters for which the old test was true.
    fn char_classes(&self) -> [CharRanges; 4] {
        let (mut letters, mut subjoined, mut nuktas, mut vocalics) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());

        for cp in 0..=0x10FFFFu32 {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            let name = self.chr_name(c);
            if name.is_empty() {
                continue;
            }
            if name.contains("LETTER") {
                letters.push(c);
                if name.contains("SUBJOINED LETTER") {
                    subjoined.push(c);
                }
            }
            if name.contains("NUKTA") {
                nuktas.push(c);
            }
            if name.contains("VOCALIC") {
                vocalics.push(c);
            }
        }

        [
            CharRanges::from_chars(letters),
            CharRanges::from_chars(subjoined),
            CharRanges::from_chars(nuktas),
            CharRanges::from_chars(vocalics),
        ]
    }

    fn finalize(self) -> Dictionaries {
        let rom_rules = self
            .rom_rules
            .iter()
            .map(|(s, rules)| {
                (
                    s.clone(),
                    rules.iter().map(ParsedRule::to_runtime).collect::<Vec<_>>(),
                )
            })
            .collect();

        let [letters, subjoined_letters, nuktas, vocalics] = self.char_classes();

        // Intern the script names: every character of a script would
        // otherwise carry its own copy of the name.
        let mut script_names: Vec<String> = Vec::new();
        let mut script_ids: HashMap<&str, u16> = HashMap::new();
        let mut char_script_pairs: Vec<(char, u16)> = Vec::with_capacity(self.char_scripts.len());
        for (c, name) in &self.char_scripts {
            let id = match script_ids.get(name.as_str()) {
                Some(&id) => id,
                None => {
                    let id = u16::try_from(script_names.len()).expect("too many script names");
                    script_names.push(name.clone());
                    script_ids.insert(name.as_str(), id);
                    id
                }
            };
            char_script_pairs.push((*c, id));
        }
        let char_scripts = CharRangeMap::from_pairs(char_script_pairs);

        // Resolve abugida behavior once per interned script, so the runtime
        // never has to lowercase a script name to look it up.
        let abugida_by_script = script_names
            .iter()
            .map(|name| {
                let script = self.scripts.get(&name.to_lowercase())?;
                Some(ScriptInfo {
                    rule_type: script.abugida_rule_type?,
                    default_vowel: script.abugida_default_vowels.first()?.clone(),
                })
            })
            .collect();

        let sorted = |set: &HashSet<String>| {
            let mut v: Vec<String> = set.iter().cloned().collect();
            v.sort();
            v
        };

        Dictionaries {
            rom_rules,
            s_prefixes: self.s_prefixes.clone(),
            script_names,
            char_scripts,
            abugida_by_script,
            vowel_signs: self.vowel_signs.clone(),
            medial_consonant_signs: self.medial_consonant_signs.clone(),
            viramas: self.viramas.clone(),
            letters,
            subjoined_letters,
            nuktas,
            vocalics,
            tone_marks: self.tone_marks.clone(),
            syllable_infos: self.syllable_infos.clone(),
            num_props: self.num_props.clone(),
            decompositions: self.decompositions.clone(),
            percentage_markers: sorted(&self.percentage_markers),
            fraction_connectors: sorted(&self.fraction_connectors),
            plus_signs: sorted(&self.plus_signs),
            minus_signs: sorted(&self.minus_signs),
        }
    }
}
