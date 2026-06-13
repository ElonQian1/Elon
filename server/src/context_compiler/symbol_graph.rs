use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
};

use super::{
    model::{
        CodeRelationship, RankedFile, RankedSymbol, RelationshipKind, RustIndex, RustSymbol,
        SymbolGraphSummary, SymbolKind, SymbolVisibility,
    },
    repo_map_tags,
    repo_snapshot::{relative_path, source_role},
};

const MAX_RELATIONSHIP_SCAN_BYTES: u64 = 384 * 1024;

pub(crate) fn build_symbol_graph(
    workspace: &Path,
    rust: &RustIndex,
    user_message: &str,
    max_symbols: usize,
    max_relationships: usize,
) -> SymbolGraphSummary {
    if rust.symbols.is_empty() {
        return SymbolGraphSummary {
            warnings: vec!["未抽取到 Rust 符号，跳过符号图。".to_string()],
            ..SymbolGraphSummary::default()
        };
    }

    let task_terms = extract_task_terms(user_message);
    let tag_index =
        repo_map_tags::build_repo_map_tag_index(workspace, &rust.symbols, max_relationships);
    let relationships = repo_map_tags::merge_relationships(
        tag_index.relationships.clone(),
        collect_relationships(workspace, &rust.symbols, max_relationships),
        max_relationships,
    );
    let file_rank = page_rank(&rust.symbols, &relationships);
    let ranked_symbols = rank_symbols(
        &rust.symbols,
        &relationships,
        &file_rank,
        &task_terms,
        max_symbols,
    );
    let ranked_files = rank_files(&rust.symbols, &ranked_symbols, &file_rank, &task_terms);

    SymbolGraphSummary {
        ranked_files,
        ranked_symbols,
        relationships,
        repo_map_tags: tag_index.summary,
        warnings: tag_index.warnings,
    }
}

fn collect_relationships(
    workspace: &Path,
    symbols: &[RustSymbol],
    max_relationships: usize,
) -> Vec<CodeRelationship> {
    let mut by_name = BTreeMap::<String, Vec<&RustSymbol>>::new();
    for symbol in symbols {
        if symbol.name.len() < 3 || symbol.kind == SymbolKind::Impl {
            continue;
        }
        by_name.entry(symbol.name.clone()).or_default().push(symbol);
    }

    let files = symbols
        .iter()
        .map(|symbol| symbol.path.clone())
        .collect::<HashSet<_>>();
    let mut relationships = Vec::new();
    let mut seen = HashSet::new();

    for path in files {
        if relationships.len() >= max_relationships {
            break;
        }
        let full_path = workspace.join(&path);
        if fs::metadata(&full_path)
            .map(|metadata| metadata.len() > MAX_RELATIONSHIP_SCAN_BYTES)
            .unwrap_or(true)
        {
            continue;
        }
        let Ok(content) = fs::read_to_string(&full_path) else {
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            if relationships.len() >= max_relationships {
                break;
            }
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("use ") {
                continue;
            }
            for (name, targets) in &by_name {
                if !contains_token(line, name) {
                    continue;
                }
                for target in targets {
                    if target.path == path {
                        continue;
                    }
                    let key = format!("{}:{}:{}", path, target.id, idx + 1);
                    if !seen.insert(key) {
                        continue;
                    }
                    relationships.push(CodeRelationship {
                        from_path: path.clone(),
                        to_symbol_id: target.id.clone(),
                        to_symbol_name: target.name.clone(),
                        to_path: target.path.clone(),
                        kind: relationship_kind(target),
                        line: idx + 1,
                        reason: format!("line mentions `{name}`"),
                    });
                    break;
                }
            }
        }
    }

    add_test_relationships(symbols, &mut relationships, max_relationships);
    relationships
}

fn add_test_relationships(
    symbols: &[RustSymbol],
    relationships: &mut Vec<CodeRelationship>,
    max_relationships: usize,
) {
    if relationships.len() >= max_relationships {
        return;
    }
    let production_symbols = symbols
        .iter()
        .filter(|symbol| symbol.role != "test" && symbol.name.len() >= 4)
        .collect::<Vec<_>>();
    for test_symbol in symbols.iter().filter(|symbol| symbol.role == "test") {
        if relationships.len() >= max_relationships {
            return;
        }
        let test_name = test_symbol.name.to_ascii_lowercase();
        let Some(target) = production_symbols.iter().find(|symbol| {
            test_name.contains(&symbol.name.to_ascii_lowercase())
                || test_symbol.path.contains(&symbol.path.replace(".rs", ""))
        }) else {
            continue;
        };
        relationships.push(CodeRelationship {
            from_path: test_symbol.path.clone(),
            to_symbol_id: target.id.clone(),
            to_symbol_name: target.name.clone(),
            to_path: target.path.clone(),
            kind: RelationshipKind::TestCovers,
            line: test_symbol.line_start,
            reason: "test symbol appears linked to production symbol".to_string(),
        });
    }
}

fn page_rank(symbols: &[RustSymbol], relationships: &[CodeRelationship]) -> HashMap<String, f64> {
    let files = symbols
        .iter()
        .map(|symbol| symbol.path.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if files.is_empty() {
        return HashMap::new();
    }
    let base = 1.0 / files.len() as f64;
    let mut rank = files
        .iter()
        .map(|path| (path.clone(), base))
        .collect::<HashMap<_, _>>();
    let mut outgoing = HashMap::<String, Vec<String>>::new();
    for relationship in relationships {
        outgoing
            .entry(relationship.from_path.clone())
            .or_default()
            .push(relationship.to_path.clone());
    }

    for _ in 0..16 {
        let mut next = files
            .iter()
            .map(|path| (path.clone(), 0.15 * base))
            .collect::<HashMap<_, _>>();
        for path in &files {
            let current = *rank.get(path).unwrap_or(&base);
            let targets = outgoing.get(path);
            if let Some(targets) = targets.filter(|targets| !targets.is_empty()) {
                let share = 0.85 * current / targets.len() as f64;
                for target in targets {
                    *next.entry(target.clone()).or_insert(0.0) += share;
                }
            } else {
                let share = 0.85 * current / files.len() as f64;
                for target in &files {
                    *next.entry(target.clone()).or_insert(0.0) += share;
                }
            }
        }
        rank = next;
    }
    rank
}

fn rank_symbols(
    symbols: &[RustSymbol],
    relationships: &[CodeRelationship],
    file_rank: &HashMap<String, f64>,
    task_terms: &[String],
    limit: usize,
) -> Vec<RankedSymbol> {
    let mut incoming = HashMap::<String, usize>::new();
    for relationship in relationships {
        *incoming
            .entry(relationship.to_symbol_id.clone())
            .or_insert(0) += 1;
    }

    let mut ranked = symbols
        .iter()
        .map(|symbol| {
            let mut score = file_rank.get(&symbol.path).copied().unwrap_or_default() * 100.0;
            let mut reasons = Vec::new();
            let incoming_count = incoming.get(&symbol.id).copied().unwrap_or_default();
            if incoming_count > 0 {
                score += incoming_count as f64 * 4.0;
                reasons.push(format!("{incoming_count} incoming references"));
            }
            if symbol.visibility != SymbolVisibility::Private {
                score += 3.0;
                reasons.push("public API surface".to_string());
            }
            if matches!(
                symbol.kind,
                SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait | SymbolKind::Function
            ) {
                score += 2.0;
            }
            let task_hits = task_match_score(symbol, task_terms);
            if task_hits > 0 {
                score += task_hits as f64 * 9.0;
                reasons.push(format!("{task_hits} task term hits"));
            }
            if !symbol.safety_notes.is_empty() {
                score += symbol.safety_notes.len() as f64 * 1.5;
                reasons.push(format!("rust notes: {}", symbol.safety_notes.join(", ")));
            }
            RankedSymbol {
                id: symbol.id.clone(),
                name: symbol.name.clone(),
                kind: symbol.kind,
                path: symbol.path.clone(),
                line_start: symbol.line_start,
                line_end: symbol.line_end,
                score,
                reasons,
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    ranked.truncate(limit);
    ranked
}

fn rank_files(
    symbols: &[RustSymbol],
    ranked_symbols: &[RankedSymbol],
    file_rank: &HashMap<String, f64>,
    task_terms: &[String],
) -> Vec<RankedFile> {
    let mut by_file = HashMap::<String, Vec<&RustSymbol>>::new();
    for symbol in symbols {
        by_file.entry(symbol.path.clone()).or_default().push(symbol);
    }
    let top_by_file =
        ranked_symbols
            .iter()
            .fold(HashMap::<String, Vec<String>>::new(), |mut acc, symbol| {
                acc.entry(symbol.path.clone()).or_default().push(format!(
                    "{}:{}",
                    symbol.kind.as_str(),
                    symbol.name
                ));
                acc
            });

    let mut ranked = by_file
        .into_iter()
        .map(|(path, symbols)| {
            let mut score = file_rank.get(&path).copied().unwrap_or_default() * 100.0;
            let mut reasons = Vec::new();
            let task_hits = task_terms
                .iter()
                .filter(|term| path.to_ascii_lowercase().contains(term.as_str()))
                .count();
            if task_hits > 0 {
                score += task_hits as f64 * 14.0;
                reasons.push(format!("{task_hits} task term hits in path"));
            }
            let public_count = symbols
                .iter()
                .filter(|symbol| symbol.visibility != SymbolVisibility::Private)
                .count();
            if public_count > 0 {
                score += public_count as f64;
                reasons.push(format!("{public_count} public symbols"));
            }
            let mut top_symbols = top_by_file.get(&path).cloned().unwrap_or_default();
            top_symbols.truncate(6);
            RankedFile {
                role: source_role(&path),
                path,
                score,
                symbol_count: symbols.len(),
                top_symbols,
                reasons,
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
    });
    ranked.truncate(16);
    ranked
}

fn task_match_score(symbol: &RustSymbol, task_terms: &[String]) -> usize {
    let haystack = format!(
        "{} {} {} {}",
        symbol.path,
        symbol.name,
        symbol.signature,
        symbol.docs.as_deref().unwrap_or("")
    )
    .to_ascii_lowercase();
    task_terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count()
}

fn relationship_kind(symbol: &RustSymbol) -> RelationshipKind {
    match symbol.kind {
        SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias => {
            RelationshipKind::TypeReference
        }
        SymbolKind::Trait => RelationshipKind::TraitReference,
        SymbolKind::Impl => RelationshipKind::ImplReference,
        _ => RelationshipKind::CallsOrMentions,
    }
}

fn contains_token(line: &str, token: &str) -> bool {
    let mut start = 0usize;
    while let Some(idx) = line[start..].find(token) {
        let absolute = start + idx;
        let before = line[..absolute].chars().next_back();
        let after = line[absolute + token.len()..].chars().next();
        if !before.map(is_ident_char).unwrap_or(false) && !after.map(is_ident_char).unwrap_or(false)
        {
            return true;
        }
        start = absolute + token.len();
    }
    false
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn extract_task_terms(user_message: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for raw in user_message
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '/'))
    {
        let term = raw.trim().trim_matches('/').to_ascii_lowercase();
        if term.len() < 3 || is_stop_word(&term) || terms.contains(&term) {
            continue;
        }
        terms.push(term);
        if terms.len() >= 24 {
            break;
        }
    }
    terms
}

fn is_stop_word(term: &str) -> bool {
    matches!(term, "the" | "and" | "for" | "with" | "this" | "that")
}

#[allow(dead_code)]
fn _path_for_tests(base: &Path, path: &Path) -> String {
    relative_path(base, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_task_symbol_and_relationships() {
        let symbols = vec![
            RustSymbol {
                id: "src/map.rs:1:struct:RepoMap".to_string(),
                name: "RepoMap".to_string(),
                kind: SymbolKind::Struct,
                path: "src/map.rs".to_string(),
                line_start: 1,
                line_end: 3,
                visibility: SymbolVisibility::Public,
                signature: "pub struct RepoMap".to_string(),
                parent: None,
                docs: None,
                role: "source",
                safety_notes: Vec::new(),
            },
            RustSymbol {
                id: "src/main.rs:4:function:build".to_string(),
                name: "build".to_string(),
                kind: SymbolKind::Function,
                path: "src/main.rs".to_string(),
                line_start: 4,
                line_end: 6,
                visibility: SymbolVisibility::Private,
                signature: "fn build(map: RepoMap)".to_string(),
                parent: None,
                docs: None,
                role: "source",
                safety_notes: Vec::new(),
            },
        ];
        let relationship = CodeRelationship {
            from_path: "src/main.rs".to_string(),
            to_symbol_id: "src/map.rs:1:struct:RepoMap".to_string(),
            to_symbol_name: "RepoMap".to_string(),
            to_path: "src/map.rs".to_string(),
            kind: RelationshipKind::TypeReference,
            line: 4,
            reason: "line mentions `RepoMap`".to_string(),
        };
        let rank = page_rank(&symbols, &[relationship.clone()]);
        let ranked = rank_symbols(
            &symbols,
            &[relationship],
            &rank,
            &["repomap".to_string()],
            8,
        );

        assert_eq!(ranked[0].name, "RepoMap");
    }
}
