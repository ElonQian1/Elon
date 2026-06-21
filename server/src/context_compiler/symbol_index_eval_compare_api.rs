use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{admin, types::AppState};

use super::symbol_index_eval_compare::{
    compare_latest_retrieval_runs, SymbolRetrievalRunCompareQuery,
};

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolRetrievalRunCompareParams {
    pub(crate) baseline: Option<String>,
    #[serde(alias = "baselineId", alias = "baselineRunId")]
    pub(crate) baseline_id: Option<String>,
    pub(crate) current: Option<String>,
    #[serde(alias = "currentId", alias = "currentRunId")]
    pub(crate) current_id: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    #[serde(alias = "caseLimit")]
    pub(crate) case_limit: Option<usize>,
}

impl SymbolRetrievalRunCompareParams {
    fn into_query(self) -> Result<SymbolRetrievalRunCompareQuery, String> {
        let baseline_id = clean(self.baseline)
            .or_else(|| clean(self.baseline_id))
            .ok_or_else(|| "baselineId 不能为空".to_string())?;
        let current_id = clean(self.current)
            .or_else(|| clean(self.current_id))
            .ok_or_else(|| "currentId 不能为空".to_string())?;
        Ok(SymbolRetrievalRunCompareQuery {
            trace_id: clean(self.trace_id),
            baseline_id,
            current_id,
            case_limit: self.case_limit.unwrap_or_default(),
        })
    }
}

pub(crate) async fn compare_symbol_retrieval_runs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SymbolRetrievalRunCompareParams>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let query = match params.into_query() {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match compare_latest_retrieval_runs(&state.data_dir, &query) {
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
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}
