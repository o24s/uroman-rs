//! Path extraction: choosing the best edge sequence through the lattice, and
//! expanding it with alternative romanizations.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::Lattice;
use crate::edge::Edge;

static ROM_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bc:([a-zA-Z]+)\s+s:([a-zA-Z]+)\b").unwrap());

impl Lattice {
    /// All edges fully contained in `[start, end)`, grouped by start position
    /// with longer spans first.
    pub(crate) fn all_edges(&self, start: usize, end: usize) -> Vec<Edge> {
        let mut result = Vec::new();

        for start2 in start..end {
            if let Some(end_positions) = self.right_links.get(&start2) {
                let mut sorted_ends: Vec<usize> = end_positions.iter().cloned().collect();
                sorted_ends.sort_unstable_by(|a, b| b.cmp(a));

                for &end2 in &sorted_ends {
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

    pub(super) fn best_edge_in_span(
        &self,
        start: usize,
        end: usize,
        skip_num_edge: bool,
    ) -> Option<Edge> {
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

    pub(super) fn best_right_neighbor_edge(
        &self,
        start: usize,
        skip_num_edge: bool,
    ) -> Option<Edge> {
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

    pub(super) fn best_left_neighbor_edge(&self, end: usize, skip_num_edge: bool) -> Option<Edge> {
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

    /// Walks the lattice right to left from `end`, concatenating the best
    /// edges' romanizations until `min_char_len` characters are collected (or
    /// `start` is reached).
    pub(super) fn find_rom_edge_path_backwards_str(
        &self,
        start: usize,
        end: usize,
        min_char_len: Option<usize>,
        skip_num_edge: bool,
    ) -> String {
        let mut current_pos = end;
        let mut rom_s = String::new();
        while current_pos > start {
            let old_pos = current_pos;
            if let Some(new_edge) = self.best_left_neighbor_edge(current_pos, skip_num_edge) {
                rom_s.insert_str(0, new_edge.txt());
                current_pos = new_edge.start();
            }

            if let Some(min_len) = min_char_len
                && rom_s.chars().count() >= min_len
            {
                break;
            }
            if current_pos == old_pos {
                current_pos -= 1;
            }
        }
        rom_s
    }

    /// Greedily walks the lattice left to right, picking the best edge at
    /// each position.
    pub(crate) fn best_rom_edge_path(
        &mut self,
        start: usize,
        end: usize,
        skip_num_edge: bool,
    ) -> Vec<Edge> {
        let mut result = Vec::new();
        let mut current_pos = start;
        while current_pos < end {
            if let Some(best_edge) = self.best_right_neighbor_edge(current_pos, skip_num_edge) {
                current_pos = best_edge.end();
                result.push(best_edge);
            } else {
                current_pos += 1;
            }
        }
        result
    }

    /// Adds alternative romanizations (`t-alt`, end-of-syllable variants) as
    /// extra edges next to the ones already selected.
    pub(crate) fn add_alternatives(&mut self, edges: &mut Vec<Edge>) {
        let mut existing_edges: HashSet<(usize, usize, String)> = edges
            .iter()
            .map(|e| (e.start(), e.end(), e.txt().to_string()))
            .collect();

        let mut new_edges_to_add = Vec::new();

        for base_edge in edges.clone() {
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

            for rom_rule in self.dicts.rom_rules(&orig_s) {
                if !self.cand_is_valid(rom_rule, start, end) {
                    continue;
                }

                let Some(rom_t) = rom_rule.target() else {
                    continue;
                };

                if rom_t == old_rom || old_rom_core == Some(rom_t) {
                    for rom_alt_base in rom_rule.alt_targets() {
                        let mut rom_alt = rom_alt_base.to_string();
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

                if let Some(rom_end_of_syllable) = rom_rule.end_of_syllable_target() {
                    if rom_t == old_rom && existing_edges.insert((start, end, rom_t.to_string())) {
                        new_edges_to_add.push(Edge::new_regular(
                            start,
                            end,
                            rom_t.to_string(),
                            "rom-alt2".to_string(),
                        ));
                    }

                    if rom_end_of_syllable == old_rom
                        && existing_edges.insert((start, end, rom_t.to_string()))
                    {
                        new_edges_to_add.push(Edge::new_regular(
                            start,
                            end,
                            rom_t.to_string(),
                            "rom-alt3".to_string(),
                        ));
                    }
                }
            }
        }

        edges.extend(new_edges_to_add);
    }
}
