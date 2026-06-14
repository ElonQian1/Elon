use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde_json::{json, Value};

use super::{
    model::{
        RepoContextIndex, RustAnalyzerLspQueryResult, RustAnalyzerLspReport, RustAnalyzerLspStatus,
        SemanticQuery, SemanticQueryMethod, SemanticQueryProvider,
    },
    rust_analyzer_lsp_locations::parse_lsp_locations,
    rust_analyzer_lsp_protocol::{LspRequestStatus, RustAnalyzerLspClient},
};

pub(crate) fn execute_semantic_query_plan(
    workspace: &Path,
    index: &RepoContextIndex,
    enabled: bool,
    timeout_ms: usize,
    max_queries: usize,
) -> RustAnalyzerLspReport {
    if !enabled {
        return RustAnalyzerLspReport::default();
    }

    let Some(root) = lsp_root(workspace, index) else {
        return RustAnalyzerLspReport {
            enabled: true,
            warnings: vec![
                "rust-analyzer LSP skipped: no Cargo.toml workspace root was found".to_string(),
            ],
            ..RustAnalyzerLspReport::default()
        };
    };

    let timeout = Duration::from_millis(timeout_ms as u64);
    let mut report = RustAnalyzerLspReport {
        enabled: true,
        workspace_path: Some(display_path(workspace, &root)),
        ..RustAnalyzerLspReport::default()
    };
    if !index.rust_analyzer.available {
        report.warnings.push(
            "rust-analyzer availability probe failed; LSP execution may be skipped or fail"
                .to_string(),
        );
    }

    let root_uri = file_uri(&root);
    let root_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace");
    let mut client = match RustAnalyzerLspClient::start(&root) {
        Ok(client) => client,
        Err(error) => {
            report.warnings.push(error);
            return report;
        }
    };

    match client.initialize(&root_uri, root_name, timeout) {
        LspRequestStatus::Succeeded(_) => {}
        LspRequestStatus::Failed(error) => {
            report
                .warnings
                .push(format!("rust-analyzer initialize failed: {error}"));
            return report;
        }
        LspRequestStatus::TimedOut => {
            report.warnings.push(format!(
                "rust-analyzer initialize timed out after {}ms",
                timeout.as_millis()
            ));
            return report;
        }
    }

    let query_limit = max_queries.max(1).min(64);
    for query in index
        .semantic_plan
        .queries
        .iter()
        .filter(|query| query.provider == SemanticQueryProvider::RustAnalyzerLsp)
        .filter(|query| is_executable_method(query.method))
        .take(query_limit)
    {
        push_result(
            &mut report,
            execute_query(workspace, &mut client, query, timeout),
        );
    }

    for query in index
        .semantic_plan
        .queries
        .iter()
        .filter(|query| query.provider == SemanticQueryProvider::RustAnalyzerLsp)
        .filter(|query| !is_executable_method(query.method))
        .take(8)
    {
        push_result(&mut report, skipped_result(query, "LSP diagnostics are collected by rust-analyzer probes; direct diagnostic pull is not executed in the Top-K LSP runner"));
    }

    client.shutdown(timeout.min(Duration::from_millis(1_000)));
    for line in client.stderr_excerpt() {
        report
            .warnings
            .push(format!("rust-analyzer stderr: {line}"));
    }
    report
}

fn execute_query(
    workspace: &Path,
    client: &mut RustAnalyzerLspClient,
    query: &SemanticQuery,
    timeout: Duration,
) -> RustAnalyzerLspQueryResult {
    let started = Instant::now();
    if query.method == SemanticQueryMethod::WorkspaceSymbol {
        let query_text = query
            .symbol
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_default();
        let status = client.request(
            query.method.as_lsp_method(),
            json!({ "query": query_text }),
            timeout,
        );
        return query_result_from_status(workspace, query, started.elapsed(), status);
    }

    let params = match query_position_params(workspace, query) {
        Ok(params) => params,
        Err(warning) => return skipped_result_with_duration(query, started.elapsed(), &warning),
    };

    match query.method {
        SemanticQueryMethod::IncomingCalls | SemanticQueryMethod::OutgoingCalls => {
            execute_call_hierarchy_query(workspace, client, query, params, timeout, started)
        }
        method => {
            let status = client.request(method.as_lsp_method(), params, timeout);
            query_result_from_status(workspace, query, started.elapsed(), status)
        }
    }
}

fn execute_call_hierarchy_query(
    workspace: &Path,
    client: &mut RustAnalyzerLspClient,
    query: &SemanticQuery,
    prepare_params: Value,
    timeout: Duration,
    started: Instant,
) -> RustAnalyzerLspQueryResult {
    let prepare = client.request(
        SemanticQueryMethod::PrepareCallHierarchy.as_lsp_method(),
        prepare_params,
        timeout,
    );
    let LspRequestStatus::Succeeded(result) = prepare else {
        return query_result_from_status(workspace, query, started.elapsed(), prepare);
    };
    let Some(item) = first_call_hierarchy_item(&result) else {
        return RustAnalyzerLspQueryResult {
            method: query.method,
            path: query.path.clone(),
            line: query.line,
            symbol: query.symbol.clone(),
            status: RustAnalyzerLspStatus::Skipped,
            duration_ms: elapsed_ms(started.elapsed()),
            summary: Some("prepareCallHierarchy returned no item".to_string()),
            locations: Vec::new(),
            warning: None,
        };
    };

    let status = client.request(
        query.method.as_lsp_method(),
        json!({ "item": item }),
        timeout,
    );
    query_result_from_status(workspace, query, started.elapsed(), status)
}

fn query_position_params(workspace: &Path, query: &SemanticQuery) -> Result<Value, String> {
    let file = workspace.join(&query.path);
    if !file.is_file() {
        return Err(format!("source file does not exist: {}", query.path));
    }
    let text = fs::read_to_string(&file)
        .map_err(|error| format!("failed to read {}: {error}", query.path))?;
    let line = query.line.max(1);
    let character = find_symbol_character(&text, line, query.symbol.as_deref());
    let uri = file_uri(&file);
    let text_document = json!({ "uri": uri });
    let position = json!({
        "line": line.saturating_sub(1),
        "character": character,
    });

    Ok(match query.method {
        SemanticQueryMethod::DocumentSymbol => {
            json!({ "textDocument": text_document })
        }
        SemanticQueryMethod::References => {
            json!({
                "textDocument": text_document,
                "position": position,
                "context": { "includeDeclaration": true }
            })
        }
        _ => {
            json!({
                "textDocument": text_document,
                "position": position,
            })
        }
    })
}

fn query_result_from_status(
    workspace: &Path,
    query: &SemanticQuery,
    elapsed: Duration,
    status: LspRequestStatus,
) -> RustAnalyzerLspQueryResult {
    match status {
        LspRequestStatus::Succeeded(result) => RustAnalyzerLspQueryResult {
            method: query.method,
            path: query.path.clone(),
            line: query.line,
            symbol: query.symbol.clone(),
            status: RustAnalyzerLspStatus::Succeeded,
            duration_ms: elapsed_ms(elapsed),
            summary: Some(summarize_lsp_result(query.method, &result)),
            locations: parse_lsp_locations(workspace, query, &result),
            warning: None,
        },
        LspRequestStatus::Failed(error) => RustAnalyzerLspQueryResult {
            method: query.method,
            path: query.path.clone(),
            line: query.line,
            symbol: query.symbol.clone(),
            status: RustAnalyzerLspStatus::Failed,
            duration_ms: elapsed_ms(elapsed),
            summary: None,
            locations: Vec::new(),
            warning: Some(error),
        },
        LspRequestStatus::TimedOut => RustAnalyzerLspQueryResult {
            method: query.method,
            path: query.path.clone(),
            line: query.line,
            symbol: query.symbol.clone(),
            status: RustAnalyzerLspStatus::TimedOut,
            duration_ms: elapsed_ms(elapsed),
            summary: None,
            locations: Vec::new(),
            warning: Some("rust-analyzer LSP request timed out".to_string()),
        },
    }
}

fn skipped_result(query: &SemanticQuery, warning: &str) -> RustAnalyzerLspQueryResult {
    skipped_result_with_duration(query, Duration::ZERO, warning)
}

fn skipped_result_with_duration(
    query: &SemanticQuery,
    elapsed: Duration,
    warning: &str,
) -> RustAnalyzerLspQueryResult {
    RustAnalyzerLspQueryResult {
        method: query.method,
        path: query.path.clone(),
        line: query.line,
        symbol: query.symbol.clone(),
        status: RustAnalyzerLspStatus::Skipped,
        duration_ms: elapsed_ms(elapsed),
        summary: None,
        locations: Vec::new(),
        warning: Some(warning.to_string()),
    }
}

fn push_result(report: &mut RustAnalyzerLspReport, result: RustAnalyzerLspQueryResult) {
    match result.status {
        RustAnalyzerLspStatus::Succeeded => {
            report.succeeded += 1;
            report.attempted += 1;
        }
        RustAnalyzerLspStatus::Failed => {
            report.failed += 1;
            report.attempted += 1;
        }
        RustAnalyzerLspStatus::TimedOut => {
            report.timed_out += 1;
            report.attempted += 1;
        }
        RustAnalyzerLspStatus::Skipped => report.skipped += 1,
    }
    report.results.push(result);
}

fn lsp_root(workspace: &Path, index: &RepoContextIndex) -> Option<PathBuf> {
    if let Some(root) = index
        .cargo
        .workspace_root
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| workspace.join(value))
        .filter(|path| path.join("Cargo.toml").is_file())
    {
        return Some(root);
    }
    if let Some(root) = index
        .cargo
        .manifest_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| workspace.join(value))
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .filter(|path| path.join("Cargo.toml").is_file())
    {
        return Some(root);
    }
    for candidate in [workspace.to_path_buf(), workspace.join("server")] {
        if candidate.join("Cargo.toml").is_file() {
            return Some(candidate);
        }
    }
    None
}

fn is_executable_method(method: SemanticQueryMethod) -> bool {
    !matches!(method, SemanticQueryMethod::Diagnostic)
}

fn first_call_hierarchy_item(value: &Value) -> Option<Value> {
    value
        .as_array()
        .and_then(|items| items.first())
        .cloned()
        .filter(Value::is_object)
}

pub(super) fn find_symbol_character(text: &str, line: usize, symbol: Option<&str>) -> usize {
    let Some(line_text) = text.lines().nth(line.saturating_sub(1)) else {
        return 0;
    };
    let Some(symbol) = symbol.filter(|value| !value.trim().is_empty()) else {
        return 0;
    };
    line_text
        .find(symbol)
        .map(|byte_index| line_text[..byte_index].chars().count())
        .unwrap_or(0)
}

pub(super) fn summarize_lsp_result(method: SemanticQueryMethod, value: &Value) -> String {
    if value.is_null() {
        return "no result".to_string();
    }
    if let Some(items) = value.as_array() {
        return format!("{} item(s)", items.len());
    }
    if method == SemanticQueryMethod::Hover {
        if let Some(contents) = value.get("contents") {
            return compact(&hover_contents_text(contents), 220);
        }
    }
    if let Some(uri) = value.get("uri").and_then(Value::as_str) {
        return format!("location {}", compact(uri, 180));
    }
    if let Some(object) = value.as_object() {
        let keys = object
            .keys()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        return format!("object fields: {keys}");
    }
    compact(&value.to_string(), 220)
}

fn hover_contents_text(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    if let Some(value) = value.get("value").and_then(Value::as_str) {
        return value.to_string();
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(hover_contents_text)
            .filter(|item| !item.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }
    value.to_string()
}

pub(super) fn file_uri(path: &Path) -> String {
    let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let raw = absolute.to_string_lossy().replace('\\', "/");
    if raw.starts_with("//") {
        return format!("file:{}", percent_encode_path(&raw));
    }
    format!("file:///{}", percent_encode_path(&raw))
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '/' | ':' | '-' | '_' | '.' | '~') {
            encoded.push(ch);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn display_path(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| ".".to_string())
}

fn elapsed_ms(elapsed: Duration) -> u64 {
    elapsed.as_millis().min(u128::from(u64::MAX)) as u64
}

fn compact(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = single_line.chars().take(max_chars).collect::<String>();
    if single_line.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
