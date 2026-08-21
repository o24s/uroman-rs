//! Number recognition: combines per-character numeric edges into digit
//! sequences, base multiplications and additions, fractions, percentages and
//! signs, layered as rule-based passes (D1, G1-G4, G6, F1) that mirror the
//! original Python implementation.

use std::collections::HashSet;
use std::sync::LazyLock;

use num_rational::Ratio;
use regex::Regex;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

use super::Lattice;
use crate::edge::{Edge, EdgeData, NumData, NumDataUpdates};

static STARTS_WITH_DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d").unwrap());
static ENDS_WITH_DIGIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d$").unwrap());

fn is_power_of_10(mut n: i64) -> bool {
    if n <= 0 {
        return false;
    }
    while n % 10 == 0 {
        n /= 10;
    }
    n == 1
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

impl Lattice {
    fn initialize_num_edges(&mut self) -> Vec<Edge> {
        let mut num_edges = Vec::new();
        for start in 0..self.s_chars.len() {
            if let Some(edge) =
                Edge::new_numeric(start, start + 1, self.s_chars[start], &self.dicts)
            {
                self.add_edge(edge.clone());
                num_edges.push(edge);
            }
        }
        num_edges
    }

    /// Runs all numeric passes in order.
    pub(crate) fn add_numbers(&mut self) {
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

    /// D1: joins adjacent single digits (and one decimal point) into one
    /// number.
    fn apply_d1_digits(&mut self, prev_pass_edges: Vec<Edge>) -> Vec<Edge> {
        let mut next_pass_edges = Vec::new();
        let mut i = 0;
        while i < prev_pass_edges.len() {
            let start_edge = &prev_pass_edges[i];

            if !edge_is_digit(start_edge) {
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

                if edge_is_digit(next_edge) {
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
                        .filter(|e| e.start() == last_edge_end + 1 && edge_is_digit(e))
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

    /// G1: digit × base (e.g. 五百 = 5 × 100).
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

    /// G2: descending-base addition (e.g. 百二十三 = 100 + 20 + 3).
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

                let prev_non_null = sub_edges.iter().rev().find(|e| !is_gap_null_edge(e));
                let can_combine = if prev_non_null.is_some_and(|p| p.is_large_power()) {
                    false
                } else if let (Some(prev), Some(right_val), Some(right_base)) =
                    (prev_non_null, right_edge.value(), right_edge.get_num_base())
                {
                    prev.get_num_base().unwrap_or(0) > right_val as i64
                        && prev.get_num_base().unwrap_or(0) > right_base
                } else {
                    is_gap_null_edge(right_edge)
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

    /// G3: number × large power (e.g. 二百万 = 200 × 10000).
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

    /// G4: adds large blocks in descending order (e.g. 一兆 + 二千万 + …),
    /// re-tagging trailing single digits by the preceding base.
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

    /// Fractions (`十分之一` -> 1/10) and percentages (`百分之五十` -> 50%).
    fn apply_fraction_and_percentage_patterns(&mut self) {
        let mut new_edges: Vec<Edge> = Vec::new();
        let mut edges_to_deactivate: Vec<Edge> = Vec::new();

        // Combine all markers, longest first, so that e.g. "百分之" wins
        // over shorter ones.
        let mut markers: Vec<(Vec<char>, &str)> = self
            .dicts
            .percentage_markers()
            .map(|m| (m.chars().collect::<Vec<char>>(), "percentage"))
            .chain(
                self.dicts
                    .fraction_connectors()
                    .map(|c| (c.chars().collect::<Vec<char>>(), "fraction")),
            )
            .collect();
        markers.sort_by_key(|(b, _)| std::cmp::Reverse(b.len()));

        'outer: for start in 0..self.s_chars.len() {
            if let Some((marker_str, marker_type)) = markers
                .iter()
                .find(|(m, _)| self.s_chars[start..].starts_with(m.as_slice()))
            {
                let marker_end = start + marker_str.len();

                // --- (Number) + Marker/Connector + (Number) ---
                // Handles "10 / 1", "十分之一", and the special case "百分之一".
                // `false`: do NOT skip numeric edges.
                if let Some(left_edge) = self.best_left_neighbor_edge(start, false)
                    && let Some(right_edge) = self.best_right_neighbor_edge(marker_end, false)
                {
                    // Ensure the pattern is contiguous and connected to the marker.
                    if left_edge.end() != start || right_edge.start() != marker_end {
                        continue;
                    }

                    if let (Some(left_val), Some(right_val)) = (
                        left_edge.value().and_then(|v| {
                            if v.fract() == 0.0 {
                                Some(v as i64)
                            } else {
                                None
                            }
                        }),
                        right_edge.value().and_then(|v| {
                            if v.fract() == 0.0 {
                                Some(v as i64)
                            } else {
                                None
                            }
                        }),
                    ) {
                        let combined_start = left_edge.start();
                        let combined_end = right_edge.end();
                        let mut consumed = false;

                        // A left value of 100 with a fraction connector is a
                        // percentage ("百分之一"); explicit percentage markers
                        // ("100 % 1") too.
                        if left_val == 100
                            && (*marker_type == "fraction" || *marker_type == "percentage")
                        {
                            new_edges.push(Edge::new_regular(
                                combined_start,
                                combined_end,
                                format!("{right_val}%"),
                                "percentage".to_string(),
                            ));
                            consumed = true;
                        } else if *marker_type == "fraction" && left_val != 0 {
                            // Standard fraction like "十分之一".
                            let fraction = Ratio::new(right_val, left_val);
                            new_edges.push(Edge::Numeric {
                                data: EdgeData {
                                    start: combined_start,
                                    end: combined_end,
                                    txt: format!("{right_val}/{left_val}"),
                                    r#type: "fraction".to_string(),
                                },
                                num_data: NumData {
                                    orig_txt: format!("{right_val}/{left_val}"),
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
                            continue 'outer;
                        }
                    }
                }

                // --- Marker + (Number) ---
                // Handles "百分之" + "一": the primary `percentage-marker` path.
                if *marker_type == "percentage"
                    && let Some(right_edge) = self.best_right_neighbor_edge(marker_end, false)
                {
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

        for edge in edges_to_deactivate {
            self.deactivate_edge(&edge);
        }
        for edge in new_edges {
            self.add_edge(edge);
        }
    }

    /// G6: attaches directly preceding plus/minus signs to numbers.
    fn apply_g6_plus_minus_signs(&mut self, active_edges: &[Edge]) {
        let signed: [(Vec<&str>, char); 2] = [
            (self.dicts.minus_signs().collect(), '-'),
            (self.dicts.plus_signs().collect(), '+'),
        ];

        for edge in active_edges.iter().filter(|e| e.value().is_some()) {
            let edge_start_char_idx = edge.start();

            for (signs, sign_char) in &signed {
                for sign in signs {
                    let sign_as_chars: Vec<char> = sign.chars().collect();
                    if let Some(start_pos_char_idx) =
                        edge_start_char_idx.checked_sub(sign_as_chars.len())
                        && self.s_chars.get(start_pos_char_idx..edge_start_char_idx)
                            == Some(sign_as_chars.as_slice())
                    {
                        let new_edge = Edge::new_regular(
                            start_pos_char_idx,
                            edge.end(),
                            format!("{sign_char}{}", edge.txt()),
                            format!("{} {sign_char}", edge.r#type()),
                        );
                        self.add_edge(new_edge);
                    }
                }
            }
        }
    }

    /// F1: separates a number directly following another number's digits.
    fn apply_f1_final_adjustments(&mut self) {
        let mut edges_to_add: Vec<Edge> = Vec::new();
        let mut edges_to_deactivate: Vec<Edge> = Vec::new();

        for left_edges in self.edge_lattice.values() {
            for left_edge in left_edges.iter() {
                if !ENDS_WITH_DIGIT_RE.is_match(left_edge.txt()) {
                    continue;
                }

                if let Some(right_link_ends) = self.right_links.get(&left_edge.end()) {
                    for right_end in right_link_ends {
                        if let Some(right_edges) =
                            self.edge_lattice.get(&(left_edge.end(), *right_end))
                        {
                            for right_edge in right_edges {
                                if right_edge.is_numeric()
                                    && STARTS_WITH_DIGIT_RE.is_match(right_edge.txt())
                                {
                                    let mut new_edge = right_edge.clone();
                                    let has_fraction = new_edge
                                        .get_num_data()
                                        .is_some_and(|d| d.fraction.is_some());
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

        for edge in edges_to_deactivate {
            self.deactivate_edge(&edge);
        }

        for new_edge in edges_to_add {
            self.add_edge(new_edge);
        }
    }

    /// Marks an edge inactive in place within the lattice.
    fn deactivate_edge(&mut self, edge: &Edge) {
        if let Some(edges) = self.edge_lattice.get_mut(&(edge.start(), edge.end()))
            && let Some(mut e) = edges.take(edge)
        {
            e.set_active(false);
            edges.insert(e);
        }
    }

    /// Deactivates numerals that should not stand alone (e.g. financial
    /// forms like 兩/參, or single characters with values > 1000).
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

    /// Adds plain edges for decimal digits not covered by any numeric edge.
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
}
