//! The dictionary data model: plain-data types shared between the build-time
//! dictionary compiler (`build.rs`) and the runtime.
//!
//! Everything in here is data only. Parsing lives in [`super::parse`], and
//! runtime lookup goes through the [`super::Dicts`] facade so that the storage
//! representation can change without touching romanization logic.

use std::collections::{HashMap, HashSet};

use rkyv::{Archive, Deserialize, Serialize};

/// A single romanization rule, reduced to the fields the runtime actually
/// consults. Load-time-only attributes (provenance, sign/marker flags, …) are
/// consumed by the builder and folded into [`Dictionaries`] instead.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct RomRule {
    pub t: Option<String>,
    pub lcodes: Vec<String>,
    pub use_only_at_start_of_word: bool,
    pub dont_use_at_start_of_word: bool,
    pub use_only_at_end_of_word: bool,
    pub dont_use_at_end_of_word: bool,
    pub use_only_for_whole_word: bool,
    pub n_restr: u32,
    pub t_alts: Vec<String>,
    pub t_at_end_of_syllable: Option<String>,
}

impl ArchivedRomRule {
    pub fn target(&self) -> Option<&str> {
        self.t.as_ref().map(|s| s.as_str())
    }

    pub fn alt_targets(&self) -> impl Iterator<Item = &str> {
        self.t_alts.iter().map(|s| s.as_str())
    }

    pub fn end_of_syllable_target(&self) -> Option<&str> {
        self.t_at_end_of_syllable.as_ref().map(|s| s.as_str())
    }

    pub fn restriction_count(&self) -> u32 {
        self.n_restr.to_native()
    }

    /// True if the rule is unrestricted by language, or restricted to a set
    /// containing `lcode`.
    pub fn matches_lcode(&self, lcode: Option<&str>) -> bool {
        self.lcodes.is_empty() || lcode.is_some_and(|lc| self.lcodes.iter().any(|c| c == lc))
    }
}

/// How an abugida script fills in unwritten inherent vowels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub enum AbugidaRuleType {
    /// Inherent vowel `a` (e.g. Devanagari).
    A,
    /// Inherent vowels `a`/`o` (e.g. Lontara).
    AO,
}

/// Abugida metadata for a script, keyed by lowercase script name (including
/// alternate names). Scripts without abugida behavior are not stored: the
/// romanizer only ever consults this map to decide inherent-vowel insertion.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub rule_type: AbugidaRuleType,
    pub default_vowel: String,
}

/// Numeric properties of a character (from `NumProps.jsonl`).
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct NumProp {
    pub rom: Option<String>,
    pub value: Option<f64>,
    /// Stored as `(numerator, denominator)`; turned into a `Ratio` at runtime.
    pub fraction: Option<(i64, i64)>,
    pub num_type: String,
    pub base: Option<i64>,
    pub mult: Option<f64>,
    pub script: Option<String>,
    pub is_large_power: bool,
}

impl From<&ArchivedAbugidaRuleType> for AbugidaRuleType {
    fn from(archived: &ArchivedAbugidaRuleType) -> Self {
        match archived {
            ArchivedAbugidaRuleType::A => AbugidaRuleType::A,
            ArchivedAbugidaRuleType::AO => AbugidaRuleType::AO,
        }
    }
}

impl ArchivedNumProp {
    pub fn rom(&self) -> Option<&str> {
        self.rom.as_ref().map(|s| s.as_str())
    }

    pub fn value(&self) -> Option<f64> {
        self.value.as_ref().map(|v| v.to_native())
    }

    pub fn fraction_parts(&self) -> Option<(i64, i64)> {
        self.fraction
            .as_ref()
            .map(|pair| (pair.0.to_native(), pair.1.to_native()))
    }

    pub fn num_type(&self) -> &str {
        &self.num_type
    }

    pub fn base(&self) -> Option<i64> {
        self.base.as_ref().map(|v| v.to_native())
    }

    pub fn mult(&self) -> Option<f64> {
        self.mult.as_ref().map(|v| v.to_native())
    }

    pub fn script(&self) -> Option<&str> {
        self.script.as_ref().map(|s| s.as_str())
    }
}

/// All dictionaries needed at runtime, fully typed by purpose.
///
/// This replaces the former string-tag-keyed `dict_bool` / `dict_str` maps
/// (`("is-virama", c) -> bool`, …) with one collection per meaning.
#[derive(Debug, Archive, Serialize, Deserialize)]
pub struct Dictionaries {
    /// Source string -> romanization rule candidates, in load order.
    pub rom_rules: HashMap<String, Vec<RomRule>>,
    /// Every prefix of every `rom_rules` key, for incremental span search.
    pub s_prefixes: HashSet<String>,
    /// Lowercase script name -> abugida behavior (abugida scripts only).
    pub abugida_scripts: HashMap<String, ScriptInfo>,
    /// Character -> script name (from `UnicodeDataProps*.txt`).
    pub char_scripts: HashMap<char, String>,
    pub vowel_signs: HashSet<char>,
    pub medial_consonant_signs: HashSet<char>,
    pub viramas: HashSet<char>,
    /// Character -> tone mark class (from `UnicodeDataOverwrite.txt`).
    pub tone_marks: HashMap<char, String>,
    /// Character -> syllable placement info (e.g. Thai pre-written vowels).
    pub syllable_infos: HashMap<char, String>,
    /// Character name overrides (checked before `unicode_names2`).
    pub char_names: HashMap<char, String>,
    /// Numeral text -> numeric properties.
    pub num_props: HashMap<String, NumProp>,
    /// Character -> (decomposition tag, decomposed string), restricted to the
    /// tagged compatibility decompositions the romanizer can act on.
    pub decompositions: HashMap<char, (String, String)>,
    /// Marker strings collected from rule attributes, sorted for determinism.
    pub percentage_markers: Vec<String>,
    pub fraction_connectors: Vec<String>,
    pub plus_signs: Vec<String>,
    pub minus_signs: Vec<String>,
}
