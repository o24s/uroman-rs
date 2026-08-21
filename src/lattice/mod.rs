//! The romanization lattice: a graph of candidate edges over the input's
//! character positions (indices are char counts, not byte offsets).
//!
//! The pipeline (driven from `lib.rs`) fills the lattice in passes:
//! script-specific preparation ([`tibetan`], [`braille`]), rule-based
//! romanization ([`romanize`]), number recognition ([`numbers`]), fallbacks,
//! and finally best-path extraction ([`paths`]).

mod braille;
mod numbers;
mod paths;
mod romanize;
mod rules;
mod tibetan;
mod word_boundaries;

use std::collections::{HashMap, HashSet};

use crate::dicts::Dicts;
use crate::edge::Edge;

pub(super) fn is_braille_char(c: char) -> bool {
    ('\u{2800}'..='\u{28FF}').contains(&c)
}

/// Per-position flags and caches collected while filling the lattice.
#[derive(Default)]
struct CharProps {
    /// Cache for [`Lattice::is_at_start_of_word`].
    preceded_by_alpha: HashMap<usize, bool>,
    /// Cache for [`Lattice::is_at_end_of_word`].
    followed_by_alpha: HashMap<usize, bool>,
    /// Positions capitalized by a Braille all-caps marker.
    is_upper: HashSet<usize>,
    /// Tibetan inherent-vowel decision per position:
    /// absent = not in a Tibetan syllable, `None` = undecided, `Some(b)` = decided.
    edge_vowel: HashMap<usize, Option<bool>>,
    /// Tibetan positions whose romanization is deleted entirely.
    edge_delete: HashSet<usize>,
}

pub(crate) struct Lattice {
    s: String,
    s_chars: Vec<char>,
    lcode: Option<String>,
    dicts: Dicts,

    /// Edges by `(start, end)` span.
    edge_lattice: HashMap<(usize, usize), HashSet<Edge>>,
    /// For each start position, the set of end positions with edges.
    right_links: HashMap<usize, HashSet<usize>>,
    /// For each end position, the set of start positions with edges.
    left_links: HashMap<usize, HashSet<usize>>,

    max_vertex: usize,

    props: CharProps,
    simple_top_rom_cache: HashMap<(usize, usize), Option<String>>,
    contains_scripts: HashSet<String>,
}

impl Lattice {
    pub fn new(s: &str, dicts: Dicts, lcode: Option<&str>) -> Self {
        let s_chars: Vec<char> = s.chars().collect();
        let max_vertex = s_chars.len();

        let mut lattice = Self {
            s_chars,
            s: s.to_string(),
            lcode: lcode.map(String::from),
            dicts,
            edge_lattice: HashMap::new(),
            right_links: HashMap::new(),
            left_links: HashMap::new(),
            max_vertex,
            props: CharProps::default(),
            simple_top_rom_cache: HashMap::new(),
            contains_scripts: HashSet::new(),
        };

        lattice.check_for_scripts();

        lattice
    }

    fn check_for_scripts(&mut self) {
        for &c in &self.s_chars {
            let script_name = self.dicts.chr_script_name(c);
            if !script_name.is_empty() {
                self.contains_scripts.insert(script_name.to_string());
            }
        }
        if self.s.contains(is_braille_char) {
            self.contains_scripts.insert("Braille".to_string());
        }
    }

    fn has_script(&self, script: &str) -> bool {
        self.contains_scripts.contains(script)
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

    fn char_is_letter_or_vowel_sign(&self, c: char) -> bool {
        self.dicts.is_vowel_sign(c) || self.dicts.chr_name(c).contains("LETTER")
    }

    fn char_is_vowel_sign(&self, c: char) -> bool {
        self.dicts.is_vowel_sign(c)
    }

    fn char_is_subjoined_letter(&self, c: char) -> bool {
        self.dicts.chr_name(c).contains("SUBJOINED LETTER")
    }
}
