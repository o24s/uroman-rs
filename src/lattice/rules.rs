//! Rule selection: which romanization rule applies to a span, given its
//! contextual restrictions.

use super::Lattice;
use crate::dicts::Rule;

impl Lattice {
    pub(super) fn cand_is_valid(&mut self, rule: &Rule, start: usize, end: usize) -> bool {
        let lcode_ok = rule.matches_lcode(self.lcode.as_deref());
        let start_ok = (!rule.use_only_at_start_of_word || self.is_at_start_of_word(start))
            && (!rule.dont_use_at_start_of_word || !self.is_at_start_of_word(start));
        let end_ok = (!rule.use_only_at_end_of_word || self.is_at_end_of_word(end))
            && (!rule.dont_use_at_end_of_word || !self.is_at_end_of_word(end));
        let whole_word_ok = !rule.use_only_for_whole_word
            || (self.is_at_start_of_word(start) && self.is_at_end_of_word(end));
        lcode_ok && start_ok && end_ok && whole_word_ok
    }

    /// The best romanization for a span, if any rule applies.
    ///
    /// With `simple_search` the end-of-syllable alternative and the result
    /// cache are skipped (used from contexts that probe neighboring spans).
    pub(super) fn simple_top_romanization_candidate_for_span(
        &mut self,
        start: usize,
        end: usize,
        simple_search: bool,
    ) -> Option<String> {
        if end > self.max_vertex {
            return None;
        }
        let span_range = (start, end);
        if !simple_search && let Some(cached_result) = self.simple_top_rom_cache.get(&span_range) {
            return cached_result.clone();
        }

        let sub: String = self.s_chars[start..end].iter().collect();
        let rules = self.dicts.rom_rules(&sub);
        if rules.is_empty() {
            if !simple_search {
                self.simple_top_rom_cache.insert(span_range, None);
            }
            return None;
        }

        let mut best_rule_with_t: Option<&Rule> = None;
        let mut best_rule_without_t: Option<&Rule> = None;

        for rule in rules {
            if self.cand_is_valid(rule, start, end) {
                if rule.target().is_some() {
                    if best_rule_with_t
                        .is_none_or(|best| rule.restriction_count() > best.restriction_count())
                    {
                        best_rule_with_t = Some(rule);
                    }
                } else if best_rule_without_t
                    .is_none_or(|best| rule.restriction_count() > best.restriction_count())
                {
                    best_rule_without_t = Some(rule);
                }
            }
        }

        let best_rule = best_rule_with_t.or(best_rule_without_t);
        let best_cand = best_rule.and_then(|r| r.target().map(str::to_string));

        if simple_search {
            return best_cand;
        }

        let Some(rule) = best_rule else {
            self.simple_top_rom_cache.insert(span_range, None);
            return None;
        };

        let mut final_cand = best_cand;
        if let Some(t_at_end) = rule.end_of_syllable_target() {
            let (is_end, _rationale) = self.is_at_end_of_syllable(end);
            if is_end {
                final_cand = Some(t_at_end.to_string());
            }
        }

        self.simple_top_rom_cache
            .insert(span_range, final_cand.clone());
        final_cand
    }

    /// The target of the first rule for `s`, ignoring restrictions.
    pub(super) fn romanization_by_first_rule(&self, s: &str) -> Option<&'static str> {
        self.dicts
            .rom_rules(s)
            .first()
            .and_then(|rule| rule.target())
    }
}
