use std::path::{Path, PathBuf};

use serde_json::Value;

use super::model::{
    RustAnalyzerLspLocation, RustAnalyzerLspLocationRole, SemanticQuery, SemanticQueryMethod,
};

const MAX_LOCATIONS: usize = 24;

pub(super) fn parse_lsp_locations(
    workspace: &Path,
    query: &SemanticQuery,
    value: &Value,
) -> Vec<RustAnalyzerLspLocation> {
    let mut locations = Vec::new();
    match query.method {
        SemanticQueryMethod::DocumentSymbol => {
            parse_document_symbols(&mut locations, &query.path, value);
        }
        SemanticQueryMethod::References => {
            parse_location_array(
                &mut locations,
                workspace,
                value,
                RustAnalyzerLspLocationRole::Reference,
            );
        }
        SemanticQueryMethod::Implementation => {
            parse_location_array(
                &mut locations,
                workspace,
                value,
                RustAnalyzerLspLocationRole::Implementation,
            );
        }
        SemanticQueryMethod::PrepareCallHierarchy => {
            parse_call_hierarchy_items(
                &mut locations,
                workspace,
                value,
                RustAnalyzerLspLocationRole::CallHierarchyItem,
                None,
            );
        }
        SemanticQueryMethod::IncomingCalls => {
            parse_call_hierarchy_calls(
                &mut locations,
                workspace,
                value,
                "from",
                RustAnalyzerLspLocationRole::IncomingCaller,
            );
        }
        SemanticQueryMethod::OutgoingCalls => {
            parse_call_hierarchy_calls(
                &mut locations,
                workspace,
                value,
                "to",
                RustAnalyzerLspLocationRole::OutgoingCallee,
            );
        }
        _ => {}
    }
    dedupe_locations(locations)
}

fn parse_location_array(
    out: &mut Vec<RustAnalyzerLspLocation>,
    workspace: &Path,
    value: &Value,
    role: RustAnalyzerLspLocationRole,
) {
    if let Some(items) = value.as_array() {
        for item in items {
            parse_location_like(out, workspace, item, role);
            if out.len() >= MAX_LOCATIONS {
                break;
            }
        }
        return;
    }
    parse_location_like(out, workspace, value, role);
}

fn parse_location_like(
    out: &mut Vec<RustAnalyzerLspLocation>,
    workspace: &Path,
    value: &Value,
    role: RustAnalyzerLspLocationRole,
) {
    let uri = value
        .get("uri")
        .or_else(|| value.get("targetUri"))
        .and_then(Value::as_str);
    let range = value
        .get("range")
        .or_else(|| value.get("targetSelectionRange"))
        .or_else(|| value.get("targetRange"));
    let (Some(uri), Some(range)) = (uri, range) else {
        return;
    };
    let Some(path) = uri_to_workspace_path(workspace, uri) else {
        return;
    };
    let Some((line, end_line)) = range_lines(range) else {
        return;
    };
    out.push(RustAnalyzerLspLocation {
        role,
        path,
        line,
        end_line,
        symbol: None,
    });
}

fn parse_document_symbols(out: &mut Vec<RustAnalyzerLspLocation>, path: &str, value: &Value) {
    let Some(items) = value.as_array() else {
        return;
    };
    for item in items {
        parse_document_symbol(out, path, item);
        if out.len() >= MAX_LOCATIONS {
            break;
        }
    }
}

fn parse_document_symbol(out: &mut Vec<RustAnalyzerLspLocation>, path: &str, item: &Value) {
    let range = item
        .get("selectionRange")
        .or_else(|| item.get("range"))
        .or_else(|| {
            item.get("location")
                .and_then(|location| location.get("range"))
        });
    if let Some((line, end_line)) = range.and_then(range_lines) {
        out.push(RustAnalyzerLspLocation {
            role: RustAnalyzerLspLocationRole::DocumentSymbol,
            path: path.to_string(),
            line,
            end_line,
            symbol: item
                .get("name")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        });
    }
    if let Some(children) = item.get("children").and_then(Value::as_array) {
        for child in children {
            if out.len() >= MAX_LOCATIONS {
                break;
            }
            parse_document_symbol(out, path, child);
        }
    }
}

fn parse_call_hierarchy_calls(
    out: &mut Vec<RustAnalyzerLspLocation>,
    workspace: &Path,
    value: &Value,
    item_key: &str,
    role: RustAnalyzerLspLocationRole,
) {
    let Some(items) = value.as_array() else {
        return;
    };
    for item in items {
        if let Some(call_item) = item.get(item_key) {
            parse_call_hierarchy_items(out, workspace, call_item, role, None);
        }
        if out.len() >= MAX_LOCATIONS {
            break;
        }
    }
}

fn parse_call_hierarchy_items(
    out: &mut Vec<RustAnalyzerLspLocation>,
    workspace: &Path,
    value: &Value,
    role: RustAnalyzerLspLocationRole,
    fallback_symbol: Option<&str>,
) {
    if let Some(items) = value.as_array() {
        for item in items {
            parse_call_hierarchy_items(out, workspace, item, role, fallback_symbol);
            if out.len() >= MAX_LOCATIONS {
                break;
            }
        }
        return;
    }

    let Some(uri) = value.get("uri").and_then(Value::as_str) else {
        return;
    };
    let Some(path) = uri_to_workspace_path(workspace, uri) else {
        return;
    };
    let range = value
        .get("selectionRange")
        .or_else(|| value.get("range"))
        .or_else(|| value.get("targetSelectionRange"));
    let Some((line, end_line)) = range.and_then(range_lines) else {
        return;
    };
    out.push(RustAnalyzerLspLocation {
        role,
        path,
        line,
        end_line,
        symbol: value
            .get("name")
            .and_then(Value::as_str)
            .or(fallback_symbol)
            .map(ToString::to_string),
    });
}

fn range_lines(range: &Value) -> Option<(usize, Option<usize>)> {
    let start = range.get("start")?;
    let line = start.get("line").and_then(Value::as_u64)? as usize + 1;
    let end_line = range
        .get("end")
        .and_then(|end| end.get("line"))
        .and_then(Value::as_u64)
        .map(|line| line as usize + 1)
        .filter(|end| *end != line);
    Some((line, end_line))
}

pub(super) fn uri_to_workspace_path(workspace: &Path, uri: &str) -> Option<String> {
    let raw = uri.strip_prefix("file://")?;
    let decoded = percent_decode(raw)?;
    let path_text = trim_windows_uri_slash(&decoded);
    let path = PathBuf::from(path_text);
    Some(display_workspace_path(workspace, &path))
}

fn trim_windows_uri_slash(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 4 && bytes[0] == b'/' && bytes[2] == b':' {
        &value[1..]
    } else {
        value
    }
}

fn display_workspace_path(workspace: &Path, path: &Path) -> String {
    let workspace_text = normalize_slashes(&workspace.to_string_lossy());
    let path_text = normalize_slashes(&path.to_string_lossy());
    let root = workspace_text.trim_end_matches('/');
    if path_text == root {
        return ".".to_string();
    }
    path_text
        .strip_prefix(root)
        .and_then(|value| value.strip_prefix('/'))
        .map(ToString::to_string)
        .unwrap_or(path_text)
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            output.push(from_hex_pair(high, low)?);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn from_hex_pair(high: u8, low: u8) -> Option<u8> {
    Some(hex_value(high)? * 16 + hex_value(low)?)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn normalize_slashes(value: &str) -> String {
    value.replace('\\', "/")
}

fn dedupe_locations(locations: Vec<RustAnalyzerLspLocation>) -> Vec<RustAnalyzerLspLocation> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for location in locations {
        let key = format!(
            "{}:{}:{}:{}",
            location.role.as_str(),
            location.path,
            location.line,
            location.symbol.as_deref().unwrap_or("")
        );
        if seen.insert(key) {
            deduped.push(location);
            if deduped.len() >= MAX_LOCATIONS {
                break;
            }
        }
    }
    deduped
}
