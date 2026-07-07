use super::*;

pub(super) fn collect_relationships(
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

pub(super) fn add_test_relationships(
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

pub(super) fn page_rank(symbols: &[RustSymbol], relationships: &[CodeRelationship]) -> HashMap<String, f64> {
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

pub(super) fn rank_symbols(
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

pub(super) fn rank_files(
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

pub(super) fn task_match_score(symbol: &RustSymbol, task_terms: &[String]) -> usize {
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

pub(super) fn relationship_kind(symbol: &RustSymbol) -> RelationshipKind {
    match symbol.kind {
        SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias => {
            RelationshipKind::TypeReference
        }
        SymbolKind::Trait => RelationshipKind::TraitReference,
        SymbolKind::Impl => RelationshipKind::ImplReference,
        _ => RelationshipKind::CallsOrMentions,
    }
}

pub(super) fn contains_token(line: &str, token: &str) -> bool {
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

pub(super) fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

pub(super) fn extract_task_terms(user_message: &str) -> Vec<String> {
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

pub(super) fn is_stop_word(term: &str) -> bool {
    matches!(term, "the" | "and" | "for" | "with" | "this" | "that")
}

#[allow(dead_code)]
pub(super) fn _path_for_tests(base: &Path, path: &Path) -> String {
    relative_path(base, path)
}


#[cfg(test)]
#[path = "symbol_graph_tests.rs"]
mod tests;
