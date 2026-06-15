use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{admin, types::AppState};

use super::{
    symbol_index_chunks::{search_latest_symbol_chunks, SymbolChunkSearch},
    symbol_index_eval::{evaluate_latest_symbol_retrieval, RetrievalEvalQuery},
    symbol_index_eval_runs::{
        evaluate_latest_symbol_retrieval_batch, list_latest_retrieval_runs,
        load_latest_retrieval_run,
    },
    symbol_index_eval_types::{
        SymbolRetrievalEvalBatchCaseQuery, SymbolRetrievalEvalBatchQuery,
        SymbolRetrievalRunHistoryQuery, SymbolRetrievalRunLookupQuery,
    },
    symbol_index_graph_query::{
        load_latest_symbol_graph, SymbolGraphQuery, SymbolRelationDirection,
    },
    symbol_index_impact_pack::{build_symbol_impact_pack, normalize_pack_max_chars},
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{search_latest_symbol_index, SymbolIndexSearch},
    symbol_index_task_pack::{build_latest_symbol_task_pack, SymbolTaskPackQuery},
};

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolIndexSearchParams {
    pub(crate) q: Option<String>,
    pub(crate) query: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    #[serde(alias = "edgeKind")]
    pub(crate) edge_kind: Option<String>,
    #[serde(alias = "includeEdges")]
    pub(crate) include_edges: Option<bool>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolGraphParams {
    pub(crate) id: Option<String>,
    #[serde(alias = "symbolId")]
    pub(crate) symbol_id: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    #[serde(alias = "edgeKind")]
    pub(crate) edge_kind: Option<String>,
    pub(crate) direction: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolImpactParams {
    pub(crate) id: Option<String>,
    #[serde(alias = "symbolId")]
    pub(crate) symbol_id: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) path: Option<String>,
    #[serde(alias = "edgeKind")]
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: Option<usize>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolImpactPackParams {
    pub(crate) id: Option<String>,
    #[serde(alias = "symbolId")]
    pub(crate) symbol_id: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) path: Option<String>,
    #[serde(alias = "edgeKind")]
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: Option<usize>,
    pub(crate) limit: Option<usize>,
    #[serde(alias = "maxChars")]
    pub(crate) max_chars: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolTaskPackParams {
    pub(crate) q: Option<String>,
    pub(crate) query: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    #[serde(alias = "edgeKind")]
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: Option<usize>,
    #[serde(alias = "searchLimit")]
    pub(crate) search_limit: Option<usize>,
    #[serde(alias = "chunkLimit")]
    pub(crate) chunk_limit: Option<usize>,
    #[serde(alias = "impactLimit")]
    pub(crate) impact_limit: Option<usize>,
    #[serde(alias = "maxChars")]
    pub(crate) max_chars: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolChunkSearchParams {
    pub(crate) q: Option<String>,
    pub(crate) query: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) path: Option<String>,
    #[serde(alias = "chunkType")]
    pub(crate) chunk_type: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolRetrievalEvalParams {
    pub(crate) q: Option<String>,
    pub(crate) query: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    #[serde(alias = "mustInclude", alias = "must_include")]
    pub(crate) must_include: Option<String>,
    pub(crate) k: Option<usize>,
    #[serde(alias = "symbolLimit")]
    pub(crate) symbol_limit: Option<usize>,
    #[serde(alias = "chunkLimit")]
    pub(crate) chunk_limit: Option<usize>,
    pub(crate) depth: Option<usize>,
    #[serde(alias = "impactLimit")]
    pub(crate) impact_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolRetrievalEvalBatchBody {
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    #[serde(default)]
    pub(crate) cases: Vec<SymbolRetrievalEvalCaseBody>,
    #[serde(alias = "recordRuns")]
    pub(crate) record_runs: Option<bool>,
    pub(crate) k: Option<usize>,
    #[serde(alias = "symbolLimit")]
    pub(crate) symbol_limit: Option<usize>,
    #[serde(alias = "chunkLimit")]
    pub(crate) chunk_limit: Option<usize>,
    pub(crate) depth: Option<usize>,
    #[serde(alias = "impactLimit")]
    pub(crate) impact_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolRetrievalEvalCaseBody {
    pub(crate) id: Option<String>,
    pub(crate) q: Option<String>,
    pub(crate) query: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    #[serde(default, alias = "mustInclude", alias = "must_include")]
    pub(crate) must_include: Value,
    pub(crate) k: Option<usize>,
    #[serde(alias = "symbolLimit")]
    pub(crate) symbol_limit: Option<usize>,
    #[serde(alias = "chunkLimit")]
    pub(crate) chunk_limit: Option<usize>,
    pub(crate) depth: Option<usize>,
    #[serde(alias = "impactLimit")]
    pub(crate) impact_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolRetrievalRunsParams {
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolRetrievalRunParams {
    pub(crate) id: Option<String>,
    #[serde(alias = "runId")]
    pub(crate) run_id: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
}

impl SymbolIndexSearchParams {
    fn into_search(self) -> SymbolIndexSearch {
        SymbolIndexSearch {
            trace_id: clean(self.trace_id),
            text: clean(self.q).or_else(|| clean(self.query)),
            kind: clean(self.kind),
            path: clean(self.path),
            edge_kind: clean(self.edge_kind),
            include_edges: self.include_edges.unwrap_or(false),
            limit: self.limit.unwrap_or_default(),
        }
    }
}

impl SymbolGraphParams {
    fn into_query(self) -> Result<SymbolGraphQuery, String> {
        let symbol_id = clean(self.id)
            .or_else(|| clean(self.symbol_id))
            .ok_or_else(|| "id 不能为空".to_string())?;
        Ok(SymbolGraphQuery {
            trace_id: clean(self.trace_id),
            symbol_id,
            edge_kind: clean(self.edge_kind),
            direction: SymbolRelationDirection::from_query_value(self.direction.as_deref()),
            limit: self.limit.unwrap_or_default(),
        })
    }
}

impl SymbolImpactParams {
    fn into_query(self) -> Result<SymbolImpactQuery, String> {
        build_impact_query(ImpactQueryParts {
            id: self.id,
            symbol_id: self.symbol_id,
            trace_id: self.trace_id,
            path: self.path,
            edge_kind: self.edge_kind,
            depth: self.depth,
            limit: self.limit,
        })
    }
}

impl SymbolImpactPackParams {
    fn into_query(self) -> Result<(SymbolImpactQuery, usize), String> {
        let query = build_impact_query(ImpactQueryParts {
            id: self.id,
            symbol_id: self.symbol_id,
            trace_id: self.trace_id,
            path: self.path,
            edge_kind: self.edge_kind,
            depth: self.depth,
            limit: self.limit,
        })?;
        Ok((
            query,
            normalize_pack_max_chars(self.max_chars.unwrap_or_default()),
        ))
    }
}

impl SymbolTaskPackParams {
    fn into_query(self) -> Result<SymbolTaskPackQuery, String> {
        let text = clean(self.q).or_else(|| clean(self.query));
        if text.is_none() {
            return Err("q 不能为空".to_string());
        }
        Ok(SymbolTaskPackQuery {
            trace_id: clean(self.trace_id),
            text,
            kind: clean(self.kind),
            path: clean(self.path),
            edge_kind: clean(self.edge_kind),
            depth: self.depth.unwrap_or_default(),
            search_limit: self.search_limit.unwrap_or_default(),
            chunk_limit: self.chunk_limit.unwrap_or_default(),
            impact_limit: self.impact_limit.unwrap_or_default(),
            max_chars: self.max_chars.unwrap_or_default(),
        })
    }
}

impl SymbolChunkSearchParams {
    fn into_search(self) -> SymbolChunkSearch {
        SymbolChunkSearch {
            trace_id: clean(self.trace_id),
            text: clean(self.q).or_else(|| clean(self.query)),
            path: clean(self.path),
            chunk_type: clean(self.chunk_type),
            limit: self.limit.unwrap_or_default(),
        }
    }
}

impl SymbolRetrievalEvalParams {
    fn into_query(self) -> Result<RetrievalEvalQuery, String> {
        let text = clean(self.q).or_else(|| clean(self.query));
        if text.is_none() {
            return Err("q 不能为空".to_string());
        }
        Ok(RetrievalEvalQuery {
            trace_id: clean(self.trace_id),
            text,
            must_include: split_must_include(self.must_include.as_deref()),
            k: self.k.unwrap_or_default(),
            symbol_limit: self.symbol_limit.unwrap_or_default(),
            chunk_limit: self.chunk_limit.unwrap_or_default(),
            depth: self.depth.unwrap_or_default(),
            impact_limit: self.impact_limit.unwrap_or_default(),
        })
    }
}

impl SymbolRetrievalEvalBatchBody {
    fn into_query(self) -> Result<SymbolRetrievalEvalBatchQuery, String> {
        if self.cases.is_empty() {
            return Err("cases 不能为空".to_string());
        }
        if self.cases.len() > 200 {
            return Err("cases 最多支持 200 条".to_string());
        }

        let trace_id = clean(self.trace_id);
        let batch_k = self.k.unwrap_or_default();
        let batch_symbol_limit = self.symbol_limit.unwrap_or_default();
        let batch_chunk_limit = self.chunk_limit.unwrap_or_default();
        let batch_depth = self.depth.unwrap_or_default();
        let batch_impact_limit = self.impact_limit.unwrap_or_default();
        let cases = self
            .cases
            .into_iter()
            .enumerate()
            .map(|(index, case)| {
                let text = clean(case.q).or_else(|| clean(case.query));
                let Some(text) = text else {
                    return Err(format!("cases[{}].q 不能为空", index));
                };
                Ok(SymbolRetrievalEvalBatchCaseQuery {
                    id: clean(case.id).unwrap_or_else(|| format!("case-{}", index + 1)),
                    query: RetrievalEvalQuery {
                        trace_id: clean(case.trace_id).or_else(|| trace_id.clone()),
                        text: Some(text),
                        must_include: parse_must_include_value(&case.must_include),
                        k: case.k.unwrap_or(batch_k),
                        symbol_limit: case.symbol_limit.unwrap_or(batch_symbol_limit),
                        chunk_limit: case.chunk_limit.unwrap_or(batch_chunk_limit),
                        depth: case.depth.unwrap_or(batch_depth),
                        impact_limit: case.impact_limit.unwrap_or(batch_impact_limit),
                    },
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(SymbolRetrievalEvalBatchQuery {
            trace_id,
            cases,
            record_runs: self.record_runs.unwrap_or(true),
        })
    }
}

impl SymbolRetrievalRunsParams {
    fn into_query(self) -> SymbolRetrievalRunHistoryQuery {
        SymbolRetrievalRunHistoryQuery {
            trace_id: clean(self.trace_id),
            limit: self.limit.unwrap_or_default(),
        }
    }
}

impl SymbolRetrievalRunParams {
    fn into_query(self) -> Result<SymbolRetrievalRunLookupQuery, String> {
        let id = clean(self.id)
            .or_else(|| clean(self.run_id))
            .ok_or_else(|| "id 不能为空".to_string())?;
        Ok(SymbolRetrievalRunLookupQuery {
            trace_id: clean(self.trace_id),
            id,
        })
    }
}

struct ImpactQueryParts {
    id: Option<String>,
    symbol_id: Option<String>,
    trace_id: Option<String>,
    path: Option<String>,
    edge_kind: Option<String>,
    depth: Option<usize>,
    limit: Option<usize>,
}

fn build_impact_query(parts: ImpactQueryParts) -> Result<SymbolImpactQuery, String> {
    let symbol_id = clean(parts.id).or_else(|| clean(parts.symbol_id));
    let path = clean(parts.path);
    if symbol_id.is_none() && path.is_none() {
        return Err("id 和 path 至少提供一个".to_string());
    }
    Ok(SymbolImpactQuery {
        trace_id: clean(parts.trace_id),
        symbol_id,
        path,
        edge_kind: clean(parts.edge_kind),
        depth: parts.depth.unwrap_or_default(),
        limit: parts.limit.unwrap_or_default(),
    })
}

pub(crate) async fn search_symbol_index(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolIndexSearchParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    match search_latest_symbol_index(&state.data_dir, &params.into_search()) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn get_symbol_graph(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolGraphParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match params.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match load_latest_symbol_graph(&state.data_dir, &query) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn get_symbol_impact(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolImpactParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match params.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match load_latest_symbol_impact(&state.data_dir, &query) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn get_symbol_impact_pack(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolImpactPackParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let (query, max_chars) = match params.into_query() {
        Ok(value) => value,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match load_latest_symbol_impact(&state.data_dir, &query) {
        Ok(response) => Json(build_symbol_impact_pack(response, max_chars)).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn get_symbol_task_pack(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolTaskPackParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match params.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match build_latest_symbol_task_pack(&state.data_dir, &query) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn search_symbol_chunks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolChunkSearchParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    match search_latest_symbol_chunks(&state.data_dir, &params.into_search()) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn eval_symbol_retrieval(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolRetrievalEvalParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match params.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match evaluate_latest_symbol_retrieval(&state.data_dir, &query) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn eval_symbol_retrieval_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolRetrievalEvalBatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match body.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match evaluate_latest_symbol_retrieval_batch(&state.data_dir, &query) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn list_symbol_retrieval_runs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolRetrievalRunsParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    match list_latest_retrieval_runs(&state.data_dir, &params.into_query()) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn get_symbol_retrieval_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolRetrievalRunParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match params.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match load_latest_retrieval_run(&state.data_dir, &query) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_must_include_value(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => split_must_include(Some(text)),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str())
            .flat_map(|text| split_must_include(Some(text)))
            .collect(),
        _ => Vec::new(),
    }
}

fn split_must_include(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(|ch| ch == ',' || ch == ';' || ch == '\n' || ch == '\r')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message,
        })),
    )
        .into_response()
}
