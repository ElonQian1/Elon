use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

use crate::{admin, types::AppState};

use super::{
    symbol_index_chunks::{search_latest_symbol_chunks, SymbolChunkSearch},
    symbol_index_embeddings::{load_latest_symbol_embedding_status, SymbolEmbeddingStatus},
    symbol_index_eval::{evaluate_latest_symbol_retrieval, RetrievalEvalQuery},
    symbol_index_eval_runs::{
        evaluate_latest_symbol_retrieval_batch, list_latest_retrieval_runs,
        load_latest_retrieval_run,
    },
    symbol_index_eval_types::{
        SymbolRetrievalEvalBatchCaseQuery, SymbolRetrievalEvalBatchQuery,
        SymbolRetrievalRunHistoryQuery, SymbolRetrievalRunLookupQuery,
    },
    symbol_index_graph_query::{load_latest_symbol_graph, SymbolGraphQuery},
    symbol_index_impact_pack::build_symbol_impact_pack,
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_query::search_latest_symbol_index,
    symbol_index_retrieval_learning::build_latest_symbol_retrieval_learning_report,
    symbol_index_task_pack::build_latest_symbol_task_pack,
    symbol_index_vector::{backfill_latest_symbol_vectors, search_latest_symbol_vectors},
};

mod params;
pub(crate) use self::params::*;
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

pub(crate) async fn get_symbol_embedding_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolEmbeddingStatusParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    match load_latest_symbol_embedding_status(&state.data_dir, &params.into_query()) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn backfill_symbol_vectors(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolVectorBackfillBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    match backfill_latest_symbol_vectors(&state.data_dir, &body.into_query()) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn search_symbol_vectors(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolVectorSearchParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match params.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match search_latest_symbol_vectors(&state.data_dir, &query) {
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

pub(crate) async fn get_symbol_retrieval_learning(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolRetrievalLearningParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    match build_latest_symbol_retrieval_learning_report(&state.data_dir, &params.into_query()) {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}


mod query_helpers;

use self::query_helpers::*;
