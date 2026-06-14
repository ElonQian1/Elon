use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolIndex {
    pub(crate) records: Vec<SymbolRecord>,
    pub(crate) edges: Vec<SymbolEdge>,
    pub(super) by_id: BTreeMap<String, usize>,
    pub(super) by_name: BTreeMap<String, Vec<usize>>,
    pub(super) by_path: BTreeMap<String, Vec<usize>>,
    pub(super) incoming_edges: BTreeMap<String, Vec<usize>>,
    pub(super) outgoing_edges: BTreeMap<String, Vec<usize>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SymbolRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) qualified_name: String,
    pub(crate) kind: String,
    pub(crate) language: &'static str,
    pub(crate) file_path: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) signature: String,
    pub(crate) visibility: String,
    pub(crate) parent_symbol_id: Option<String>,
    pub(crate) module_path: String,
    pub(crate) doc_summary: Option<String>,
    pub(crate) role: &'static str,
    pub(crate) importance_score: Option<f64>,
    pub(crate) signature_hash: String,
    pub(crate) source_providers: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SymbolEdge {
    pub(crate) id: String,
    pub(crate) source: &'static str,
    pub(crate) kind: String,
    pub(crate) from_symbol_id: Option<String>,
    pub(crate) from_path: String,
    pub(crate) line: usize,
    pub(crate) to_symbol_id: Option<String>,
    pub(crate) to_symbol_name: Option<String>,
    pub(crate) to_path: Option<String>,
    pub(crate) confidence: f32,
    pub(crate) reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolQuery {
    pub(crate) text: String,
    pub(crate) limit: usize,
    pub(crate) kind: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SymbolLookupSummary {
    pub(crate) symbol_count: usize,
    pub(crate) edge_count: usize,
    pub(crate) file_count: usize,
    pub(crate) kind_counts: BTreeMap<String, usize>,
    pub(crate) source_counts: BTreeMap<String, usize>,
    pub(crate) lsp_edge_count: usize,
    pub(crate) query_api: Vec<&'static str>,
}

#[allow(dead_code)]
impl SymbolIndex {
    pub(crate) fn get_symbol(&self, id: &str) -> Option<&SymbolRecord> {
        self.by_id
            .get(id)
            .and_then(|index| self.records.get(*index))
    }

    pub(crate) fn search_symbols(&self, query: SymbolQuery) -> Vec<&SymbolRecord> {
        let terms = tokenize(&query.text);
        if terms.is_empty() {
            return self.records.iter().take(query.limit.max(1)).collect();
        }
        let limit = query.limit.max(1);
        let mut scored = self
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                query
                    .kind
                    .as_deref()
                    .map(|kind| record.kind == kind)
                    .unwrap_or(true)
            })
            .filter_map(|(index, record)| {
                let score = symbol_match_score(record, &terms);
                (score > 0).then_some((index, score))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| {
                    self.records[left.0]
                        .file_path
                        .cmp(&self.records[right.0].file_path)
                })
                .then_with(|| {
                    self.records[left.0]
                        .start_line
                        .cmp(&self.records[right.0].start_line)
                })
        });
        scored
            .into_iter()
            .take(limit)
            .filter_map(|(index, _)| self.records.get(index))
            .collect()
    }

    pub(crate) fn symbols_in_file(&self, path: &str) -> Vec<&SymbolRecord> {
        self.by_path
            .get(&normalize_path(path))
            .into_iter()
            .flatten()
            .filter_map(|index| self.records.get(*index))
            .collect()
    }

    pub(crate) fn references_to(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.incoming_edges
            .get(symbol_id)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .filter(|edge| edge.kind == "reference" || edge.kind == "def_ref")
            .collect()
    }

    pub(crate) fn neighbors(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        let mut edge_indexes = BTreeSet::new();
        if let Some(incoming) = self.incoming_edges.get(symbol_id) {
            edge_indexes.extend(incoming.iter().copied());
        }
        if let Some(outgoing) = self.outgoing_edges.get(symbol_id) {
            edge_indexes.extend(outgoing.iter().copied());
        }
        edge_indexes
            .into_iter()
            .filter_map(|index| self.edges.get(index))
            .collect()
    }

    pub(crate) fn tests_for_symbol(&self, symbol_id: &str) -> Vec<&SymbolEdge> {
        self.incoming_edges
            .get(symbol_id)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .filter(|edge| edge.kind == "test_covers")
            .collect()
    }

    pub(crate) fn lookup_summary(&self) -> SymbolLookupSummary {
        let mut kind_counts = BTreeMap::new();
        let mut source_counts = BTreeMap::new();
        let mut files = BTreeSet::new();
        for record in &self.records {
            *kind_counts.entry(record.kind.clone()).or_insert(0) += 1;
            files.insert(record.file_path.clone());
            for source in &record.source_providers {
                *source_counts.entry(source.clone()).or_insert(0) += 1;
            }
        }
        SymbolLookupSummary {
            symbol_count: self.records.len(),
            edge_count: self.edges.len(),
            file_count: files.len(),
            kind_counts,
            source_counts,
            lsp_edge_count: self
                .edges
                .iter()
                .filter(|edge| edge.source == "rust_analyzer_lsp")
                .count(),
            query_api: vec![
                "search_symbols",
                "get_symbol",
                "symbols_in_file",
                "references_to",
                "neighbors",
                "tests_for_symbol",
            ],
        }
    }

    pub(super) fn rebuild_lookups(&mut self) {
        self.by_id.clear();
        self.by_name.clear();
        self.by_path.clear();
        for (index, record) in self.records.iter().enumerate() {
            self.by_id.insert(record.id.clone(), index);
            self.by_name
                .entry(record.name.to_ascii_lowercase())
                .or_default()
                .push(index);
            self.by_path
                .entry(normalize_path(&record.file_path))
                .or_default()
                .push(index);
        }
    }

    pub(super) fn rebuild_edge_lookups(&mut self) {
        self.incoming_edges.clear();
        self.outgoing_edges.clear();
        for (index, edge) in self.edges.iter().enumerate() {
            if let Some(to_symbol_id) = edge.to_symbol_id.as_deref() {
                self.incoming_edges
                    .entry(to_symbol_id.to_string())
                    .or_default()
                    .push(index);
            }
            if let Some(from_symbol_id) = edge.from_symbol_id.as_deref() {
                self.outgoing_edges
                    .entry(from_symbol_id.to_string())
                    .or_default()
                    .push(index);
            }
        }
    }

    pub(super) fn push_edge(&mut self, seen: &mut HashSet<String>, mut edge: SymbolEdge) {
        let id = edge_id(&edge);
        if !seen.insert(id.clone()) {
            return;
        }
        edge.id = id;
        self.edges.push(edge);
    }

    pub(super) fn find_symbol_index_at(
        &self,
        path: &str,
        line: usize,
        name: Option<&str>,
    ) -> Option<usize> {
        let path = normalize_path(path);
        let candidates = self.by_path.get(&path)?;
        if let Some(name) = name {
            if let Some(index) = candidates.iter().copied().find(|index| {
                let record = &self.records[*index];
                record.name == name && record.start_line == line
            }) {
                return Some(index);
            }
        }
        if let Some(index) = candidates
            .iter()
            .copied()
            .find(|index| self.records[*index].start_line == line)
        {
            return Some(index);
        }
        candidates
            .iter()
            .copied()
            .filter(|index| {
                let record = &self.records[*index];
                record.start_line <= line && line <= record.end_line
            })
            .min_by_key(|index| {
                let record = &self.records[*index];
                record.end_line.saturating_sub(record.start_line)
            })
    }
}

pub(super) fn push_source(record: &mut SymbolRecord, source: &str) {
    if !record
        .source_providers
        .iter()
        .any(|existing| existing == source)
    {
        record.source_providers.push(source.to_string());
    }
}

pub(super) fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

pub(super) fn stable_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn symbol_match_score(record: &SymbolRecord, terms: &[String]) -> i32 {
    let mut score = 0;
    let name = record.name.to_ascii_lowercase();
    let qualified = record.qualified_name.to_ascii_lowercase();
    let path = record.file_path.to_ascii_lowercase();
    let signature = record.signature.to_ascii_lowercase();
    let docs = record
        .doc_summary
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    for term in terms {
        if name == *term {
            score += 120;
        } else if name.contains(term) {
            score += 80;
        }
        if qualified.contains(term) {
            score += 45;
        }
        if path.contains(term) {
            score += 25;
        }
        if signature.contains(term) {
            score += 15;
        }
        if docs.contains(term) {
            score += 10;
        }
    }
    if record.visibility == "pub" {
        score += 8;
    }
    if record.importance_score.is_some() {
        score += 10;
    }
    score
}

fn tokenize(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

fn edge_id(edge: &SymbolEdge) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}",
        edge.source,
        edge.kind,
        edge.from_symbol_id.as_deref().unwrap_or(""),
        edge.from_path,
        edge.line,
        edge.to_symbol_id
            .as_deref()
            .or(edge.to_symbol_name.as_deref())
            .unwrap_or("")
    )
}
