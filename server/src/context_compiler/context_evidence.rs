use std::{collections::HashSet, fs, path::Path};

use sha2::{Digest, Sha256};

use super::{
    model::{
        BuildCommand, ContextEvidence, ContextFact, EvidenceSnippet, FeatureFlagFact,
        NeighborSummary, RankedSymbol, RelationshipKind, RepoContextIndex, RustSymbol,
        SymbolVisibility, TaskProfile, TestTarget,
    },
    relevance::RelevantFile,
};

const MAX_SNIPPETS: usize = 8;
const MAX_SNIPPET_LINES: usize = 160;
const MAX_NEIGHBORS: usize = 12;
const MAX_FACTS: usize = 20;

pub(crate) fn build_context_evidence(
    workspace: &Path,
    index: &RepoContextIndex,
    relevant_files: &[RelevantFile],
) -> ContextEvidence {
    let mut snippets = collect_snippets(workspace, index, relevant_files);
    snippets.sort_by(|left, right| {
        role_priority(right.role)
            .cmp(&role_priority(left.role))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    snippets.truncate(MAX_SNIPPETS);

    let snippet_paths = snippets
        .iter()
        .map(|snippet| snippet.path.clone())
        .collect::<HashSet<_>>();
    let neighbor_summaries = collect_neighbor_summaries(index, &snippet_paths);
    let test_targets = collect_test_targets(index, &snippet_paths);
    let build_commands = collect_build_commands(index, &snippet_paths);
    let invariants = collect_invariants(index);
    let public_api_contracts = collect_public_api_contracts(index);
    let unsafe_boundaries = collect_unsafe_boundaries(index);
    let feature_flags = collect_feature_flags(index);
    let missing_context = collect_missing_context(index, &snippets, &test_targets);
    let recommended_actions = recommended_actions(&index.task);

    ContextEvidence {
        snippets,
        neighbor_summaries,
        test_targets,
        build_commands,
        invariants,
        public_api_contracts,
        unsafe_boundaries,
        feature_flags,
        missing_context,
        recommended_actions,
    }
}

fn collect_snippets(
    workspace: &Path,
    index: &RepoContextIndex,
    relevant_files: &[RelevantFile],
) -> Vec<EvidenceSnippet> {
    let mut snippets = Vec::new();
    let mut seen = HashSet::new();

    for ranked in index.graph.ranked_symbols.iter().take(10) {
        let Some(symbol) = find_symbol(index, &ranked.id) else {
            continue;
        };
        if let Some(snippet) = snippet_for_symbol(workspace, symbol, ranked, &index.task) {
            let key = snippet_key(&snippet);
            if seen.insert(key) {
                snippets.push(snippet);
            }
        }
    }

    for file in relevant_files.iter().take(6) {
        if snippets.len() >= MAX_SNIPPETS {
            break;
        }
        if let Some(snippet) = snippet_for_relevant_file(workspace, file, &index.task) {
            let key = snippet_key(&snippet);
            if seen.insert(key) {
                snippets.push(snippet);
            }
        }
    }

    snippets
}

fn snippet_for_symbol(
    workspace: &Path,
    symbol: &RustSymbol,
    ranked: &RankedSymbol,
    task: &TaskProfile,
) -> Option<EvidenceSnippet> {
    let full_path = workspace.join(&symbol.path);
    let content = fs::read_to_string(&full_path).ok()?;
    let line_count = content.lines().count();
    let line_start = symbol.line_start.max(1);
    let line_end = symbol
        .line_end
        .max(line_start)
        .min(line_start + MAX_SNIPPET_LINES - 1)
        .min(line_count.max(1));
    let snippet = extract_lines(&content, line_start, line_end);
    Some(EvidenceSnippet {
        id: format!("S{}", stable_suffix(&ranked.id)),
        path: symbol.path.clone(),
        role: evidence_role(symbol, task),
        symbols: vec![symbol.id.clone()],
        line_start,
        line_end,
        sha256: sha256_hex(&content),
        reason: ranked.reasons.join("; "),
        content: snippet,
    })
}

fn snippet_for_relevant_file(
    workspace: &Path,
    file: &RelevantFile,
    task: &TaskProfile,
) -> Option<EvidenceSnippet> {
    let full_path = workspace.join(&file.path);
    let content = fs::read_to_string(&full_path).ok()?;
    let line_count = content.lines().count().max(1);
    let focus_line = file.matches.first().map(|item| item.line).unwrap_or(1);
    let line_start = focus_line.saturating_sub(30).max(1);
    let line_end = (line_start + MAX_SNIPPET_LINES - 1).min(line_count);
    Some(EvidenceSnippet {
        id: format!("F{}", stable_suffix(&file.path)),
        path: file.path.clone(),
        role: if task
            .suspected_files
            .iter()
            .any(|path| file.path.ends_with(path) || path.ends_with(&file.path))
        {
            "edit-target"
        } else {
            file.role
        },
        symbols: Vec::new(),
        line_start,
        line_end,
        sha256: sha256_hex(&content),
        reason: file.reasons.join("; "),
        content: extract_lines(&content, line_start, line_end),
    })
}

fn collect_neighbor_summaries(
    index: &RepoContextIndex,
    snippet_paths: &HashSet<String>,
) -> Vec<NeighborSummary> {
    let mut neighbors = Vec::new();
    let mut seen = HashSet::new();
    for relationship in &index.graph.relationships {
        let from_in = snippet_paths.contains(&relationship.from_path);
        let to_in = snippet_paths.contains(&relationship.to_path);
        if from_in == to_in {
            continue;
        }
        let path = if from_in {
            &relationship.to_path
        } else {
            &relationship.from_path
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        neighbors.push(NeighborSummary {
            path: path.clone(),
            relationship: relationship.kind,
            symbols: vec![relationship.to_symbol_name.clone()],
            reason: relationship.reason.clone(),
            needed_if: needed_if(relationship.kind).to_string(),
        });
        if neighbors.len() >= MAX_NEIGHBORS {
            break;
        }
    }
    neighbors
}

fn collect_test_targets(
    index: &RepoContextIndex,
    snippet_paths: &HashSet<String>,
) -> Vec<TestTarget> {
    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    for symbol in &index.rust.symbols {
        if !(symbol.role == "test"
            || symbol.path.contains("/tests/")
            || symbol.path.ends_with("_test.rs"))
        {
            continue;
        }
        if !seen.insert(symbol.path.clone()) {
            continue;
        }
        targets.push(TestTarget {
            path: symbol.path.clone(),
            reason: "test-like Rust file discovered by repo index".to_string(),
        });
        if targets.len() >= 8 {
            return targets;
        }
    }
    for path in snippet_paths {
        if path.contains("context_compiler") && seen.insert("context_compiler".to_string()) {
            targets.push(TestTarget {
                path: path.clone(),
                reason: "context_compiler modules have focused unit tests".to_string(),
            });
        }
    }
    targets
}

fn collect_build_commands(
    index: &RepoContextIndex,
    snippet_paths: &HashSet<String>,
) -> Vec<BuildCommand> {
    let manifest = index
        .cargo
        .manifest_path
        .as_deref()
        .unwrap_or("server/Cargo.toml");
    let mut commands = vec![BuildCommand {
        command: format!("cargo check --manifest-path {manifest} --bin elon-server"),
        reason: "server Rust code path changed or may be affected".to_string(),
    }];
    if snippet_paths
        .iter()
        .any(|path| path.contains("context_compiler"))
    {
        commands.push(BuildCommand {
            command: format!("cargo test --manifest-path {manifest} context_compiler"),
            reason: "context compiler has focused unit coverage".to_string(),
        });
    }
    if index
        .task
        .keywords
        .iter()
        .any(|term| term.contains("test") || term.contains("验证"))
    {
        commands.push(BuildCommand {
            command: format!("cargo test --manifest-path {manifest} --all-targets"),
            reason: "task mentions tests or validation".to_string(),
        });
    }
    commands
}

fn collect_invariants(index: &RepoContextIndex) -> Vec<ContextFact> {
    let mut facts = vec![
        ContextFact {
            subject: "source of truth".to_string(),
            path: "context_pack".to_string(),
            line_start: 0,
            line_end: 0,
            detail: "context pack is navigation evidence only; read real files before editing"
                .to_string(),
        },
        ContextFact {
            subject: "rust refactor".to_string(),
            path: "symbol_graph".to_string(),
            line_start: 0,
            line_end: 0,
            detail: "public API, trait impl, enum match, Drop, unsafe, Send/Sync and await boundaries require caller/test review".to_string(),
        },
    ];
    for symbol in index
        .rust
        .symbols
        .iter()
        .filter(|symbol| !symbol.safety_notes.is_empty())
        .take(6)
    {
        facts.push(ContextFact {
            subject: symbol.name.clone(),
            path: symbol.path.clone(),
            line_start: symbol.line_start,
            line_end: symbol.line_end,
            detail: format!("rust boundary notes: {}", symbol.safety_notes.join(", ")),
        });
    }
    facts.truncate(MAX_FACTS);
    facts
}

fn collect_public_api_contracts(index: &RepoContextIndex) -> Vec<ContextFact> {
    let mut facts = Vec::new();
    for ranked in &index.graph.ranked_symbols {
        let Some(symbol) = find_symbol(index, &ranked.id) else {
            continue;
        };
        if symbol.visibility == SymbolVisibility::Private {
            continue;
        }
        facts.push(ContextFact {
            subject: symbol.name.clone(),
            path: symbol.path.clone(),
            line_start: symbol.line_start,
            line_end: symbol.line_end,
            detail: format!(
                "{} {} is visible as {}; check callers before signature changes",
                symbol.kind.as_str(),
                symbol.name,
                symbol.visibility.as_str()
            ),
        });
        if facts.len() >= MAX_FACTS {
            break;
        }
    }
    facts
}

fn collect_unsafe_boundaries(index: &RepoContextIndex) -> Vec<ContextFact> {
    index
        .rust
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.safety_notes.iter().any(|note| {
                matches!(
                    note.as_str(),
                    "unsafe boundary"
                        | "Drop semantics"
                        | "Send/Sync contract"
                        | "await boundary"
                        | "cfg/feature gate"
                )
            })
        })
        .take(MAX_FACTS)
        .map(|symbol| ContextFact {
            subject: symbol.name.clone(),
            path: symbol.path.clone(),
            line_start: symbol.line_start,
            line_end: symbol.line_end,
            detail: symbol.safety_notes.join(", "),
        })
        .collect()
}

fn collect_feature_flags(index: &RepoContextIndex) -> Vec<FeatureFlagFact> {
    let mut flags = Vec::new();
    for package in &index.cargo.packages {
        for feature in package.features.iter().take(12) {
            flags.push(FeatureFlagFact {
                package: package.name.clone(),
                feature: feature.clone(),
                manifest_path: package.manifest_path.clone(),
            });
            if flags.len() >= MAX_FACTS {
                return flags;
            }
        }
    }
    flags
}

fn collect_missing_context(
    index: &RepoContextIndex,
    snippets: &[EvidenceSnippet],
    tests: &[TestTarget],
) -> Vec<String> {
    let mut items = Vec::new();
    if snippets.is_empty() {
        items.push("no source snippets selected; use search/read tools before editing".to_string());
    }
    if tests.is_empty() {
        items.push("no direct test file identified; run targeted cargo test/check before claiming behavior".to_string());
    }
    if !index.rust_analyzer.available {
        items.push(
            "rust-analyzer not available; semantic enhancement is syntax/grep based only"
                .to_string(),
        );
    }
    if index.graph.relationships.is_empty() {
        items.push(
            "no cross-file relationships found; caller/callee impact may be incomplete".to_string(),
        );
    }
    items
}

fn recommended_actions(task: &TaskProfile) -> Vec<String> {
    let mut actions = vec![
        "Open every edit-target snippet from disk before patching; snippets are not source of truth.".to_string(),
        "Check callers/callees from neighbor_summaries before changing signatures or ownership.".to_string(),
        "Run the listed build_commands after editing Rust code.".to_string(),
    ];
    actions.extend(task.action_hints.iter().cloned());
    actions
}

fn find_symbol<'a>(index: &'a RepoContextIndex, id: &str) -> Option<&'a RustSymbol> {
    index.rust.symbols.iter().find(|symbol| symbol.id == id)
}

fn evidence_role(symbol: &RustSymbol, task: &TaskProfile) -> &'static str {
    if task
        .suspected_files
        .iter()
        .any(|path| symbol.path.ends_with(path) || path.ends_with(&symbol.path))
        || task
            .suspected_symbols
            .iter()
            .any(|name| symbol.name == *name || symbol.id.contains(name))
    {
        "edit-target"
    } else if symbol.role == "test" {
        "test"
    } else if symbol.visibility != SymbolVisibility::Private {
        "interface"
    } else {
        "caller"
    }
}

fn needed_if(kind: RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::CallsOrMentions => "modifying call shape or behavior",
        RelationshipKind::TypeReference => "changing type fields, constructors, or ownership",
        RelationshipKind::TraitReference => "changing trait methods or bounds",
        RelationshipKind::ImplReference => "changing impl receiver, trait target, or generics",
        RelationshipKind::TestCovers => "updating expected behavior",
    }
}

fn extract_lines(content: &str, line_start: usize, line_end: usize) -> String {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line_number = idx + 1;
            (line_number >= line_start && line_number <= line_end).then_some(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn stable_suffix(value: &str) -> String {
    sha256_hex(value).chars().take(10).collect()
}

fn snippet_key(snippet: &EvidenceSnippet) -> String {
    format!(
        "{}:{}:{}",
        snippet.path, snippet.line_start, snippet.line_end
    )
}

fn role_priority(role: &str) -> usize {
    match role {
        "edit-target" => 5,
        "interface" => 4,
        "caller" => 3,
        "test" => 2,
        _ => 1,
    }
}
