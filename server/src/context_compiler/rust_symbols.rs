use std::{fs, path::Path};

use super::{
    model::{RustIndex, RustSymbol, SymbolKind, SymbolVisibility},
    repo_snapshot::{relative_path, source_role},
    repo_walk,
};

const MAX_RUST_FILE_BYTES: u64 = 512 * 1024;

pub(crate) fn collect_rust_index(workspace: &Path, max_files: usize) -> RustIndex {
    let mut index = RustIndex::default();
    scan_dir(workspace, workspace, max_files, &mut index);
    assign_impl_parents(&mut index.symbols);
    index
}

fn scan_dir(base: &Path, _dir: &Path, max_files: usize, index: &mut RustIndex) {
    for path in repo_walk::collect_matching_files(base, max_files, is_rust_file) {
        if index.files_scanned >= max_files {
            return;
        }
        if fs::metadata(&path)
            .map(|metadata| metadata.len() > MAX_RUST_FILE_BYTES)
            .unwrap_or(true)
        {
            index.warnings.push(format!(
                "跳过过大的 Rust 文件：{}",
                relative_path(base, &path)
            ));
            continue;
        }
        index.files_scanned += 1;
        match fs::read_to_string(&path) {
            Ok(content) => index
                .symbols
                .extend(extract_file_symbols(base, &path, &content)),
            Err(_) => index.warnings.push(format!(
                "读取 Rust 文件失败：{}",
                relative_path(base, &path)
            )),
        }
    }
}

fn is_rust_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("rs")
}

fn extract_file_symbols(base: &Path, path: &Path, content: &str) -> Vec<RustSymbol> {
    let relative = relative_path(base, path);
    let role = source_role(&relative);
    let lines = content.lines().collect::<Vec<_>>();
    let mut symbols = Vec::new();
    let mut pending_docs = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if let Some(doc) = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
        {
            pending_docs.push(doc.trim().to_string());
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("#[") || trimmed.starts_with("//") {
            continue;
        }

        let Some(parsed) = parse_symbol_line(trimmed) else {
            pending_docs.clear();
            continue;
        };
        let line_start = idx + 1;
        let line_end = find_block_end(&lines, idx).unwrap_or(line_start);
        let signature = collect_signature(&lines, idx, line_end);
        let docs = (!pending_docs.is_empty()).then(|| compact(&pending_docs.join(" "), 180));
        let safety_notes = rust_safety_notes(&parsed, &signature, &lines, line_start, line_end);
        let id = format!(
            "{}:{}:{}:{}",
            relative,
            line_start,
            parsed.kind.as_str(),
            parsed.name
        );

        symbols.push(RustSymbol {
            id,
            name: parsed.name,
            kind: parsed.kind,
            path: relative.clone(),
            line_start,
            line_end,
            visibility: parsed.visibility,
            signature,
            parent: None,
            docs,
            role,
            safety_notes,
        });
        pending_docs.clear();
    }

    symbols
}

#[derive(Debug, Clone)]
struct ParsedSymbol {
    name: String,
    kind: SymbolKind,
    visibility: SymbolVisibility,
}

fn parse_symbol_line(line: &str) -> Option<ParsedSymbol> {
    if let Some(name) = parse_macro(line) {
        return Some(ParsedSymbol {
            name,
            kind: SymbolKind::Macro,
            visibility: visibility(line).0,
        });
    }
    if let Some(name) = parse_impl(line) {
        return Some(ParsedSymbol {
            name,
            kind: SymbolKind::Impl,
            visibility: visibility(line).0,
        });
    }

    let (visibility, without_visibility) = visibility(line);
    for (keyword, kind) in [
        ("struct", SymbolKind::Struct),
        ("enum", SymbolKind::Enum),
        ("trait", SymbolKind::Trait),
        ("fn", SymbolKind::Function),
        ("type", SymbolKind::TypeAlias),
        ("const", SymbolKind::Const),
        ("static", SymbolKind::Static),
        ("mod", SymbolKind::Module),
    ] {
        if let Some(rest) = after_keyword(without_visibility, keyword) {
            if let Some(name) = parse_identifier(rest) {
                return Some(ParsedSymbol {
                    name,
                    kind,
                    visibility,
                });
            }
        }
    }
    None
}

fn visibility(line: &str) -> (SymbolVisibility, &str) {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("pub(crate)") {
        return (SymbolVisibility::Crate, rest.trim_start());
    }
    if let Some(rest) = trimmed.strip_prefix("pub ") {
        return (SymbolVisibility::Public, rest.trim_start());
    }
    if let Some(rest) = trimmed.strip_prefix("pub(") {
        let rest = rest
            .split_once(')')
            .map(|(_, tail)| tail.trim_start())
            .unwrap_or(trimmed);
        return (SymbolVisibility::Crate, rest);
    }
    (SymbolVisibility::Private, trimmed)
}

fn parse_macro(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("macro_rules!")?;
    parse_identifier(rest.trim_start_matches(|ch: char| ch == ' ' || ch == '\t' || ch == '('))
}

fn parse_impl(line: &str) -> Option<String> {
    let trimmed = line
        .trim_start()
        .strip_prefix("unsafe ")
        .unwrap_or(line.trim_start());
    let rest = trimmed.strip_prefix("impl ")?;
    let name = rest
        .split('{')
        .next()
        .unwrap_or(rest)
        .split(" where ")
        .next()
        .unwrap_or(rest)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!name.is_empty()).then(|| format!("impl {name}"))
}

fn after_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let mut offset = 0usize;
    for token in ["async ", "unsafe ", "const ", "extern \"C\" "] {
        if let Some(rest) = line
            .get(offset..)
            .and_then(|value| value.strip_prefix(token))
        {
            offset = line.len() - rest.len();
        }
    }
    find_keyword(&line[offset..], keyword).map(|idx| &line[offset + idx + keyword.len()..])
}

fn find_keyword(line: &str, keyword: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let needle = keyword.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return None;
    }
    for idx in 0..=bytes.len() - needle.len() {
        if &bytes[idx..idx + needle.len()] != needle {
            continue;
        }
        let before = line[..idx].chars().next_back();
        let after = line[idx + needle.len()..].chars().next();
        if before.map(is_ident_char).unwrap_or(false) || after.map(is_ident_char).unwrap_or(false) {
            continue;
        }
        return Some(idx);
    }
    None
}

fn parse_identifier(rest: &str) -> Option<String> {
    let trimmed = rest.trim_start_matches(|ch: char| !is_ident_start(ch));
    let mut name = String::new();
    for ch in trimmed.chars() {
        if !is_ident_char(ch) {
            break;
        }
        name.push(ch);
    }
    (!name.is_empty()).then_some(name)
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn find_block_end(lines: &[&str], start_idx: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut seen_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    seen_open = true;
                }
                '}' if seen_open => {
                    depth -= 1;
                    if depth <= 0 {
                        return Some(idx + 1);
                    }
                }
                _ => {}
            }
        }
        if idx == start_idx && !seen_open && line.trim_end().ends_with(';') {
            return Some(idx + 1);
        }
    }
    Some(start_idx + 1)
}

fn collect_signature(lines: &[&str], start_idx: usize, line_end: usize) -> String {
    let mut collected = Vec::new();
    for line in lines
        .iter()
        .skip(start_idx)
        .take(line_end.saturating_sub(start_idx).min(6))
    {
        let trimmed = line.trim();
        collected.push(trimmed);
        if trimmed.ends_with('{') || trimmed.ends_with(';') || trimmed.contains('{') {
            break;
        }
    }
    compact(&collected.join(" "), 240)
}

fn rust_safety_notes(
    parsed: &ParsedSymbol,
    signature: &str,
    lines: &[&str],
    line_start: usize,
    line_end: usize,
) -> Vec<String> {
    let body = lines
        .iter()
        .skip(line_start.saturating_sub(1))
        .take(line_end.saturating_sub(line_start).saturating_add(1))
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    let mut notes = Vec::new();
    if signature.contains("unsafe ") || body.contains("unsafe {") {
        notes.push("unsafe boundary".to_string());
    }
    if parsed.kind == SymbolKind::Impl && signature.contains("Drop for") {
        notes.push("Drop semantics".to_string());
    }
    if signature.contains("Send") || signature.contains("Sync") || body.contains("Send + Sync") {
        notes.push("Send/Sync contract".to_string());
    }
    if signature.contains("Result<") || body.contains("anyhow::") || body.contains("thiserror") {
        notes.push("error propagation".to_string());
    }
    if signature.contains("async fn") || body.contains(".await") {
        notes.push("await boundary".to_string());
    }
    if body.contains("#[cfg(") || body.contains("cfg!(") {
        notes.push("cfg/feature gate".to_string());
    }
    if body.contains("match ") && (body.contains("enum") || parsed.kind == SymbolKind::Enum) {
        notes.push("enum match surface".to_string());
    }
    notes
}

fn assign_impl_parents(symbols: &mut [RustSymbol]) {
    let impls = symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Impl)
        .map(|symbol| {
            (
                symbol.id.clone(),
                symbol.path.clone(),
                symbol.line_start,
                symbol.line_end,
            )
        })
        .collect::<Vec<_>>();
    for symbol in symbols {
        if symbol.kind == SymbolKind::Impl || symbol.parent.is_some() {
            continue;
        }
        if let Some((impl_id, _, _, _)) = impls.iter().find(|(_, path, start, end)| {
            *path == symbol.path && symbol.line_start > *start && symbol.line_start <= *end
        }) {
            symbol.parent = Some(impl_id.clone());
        }
    }
}

fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_symbols_and_impl_parent() {
        let content = r#"
/// Keeps repo context.
pub struct ContextBundle {
    value: usize,
}

impl ContextBundle {
    pub async fn build() -> anyhow::Result<Self> {
        Ok(Self { value: 1 })
    }
}
"#;
        let symbols = extract_file_symbols(Path::new("."), Path::new("src/context.rs"), content);

        assert!(symbols
            .iter()
            .any(|symbol| symbol.name == "ContextBundle" && symbol.kind == SymbolKind::Struct));
        assert!(symbols.iter().any(|symbol| symbol.name == "build"
            && symbol.safety_notes.contains(&"await boundary".to_string())));
    }
}
