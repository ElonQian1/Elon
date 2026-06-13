use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::Path,
};

use super::{
    model::{
        CodeRelationship, RelationshipKind, RepoMapTagEdge, RepoMapTagSummary, RustSymbol,
        SymbolKind,
    },
    repo_snapshot::relative_path,
};

const MAX_TAG_SCAN_BYTES: u64 = 512 * 1024;
const MAX_SUMMARY_EDGES: usize = 24;

pub(crate) struct RepoMapTagIndex {
    pub(crate) summary: RepoMapTagSummary,
    pub(crate) relationships: Vec<CodeRelationship>,
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn build_repo_map_tag_index(
    workspace: &Path,
    symbols: &[RustSymbol],
    max_relationships: usize,
) -> RepoMapTagIndex {
    let definitions = build_definitions(symbols);
    let definition_count = definitions.values().map(Vec::len).sum();
    let mut builders = BTreeMap::<String, EdgeBuilder>::new();
    let mut warnings = Vec::new();
    let files = symbols
        .iter()
        .map(|symbol| symbol.path.clone())
        .collect::<BTreeSet<_>>();

    for path in files {
        let full_path = workspace.join(&path);
        if fs::metadata(&full_path)
            .map(|metadata| metadata.len() > MAX_TAG_SCAN_BYTES)
            .unwrap_or(true)
        {
            warnings.push(format!(
                "Aider-style tag scan skipped oversized file: {}",
                relative_path(workspace, &full_path)
            ));
            continue;
        }
        let Ok(content) = fs::read_to_string(&full_path) else {
            warnings.push(format!(
                "Aider-style tag scan failed to read file: {}",
                relative_path(workspace, &full_path)
            ));
            continue;
        };
        collect_reference_tags(&path, &content, &definitions, &mut builders);
    }

    let mut edges = builders
        .into_values()
        .map(EdgeBuilder::into_edge)
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.references.cmp(&left.references))
            .then_with(|| left.from_path.cmp(&right.from_path))
            .then_with(|| left.symbol.cmp(&right.symbol))
    });

    let relationships = edges
        .iter()
        .take(max_relationships)
        .map(edge_to_relationship)
        .collect::<Vec<_>>();
    let references = edges.iter().map(|edge| edge.references).sum();
    edges.truncate(MAX_SUMMARY_EDGES);

    RepoMapTagIndex {
        summary: RepoMapTagSummary {
            definitions: definition_count,
            references,
            edges,
            warnings: warnings.clone(),
        },
        relationships,
        warnings,
    }
}

pub(crate) fn merge_relationships(
    tagged: Vec<CodeRelationship>,
    heuristic: Vec<CodeRelationship>,
    max_relationships: usize,
) -> Vec<CodeRelationship> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();
    for relationship in tagged.into_iter().chain(heuristic) {
        let key = format!(
            "{}:{}:{}:{}",
            relationship.from_path,
            relationship.to_symbol_id,
            relationship.line,
            relationship.kind.as_str()
        );
        if !seen.insert(key) {
            continue;
        }
        merged.push(relationship);
        if merged.len() >= max_relationships {
            break;
        }
    }
    merged
}

fn build_definitions<'a>(symbols: &'a [RustSymbol]) -> BTreeMap<String, Vec<&'a RustSymbol>> {
    let mut definitions = BTreeMap::<String, Vec<&RustSymbol>>::new();
    for symbol in symbols {
        if symbol.name.len() < 3 || symbol.kind == SymbolKind::Impl {
            continue;
        }
        definitions
            .entry(symbol.name.clone())
            .or_default()
            .push(symbol);
    }
    for targets in definitions.values_mut() {
        targets.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.line_start.cmp(&right.line_start))
        });
    }
    definitions
}

fn collect_reference_tags(
    path: &str,
    content: &str,
    definitions: &BTreeMap<String, Vec<&RustSymbol>>,
    builders: &mut BTreeMap<String, EdgeBuilder>,
) {
    for (idx, line) in content.lines().enumerate() {
        let code = strip_line_noise(line);
        if code.trim().is_empty() {
            continue;
        }
        let identifiers = extract_identifiers(&code);
        for name in identifiers {
            let Some(targets) = definitions.get(&name) else {
                continue;
            };
            if let Some(target) = choose_target(path, idx + 1, targets) {
                let key = format!("{path}:{}", target.id);
                builders
                    .entry(key)
                    .or_insert_with(|| EdgeBuilder::new(path, target))
                    .lines
                    .insert(idx + 1);
            }
        }
    }
}

fn choose_target<'a>(
    from_path: &str,
    line: usize,
    targets: &[&'a RustSymbol],
) -> Option<&'a RustSymbol> {
    targets
        .iter()
        .copied()
        .filter(|target| {
            !(target.path == from_path && line >= target.line_start && line <= target.line_end)
        })
        .find(|target| target.path != from_path)
}

struct EdgeBuilder {
    symbol: String,
    target_id: String,
    from_path: String,
    to_path: String,
    definition_line: usize,
    relationship_kind: RelationshipKind,
    lines: BTreeSet<usize>,
}

impl EdgeBuilder {
    fn new(from_path: &str, target: &RustSymbol) -> Self {
        Self {
            symbol: target.name.clone(),
            target_id: target.id.clone(),
            from_path: from_path.to_string(),
            to_path: target.path.clone(),
            definition_line: target.line_start,
            relationship_kind: relationship_kind(target),
            lines: BTreeSet::new(),
        }
    }

    fn into_edge(self) -> RepoMapTagEdge {
        let reference_lines = self.lines.iter().copied().collect::<Vec<_>>();
        let references = reference_lines.len();
        let score = references as f64 * 4.0 + edge_kind_weight(self.relationship_kind);
        RepoMapTagEdge {
            symbol: self.symbol,
            target_symbol_id: self.target_id,
            from_path: self.from_path,
            to_path: self.to_path,
            definition_line: self.definition_line,
            reference_lines,
            references,
            score,
            reason: "Aider-style def/ref tag edge".to_string(),
        }
    }
}

fn edge_to_relationship(edge: &RepoMapTagEdge) -> CodeRelationship {
    CodeRelationship {
        from_path: edge.from_path.clone(),
        to_symbol_id: edge.target_symbol_id.clone(),
        to_symbol_name: edge.symbol.clone(),
        to_path: edge.to_path.clone(),
        kind: infer_edge_kind(edge),
        line: edge.reference_lines.first().copied().unwrap_or(1),
        reason: format!(
            "Aider-style def/ref tags found {} references to `{}`",
            edge.references, edge.symbol
        ),
    }
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

fn infer_edge_kind(edge: &RepoMapTagEdge) -> RelationshipKind {
    if edge
        .symbol
        .chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
    {
        RelationshipKind::TypeReference
    } else {
        RelationshipKind::CallsOrMentions
    }
}

fn edge_kind_weight(kind: RelationshipKind) -> f64 {
    match kind {
        RelationshipKind::TraitReference => 3.0,
        RelationshipKind::TypeReference | RelationshipKind::ImplReference => 2.0,
        RelationshipKind::TestCovers => 1.5,
        RelationshipKind::CallsOrMentions => 1.0,
    }
}

fn strip_line_noise(line: &str) -> String {
    let code = line.split("//").next().unwrap_or(line);
    let mut out = String::with_capacity(code.len());
    let mut in_string = false;
    let mut escaped = false;
    for ch in code.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            out.push(' ');
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

fn extract_identifiers(line: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut seen = HashSet::new();
    let mut current = String::new();
    for ch in line.chars().chain(std::iter::once(' ')) {
        if is_ident_char(ch) {
            current.push(ch);
            continue;
        }
        if current.len() >= 3 && !is_keyword(&current) && seen.insert(current.clone()) {
            identifiers.push(current.clone());
        }
        current.clear();
    }
    identifiers
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "for"
            | "from"
            | "fn"
            | "if"
            | "impl"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
}
