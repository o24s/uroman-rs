//! The romanization lattice: a graph of candidate edges over the
//! input's character positions (indices are char counts, not byte
//! offsets).

mod braille;
mod numbers;
mod paths;
mod romanize;
mod rules;
mod tibetan;
mod word_boundaries;

use crate::core::UromanInner;
use crate::edge::Edge;
use std::collections::{HashMap, HashSet};

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
    pub props: HashMap<(&'static str, usize), Option<bool>>,

    // self.simple_top_rom_cache: dict
    pub simple_top_rom_cache: HashMap<(usize, usize), Option<String>>,
    // self.contains_script: defaultdict(bool)
    pub contains_script: HashMap<String, bool>,
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
}
