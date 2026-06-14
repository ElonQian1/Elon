use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{admin, types::AppState};

use super::{
    symbol_index_graph_query::{
        load_latest_symbol_graph, SymbolGraphQuery, SymbolRelationDirection,
    },
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{search_latest_symbol_index, SymbolIndexSearch},
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
        let symbol_id = clean(self.id).or_else(|| clean(self.symbol_id));
        let path = clean(self.path);
        if symbol_id.is_none() && path.is_none() {
            return Err("id 和 path 至少提供一个".to_string());
        }
        Ok(SymbolImpactQuery {
            trace_id: clean(self.trace_id),
            symbol_id,
            path,
            edge_kind: clean(self.edge_kind),
            depth: self.depth.unwrap_or_default(),
            limit: self.limit.unwrap_or_default(),
        })
    }
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

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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
