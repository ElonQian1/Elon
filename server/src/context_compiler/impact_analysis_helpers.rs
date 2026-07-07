use super::*;

pub(super) fn load_sources(workspace: &Path, symbols: &[RustSymbol]) -> HashMap<String, Vec<String>> {
    let mut sources = HashMap::new();
    for path in symbols
        .iter()
        .map(|symbol| symbol.path.clone())
        .collect::<HashSet<_>>()
    {
        let full_path = workspace.join(&path);
        if fs::metadata(&full_path)
            .map(|metadata| metadata.len() > MAX_SCAN_BYTES)
            .unwrap_or(true)
        {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&full_path) {
            sources.insert(path, content.lines().map(ToString::to_string).collect());
        }
    }
    sources
}

pub(super) fn collect_trait_implementations(index: &RepoContextIndex) -> Vec<ImpactFact> {
    let traits = index
        .rust
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Trait)
        .collect::<Vec<_>>();
    if traits.is_empty() {
        return Vec::new();
    }

    let mut facts = Vec::new();
    for implementation in index
        .rust
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Impl)
    {
        let signature = implementation.signature.as_str();
        for trait_symbol in &traits {
            if !impl_mentions_trait(signature, &trait_symbol.name) {
                continue;
            }
            facts.push(ImpactFact {
                subject: format!("{} -> {}", trait_symbol.name, implementation.name),
                path: implementation.path.clone(),
                line: implementation.line_start,
                kind: ImpactKind::TraitImplementation,
                evidence: compact(signature, 180),
                reason: "impl signature mentions ranked trait; changing trait methods or bounds affects this impl".to_string(),
            });
            if facts.len() >= MAX_FACTS_PER_KIND {
                return facts;
            }
        }
    }
    facts
}

pub(super) fn collect_function_call_sites(
    index: &RepoContextIndex,
    sources: &HashMap<String, Vec<String>>,
) -> Vec<ImpactFact> {
    let functions = index
        .graph
        .ranked_symbols
        .iter()
        .filter(|ranked| ranked.kind == SymbolKind::Function)
        .take(24)
        .filter_map(|ranked| find_symbol(index, &ranked.id))
        .collect::<Vec<_>>();
    let mut facts = Vec::new();
    let mut seen = HashSet::new();

    for function in functions {
        if function.name.len() < 3 {
            continue;
        }
        for (path, lines) in sources {
            for (idx, line) in lines.iter().enumerate() {
                if path == &function.path
                    && idx + 1 >= function.line_start
                    && idx + 1 <= function.line_end
                {
                    continue;
                }
                if !looks_like_call(line, &function.name) {
                    continue;
                }
                let key = format!("{path}:{}:{}", idx + 1, function.name);
                if !seen.insert(key) {
                    continue;
                }
                facts.push(ImpactFact {
                    subject: function.name.clone(),
                    path: path.clone(),
                    line: idx + 1,
                    kind: ImpactKind::FunctionCallSite,
                    evidence: compact(line.trim(), 180),
                    reason: "call-like token hit; signature or behavior changes should inspect this caller".to_string(),
                });
                if facts.len() >= MAX_FACTS_PER_KIND {
                    return facts;
                }
            }
        }
    }
    facts
}

pub(super) fn collect_enum_match_sites(
    index: &RepoContextIndex,
    sources: &HashMap<String, Vec<String>>,
) -> Vec<ImpactFact> {
    let mut facts = Vec::new();
    for enum_symbol in index
        .graph
        .ranked_symbols
        .iter()
        .filter(|ranked| ranked.kind == SymbolKind::Enum)
        .take(10)
        .filter_map(|ranked| find_symbol(index, &ranked.id))
    {
        let variants = extract_enum_variants(enum_symbol, sources);
        for (path, lines) in sources {
            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if !(trimmed.contains("match ")
                    || trimmed.contains("matches!")
                    || variants.iter().any(|variant| {
                        trimmed.contains(&format!("{}::{variant}", enum_symbol.name))
                            || contains_token(trimmed, variant)
                    }))
                {
                    continue;
                }
                facts.push(ImpactFact {
                    subject: enum_symbol.name.clone(),
                    path: path.clone(),
                    line: idx + 1,
                    kind: ImpactKind::EnumMatchSite,
                    evidence: compact(trimmed, 180),
                    reason: "enum or variant appears near match-like control flow; adding variants may require updates".to_string(),
                });
                if facts.len() >= MAX_FACTS_PER_KIND {
                    return facts;
                }
            }
        }
    }
    facts
}

pub(super) fn collect_field_accesses(
    index: &RepoContextIndex,
    sources: &HashMap<String, Vec<String>>,
) -> Vec<ImpactFact> {
    let mut facts = Vec::new();
    let mut seen = HashSet::new();
    for struct_symbol in index
        .graph
        .ranked_symbols
        .iter()
        .filter(|ranked| ranked.kind == SymbolKind::Struct)
        .take(8)
        .filter_map(|ranked| find_symbol(index, &ranked.id))
    {
        let fields = extract_struct_fields(struct_symbol, sources);
        for field in fields {
            if field.len() < 3 {
                continue;
            }
            for (path, lines) in sources {
                for (idx, line) in lines.iter().enumerate() {
                    let Some(kind) = field_access_kind(line, &field) else {
                        continue;
                    };
                    let key = format!("{path}:{}:{field}:{}", idx + 1, kind.as_str());
                    if !seen.insert(key) {
                        continue;
                    }
                    facts.push(ImpactFact {
                        subject: format!("{}.{}", struct_symbol.name, field),
                        path: path.clone(),
                        line: idx + 1,
                        kind,
                        evidence: compact(line.trim(), 180),
                        reason: "struct field access found; field rename/type/ownership changes affect this line".to_string(),
                    });
                    if facts.len() >= MAX_FACTS_PER_KIND {
                        return facts;
                    }
                }
            }
        }
    }
    facts
}

pub(super) fn collect_public_api_references(index: &RepoContextIndex) -> Vec<ImpactFact> {
    let by_id = index
        .rust
        .symbols
        .iter()
        .map(|symbol| (symbol.id.as_str(), symbol))
        .collect::<HashMap<_, _>>();
    let mut facts = Vec::new();
    for relationship in &index.graph.relationships {
        if relationship.kind == RelationshipKind::TestCovers {
            continue;
        }
        let Some(target) = by_id.get(relationship.to_symbol_id.as_str()) else {
            continue;
        };
        if target.visibility == SymbolVisibility::Private {
            continue;
        }
        facts.push(ImpactFact {
            subject: target.name.clone(),
            path: relationship.from_path.clone(),
            line: relationship.line,
            kind: ImpactKind::PublicApiReference,
            evidence: relationship.reason.clone(),
            reason: format!(
                "{} is {}; public API changes should inspect this reference",
                target.kind.as_str(),
                target.visibility.as_str()
            ),
        });
        if facts.len() >= MAX_FACTS_PER_KIND {
            break;
        }
    }
    facts
}

pub(super) fn collect_test_links(
    index: &RepoContextIndex,
    sources: &HashMap<String, Vec<String>>,
) -> Vec<ImpactFact> {
    let mut facts = index
        .graph
        .relationships
        .iter()
        .filter(|relationship| relationship.kind == RelationshipKind::TestCovers)
        .take(MAX_FACTS_PER_KIND)
        .map(|relationship| ImpactFact {
            subject: relationship.to_symbol_name.clone(),
            path: relationship.from_path.clone(),
            line: relationship.line,
            kind: ImpactKind::TestLink,
            evidence: relationship.reason.clone(),
            reason: "symbol graph linked this test to production code".to_string(),
        })
        .collect::<Vec<_>>();

    if facts.len() >= MAX_FACTS_PER_KIND {
        return facts;
    }

    let production = index
        .graph
        .ranked_symbols
        .iter()
        .take(30)
        .filter_map(|ranked| find_symbol(index, &ranked.id))
        .filter(|symbol| symbol.role != "test" && symbol.name.len() >= 4)
        .collect::<Vec<_>>();
    for (path, lines) in sources.iter().filter(|(path, _)| path.contains("test")) {
        for (idx, line) in lines.iter().enumerate() {
            let Some(symbol) = production
                .iter()
                .find(|symbol| contains_token(line, &symbol.name))
            else {
                continue;
            };
            facts.push(ImpactFact {
                subject: symbol.name.clone(),
                path: path.clone(),
                line: idx + 1,
                kind: ImpactKind::TestLink,
                evidence: compact(line.trim(), 180),
                reason: "test file mentions ranked production symbol".to_string(),
            });
            if facts.len() >= MAX_FACTS_PER_KIND {
                return facts;
            }
        }
    }
    facts
}

pub(super) fn collect_async_boundaries(index: &RepoContextIndex) -> Vec<ImpactFact> {
    index
        .graph
        .ranked_symbols
        .iter()
        .filter_map(|ranked| find_symbol(index, &ranked.id))
        .filter(|symbol| {
            symbol.safety_notes.iter().any(|note| {
                matches!(
                    note.as_str(),
                    "await boundary" | "Drop semantics" | "Send/Sync contract" | "unsafe boundary"
                )
            })
        })
        .take(MAX_FACTS_PER_KIND)
        .map(|symbol| ImpactFact {
            subject: symbol.name.clone(),
            path: symbol.path.clone(),
            line: symbol.line_start,
            kind: ImpactKind::AsyncBoundary,
            evidence: symbol.signature.clone(),
            reason: format!("rust boundary notes: {}", symbol.safety_notes.join(", ")),
        })
        .collect()
}

pub(super) fn collect_limitations(
    index: &RepoContextIndex,
    sources: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut limitations = Vec::new();
    if !index.rust_analyzer.available {
        limitations.push(
            "rust-analyzer semantic report is unavailable; impact facts are syntax/graph heuristics"
                .to_string(),
        );
    }
    if sources.len() < index.rust.files_scanned {
        limitations.push(
            "some scanned Rust files were skipped for impact analysis due to size/read limits"
                .to_string(),
        );
    }
    if index.impact_is_empty_hint() {
        limitations.push("no high-confidence impact facts were inferred; agent must query callers/references before editing".to_string());
    }
    limitations
}

trait ImpactEmptyHint {
    fn impact_is_empty_hint(&self) -> bool;
}

impl ImpactEmptyHint for RepoContextIndex {
    fn impact_is_empty_hint(&self) -> bool {
        self.graph.relationships.is_empty() && self.graph.ranked_symbols.is_empty()
    }
}

pub(super) fn extract_struct_fields(
    symbol: &RustSymbol,
    sources: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    extract_body_lines(symbol, sources)
        .into_iter()
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches(',');
            if trimmed.starts_with("//")
                || trimmed.starts_with("#[")
                || trimmed.contains(" fn ")
                || trimmed == "{"
                || trimmed == "}"
            {
                return None;
            }
            let before_colon = trimmed.split_once(':')?.0;
            let name = before_colon
                .split_whitespace()
                .last()
                .unwrap_or(before_colon)
                .trim_start_matches("r#");
            is_identifier(name).then(|| name.to_string())
        })
        .take(12)
        .collect()
}

pub(super) fn extract_enum_variants(
    symbol: &RustSymbol,
    sources: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    extract_body_lines(symbol, sources)
        .into_iter()
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches(',');
            if trimmed.starts_with("//")
                || trimmed.starts_with("#[")
                || trimmed == "{"
                || trimmed == "}"
                || trimmed.contains(':')
            {
                return None;
            }
            let name = trimmed
                .split(['(', '{', '=', ' '])
                .next()
                .unwrap_or_default()
                .trim_start_matches("r#");
            is_identifier(name).then(|| name.to_string())
        })
        .take(20)
        .collect()
}

pub(super) fn extract_body_lines(symbol: &RustSymbol, sources: &HashMap<String, Vec<String>>) -> Vec<String> {
    sources
        .get(&symbol.path)
        .into_iter()
        .flat_map(|lines| {
            lines
                .iter()
                .skip(symbol.line_start)
                .take(
                    symbol
                        .line_end
                        .saturating_sub(symbol.line_start)
                        .saturating_sub(1),
                )
                .cloned()
        })
        .collect()
}

pub(super) fn impl_mentions_trait(signature: &str, trait_name: &str) -> bool {
    signature.contains(&format!(" {trait_name} for "))
        || signature.contains(&format!("::{trait_name} for "))
}

pub(super) fn looks_like_call(line: &str, name: &str) -> bool {
    line.contains(&format!("{name}(")) || line.contains(&format!(".{name}("))
}

pub(super) fn field_access_kind(line: &str, field: &str) -> Option<ImpactKind> {
    if line.contains(&format!(".{field} =")) || line.contains(&format!("{field}:")) {
        return Some(ImpactKind::FieldWrite);
    }
    if line.contains(&format!(".{field}")) {
        return Some(ImpactKind::FieldRead);
    }
    None
}

pub(super) fn find_symbol<'a>(index: &'a RepoContextIndex, id: &str) -> Option<&'a RustSymbol> {
    index.rust.symbols.iter().find(|symbol| symbol.id == id)
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

pub(super) fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub(super) fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

pub(super) fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
