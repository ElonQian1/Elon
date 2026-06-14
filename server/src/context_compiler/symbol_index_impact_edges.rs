use std::collections::HashSet;

use super::{
    model::{ImpactFact, ImpactKind, RustImpactAnalysis},
    symbol_index::{normalize_path, SymbolEdge, SymbolIndex},
};

impl SymbolIndex {
    pub(super) fn add_impact_edges(&mut self, impact: &RustImpactAnalysis) {
        let mut seen = HashSet::new();
        for fact in impact
            .trait_implementations
            .iter()
            .chain(impact.function_call_sites.iter())
            .chain(impact.enum_match_sites.iter())
            .chain(impact.field_accesses.iter())
            .chain(impact.public_api_references.iter())
            .chain(impact.test_links.iter())
            .chain(impact.async_boundaries.iter())
        {
            self.add_impact_fact_edge(&mut seen, fact);
        }
    }

    fn add_impact_fact_edge(&mut self, seen: &mut HashSet<String>, fact: &ImpactFact) {
        let target_name = impact_target_name(fact);
        let from_index = self.find_symbol_index_at(&fact.path, fact.line, None);
        let target_index = self.find_symbol_index_by_name(&target_name).or_else(|| {
            (fact.kind == ImpactKind::AsyncBoundary)
                .then_some(from_index)
                .flatten()
        });
        self.push_edge(
            seen,
            SymbolEdge {
                id: String::new(),
                source: "impact_analysis",
                kind: impact_edge_kind(fact.kind).to_string(),
                from_symbol_id: from_index.map(|index| self.records[index].id.clone()),
                from_path: normalize_path(&fact.path),
                line: fact.line,
                to_symbol_id: target_index.map(|index| self.records[index].id.clone()),
                to_symbol_name: target_index
                    .map(|index| self.records[index].name.clone())
                    .or_else(|| (!target_name.is_empty()).then_some(target_name)),
                to_path: target_index.map(|index| self.records[index].file_path.clone()),
                confidence: impact_edge_confidence(fact.kind),
                reason: format!("{}: {}; {}", fact.kind.as_str(), fact.evidence, fact.reason),
            },
        );
    }

    fn find_symbol_index_by_name(&self, name: &str) -> Option<usize> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .and_then(|matches| matches.first().copied())
    }
}

fn impact_target_name(fact: &ImpactFact) -> String {
    match fact.kind {
        ImpactKind::TraitImplementation => fact
            .subject
            .split(" -> ")
            .next()
            .unwrap_or(&fact.subject)
            .to_string(),
        ImpactKind::FieldRead | ImpactKind::FieldWrite => fact
            .subject
            .split('.')
            .next()
            .unwrap_or(&fact.subject)
            .to_string(),
        _ => fact.subject.clone(),
    }
}

fn impact_edge_kind(kind: ImpactKind) -> &'static str {
    match kind {
        ImpactKind::TraitImplementation => "implements",
        ImpactKind::FunctionCallSite => "calls",
        ImpactKind::EnumMatchSite | ImpactKind::FieldRead | ImpactKind::FieldWrite => "type_uses",
        ImpactKind::PublicApiReference => "references",
        ImpactKind::TestLink => "test_covers",
        ImpactKind::AsyncBoundary => "safety_boundary",
    }
}

fn impact_edge_confidence(kind: ImpactKind) -> f32 {
    match kind {
        ImpactKind::TraitImplementation => 0.7,
        ImpactKind::FunctionCallSite => 0.5,
        ImpactKind::EnumMatchSite | ImpactKind::FieldRead | ImpactKind::FieldWrite => 0.45,
        ImpactKind::PublicApiReference | ImpactKind::TestLink => 0.6,
        ImpactKind::AsyncBoundary => 0.75,
    }
}
