use super::Lattice;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

static GOOD_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^'?(?:.|bd|br|brg|brgy|bs|bsh|bst|bt|bts|by|bz|bzh|ch|db|dby|dk|dm|dp|dpy|dr|gl|gn|gr|gs|gt|gy|gzh|kh|khr|khy|kr|ky|ld|lh|lt|mkh|mny|mth|mtsh|ny|ph|phr|phy|rgy|rk|el|rn|rny|rt|rts|sk|skr|sky|sl|sm|sn|sny|sp|spy|sr|st|th|ts|tsh)$").unwrap()
});
static GOOD_SUFFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:|[bcdfghjklmnpqrstvwxz]|bh|bs|ch|cs|dd|ddh|dh|dz|dzh|gh|gr|gs|kh|khs|kss|n|nn|nt|ms|ng|ngs|ns|ph|rm|sh|ss|th|ts|tsh|tt|tth|zh|zhs)'?$").unwrap()
});
static ROMS_CONSONANT_A_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([bcdfghjklmnpqrstvwxyz].*)a$").unwrap());
static ROM_VOWEL_END_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[aeiou]+$").unwrap());

impl<'a> Lattice<'a> {
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
                self.props.insert(("edge-vowel", i), None);

                if self.char_is_vowel_sign(c) {
                    vowel_pos = Some(i);
                    self.props.insert(("edge-vowel", i), Some(true));
                    if roms.len() == 1 && roms[0] == "'" {
                        self.props
                            .insert(("edge-delete", i - 1), Some(true));
                    }
                } else if self.char_is_subjoined_letter(c) {
                    subjoined_letter_positions.insert(i);
                    if i > first_letter_position {
                        // The special index `\u0FB0` (SUBJOINED -A) converts the previous character into a vowel
                        if c == '\u{0FB0}' {
                            vowel_pos = Some(i - 1);
                            self.props
                                .insert(("edge-vowel", i - 1), Some(true));
                        } else {
                            // Other indices suppress the vowel of the previous character
                            self.props
                                .insert(("edge-vowel", i - 1), Some(false));
                        }
                    }
                    rom = ROMS_CONSONANT_A_END_RE.replace(&rom, "$1").to_string();
                } else if c == '\u{0F60}' {
                    // TIBETAN LETTER -A
                    self.props
                        .insert(("edge-vowel", i), Some(false));
                    if i > first_letter_position {
                        vowel_pos = Some(i - 1);
                        self.props
                            .insert(("edge-vowel", i - 1), Some(true));
                        if i == *positions.last().unwrap() {
                            self.props
                                .insert(("edge-delete", i), Some(true));
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
                        .get(&("edge-vowel", i))
                        .and_then(|&v| v)
                        .is_none()
                    {
                        self.props
                            .insert(("edge-vowel", i), Some(false));
                    }
                }
            } else {
                let mut best_cost = f64::INFINITY;
                let mut best_vowel_pos: Option<usize> = None;

                for (idx, &i) in positions.iter().enumerate() {
                    let pre: String = roms[..=idx].join("");
                    let post: String = roms[idx + 1..].join("");

                    let cost =
                        if self.props.get(&("edge-vowel", i)) == Some(&Some(false)) {
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
                            .insert(("edge-vowel", i), Some(i == pos));
                    }
                }
            }
        }
    }
}
