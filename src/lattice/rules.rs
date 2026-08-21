use super::Lattice;
use crate::rom_rule::RomRule;

impl<'a> Lattice<'a> {
    pub(super) fn cand_is_valid(&mut self, rule: &RomRule, start: usize, end: usize) -> bool {
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

    pub fn simple_top_romanization_candidate_for_span(
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
        let Some(rules) = self.uroman.rom_rules.get(&sub) else {
            if !simple_search {
                self.simple_top_rom_cache.insert(span_range, None);
            }
            return None;
        };

        let mut best_rule_with_t: Option<&RomRule> = None;
        let mut best_rule_without_t: Option<&RomRule> = None;

        for rule in rules.iter() {
            if self.cand_is_valid(rule, start, end) {
                if rule.t.is_some() {
                    if best_rule_with_t.is_none()
                        || rule.n_restr > best_rule_with_t.unwrap().n_restr
                    {
                        best_rule_with_t = Some(rule);
                    }
                } else if best_rule_without_t.is_none()
                    || rule.n_restr > best_rule_without_t.unwrap().n_restr
                {
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

    pub(super) fn romanization_by_first_rule(&self, s: &str) -> Option<String> {
        self.uroman
            .rom_rules
            .get(s)
            .and_then(|rules| rules.first())
            .and_then(|rule| rule.t.clone())
    }
}
