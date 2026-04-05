//! Defines the `RomRule` struct and related parsing logic.

use std::collections::HashMap;

use crate::{
    core::UromanInner,
    utils::{
        dequote_string, has_value_in_double_colon_del_list, slot_value_in_double_colon_del_list,
    },
};

#[allow(unused)]
/// Represents a single romanization rule parsed from the data files.
#[derive(Debug, Clone)]
pub(super) struct RomRule {
    pub s: String,
    pub t: Option<String>,
    pub prov: String,
    pub lcodes: Vec<String>,
    pub use_only_at_start_of_word: bool,
    pub dont_use_at_start_of_word: bool,
    pub use_only_at_end_of_word: bool,
    pub dont_use_at_end_of_word: bool,
    pub use_only_for_whole_word: bool,
    pub n_restr: usize,
    pub t_alts: Vec<String>,
    pub num: Option<crate::core::Value>,
    pub is_minus_sign: bool,
    pub is_plus_sign: bool,
    pub is_decimal_point: bool,
    pub fraction_connector: bool,
    pub percentage_marker: bool,
    pub int_frac_connector: bool,
    pub is_large_power: bool,
    pub t_at_end_of_syllable: Option<String>,
}

impl RomRule {
    /// Creates a simple `RomRule` with default values for most fields.
    pub fn new_simple(s: String, t: &str, provenance: &str) -> Self {
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
            num: None,
            is_minus_sign: false,
            is_plus_sign: false,
            is_decimal_point: false,
            fraction_connector: false,
            percentage_marker: false,
            int_frac_connector: false,
            is_large_power: false,
            t_at_end_of_syllable: None,
        }
    }

    /// Checks if the rule has no contextual restrictions (lcodes, word position).
    /// This is crucial for the rule overwriting logic in `load_rom_file`.
    pub fn is_unconditional(&self) -> bool {
        self.lcodes.is_empty()
            && !self.use_only_at_start_of_word
            && !self.dont_use_at_start_of_word
            && !self.use_only_at_end_of_word
            && !self.dont_use_at_end_of_word
            && !self.use_only_for_whole_word
    }

    pub fn from_line(
        line: &str,
        provenance: &str,
        file_format: &str,
        uroman: &mut UromanInner,
    ) -> Option<Self> {
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

        // println!("DEBUG: RomRule::from_line - s: '{}', t: {:?}, prov: {}", s, t, provenance);

        let t = uroman.second_rom_filter(&s, t.as_deref());

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

        let num = slot_value_in_double_colon_del_list(line, "num")
            .and_then(crate::utils::robust_str_to_num);

        let is_minus_sign = has_value_in_double_colon_del_list(line, "is-minus-sign");
        let is_plus_sign = has_value_in_double_colon_del_list(line, "is-plus-sign");
        let is_decimal_point = has_value_in_double_colon_del_list(line, "is-decimal-point");
        let fraction_connector = has_value_in_double_colon_del_list(line, "fraction-connector");
        let percentage_marker = has_value_in_double_colon_del_list(line, "percentage-marker");
        let int_frac_connector = has_value_in_double_colon_del_list(line, "int-frac-connector");
        let is_large_power = has_value_in_double_colon_del_list(line, "is-large-power");
        let t_at_end_of_syllable = slot_value_in_double_colon_del_list(line, "t-end-of-syllable")
            .map(|s_val| dequote_string(s_val).to_string());

        // Calculate the number of restrictions in a more declarative way
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
        .count();

        Some(RomRule {
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
            num,
            is_minus_sign,
            is_plus_sign,
            is_decimal_point,
            fraction_connector,
            percentage_marker,
            int_frac_connector,
            is_large_power,
            t_at_end_of_syllable,
        })
    }
}

/// A collection of romanization rules, typically grouped by the source string `s`.
pub type RomRules = HashMap<String, Vec<RomRule>>;
