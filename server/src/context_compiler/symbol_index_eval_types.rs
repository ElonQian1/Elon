use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

const DEFAULT_EVAL_K: usize = 10;
const MAX_EVAL_K: usize = 100;
const DEFAULT_EVAL_SYMBOL_LIMIT: usize = 20;
const MAX_EVAL_SYMBOL_LIMIT: usize = 100;
const DEFAULT_EVAL_CHUNK_LIMIT: usize = 30;
const MAX_EVAL_CHUNK_LIMIT: usize = 100;
const DEFAULT_EVAL_IMPACT_LIMIT: usize = 120;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolRetrievalEvalQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) must_include: Vec<String>,
    pub(crate) vector_model: Option<String>,
    pub(crate) k: usize,
    pub(crate) symbol_limit: usize,
    pub(crate) chunk_limit: usize,
    pub(crate) vector_limit: usize,
    pub(crate) depth: usize,
    pub(crate) impact_limit: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolRetrievalEvalBatchQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) cases: Vec<SymbolRetrievalEvalBatchCaseQuery>,
    pub(crate) record_runs: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolRetrievalEvalBatchCaseQuery {
    pub(crate) id: String,
    pub(crate) query: SymbolRetrievalEvalQuery,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolRetrievalRunHistoryQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolRetrievalRunLookupQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) id: String,
}

impl SymbolRetrievalEvalQuery {
    pub(crate) fn k(&self) -> usize {
        if self.k == 0 {
            DEFAULT_EVAL_K
        } else {
            self.k.min(MAX_EVAL_K)
        }
    }

    pub(crate) fn symbol_limit(&self) -> usize {
        if self.symbol_limit == 0 {
            DEFAULT_EVAL_SYMBOL_LIMIT
        } else {
            self.symbol_limit.min(MAX_EVAL_SYMBOL_LIMIT)
        }
    }

    pub(crate) fn chunk_limit(&self) -> usize {
        if self.chunk_limit == 0 {
            DEFAULT_EVAL_CHUNK_LIMIT
        } else {
            self.chunk_limit.min(MAX_EVAL_CHUNK_LIMIT)
        }
    }

    pub(crate) fn vector_limit(&self) -> usize {
        if self.vector_limit == 0 {
            DEFAULT_EVAL_CHUNK_LIMIT
        } else {
            self.vector_limit.min(MAX_EVAL_CHUNK_LIMIT)
        }
    }

    pub(crate) fn impact_limit(&self) -> usize {
        if self.impact_limit == 0 {
            DEFAULT_EVAL_IMPACT_LIMIT
        } else {
            self.impact_limit
        }
    }
}

impl SymbolRetrievalRunHistoryQuery {
    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            20
        } else {
            self.limit.min(100)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalRunsResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolRetrievalRunHistoryQueryEcho,
    pub(crate) runs: Vec<SymbolRetrievalRunSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalRunDetailResponse {
    pub(crate) db_path: String,
    pub(crate) run: SymbolRetrievalRunDetail,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalRunHistoryQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalRunSummary {
    pub(crate) id: String,
    pub(crate) query: String,
    pub(crate) scores: Value,
    pub(crate) created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalRunDetail {
    pub(crate) id: String,
    pub(crate) query: String,
    pub(crate) selected_chunks: Value,
    pub(crate) scores: Value,
    pub(crate) created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalEvalBatchResponse {
    pub(crate) run_id: String,
    pub(crate) trace_id: Option<String>,
    pub(crate) db_path: Option<String>,
    pub(crate) record_db_path: Option<String>,
    pub(crate) recorded: bool,
    pub(crate) record_error: Option<String>,
    pub(crate) case_count: usize,
    pub(crate) evaluated_count: usize,
    pub(crate) failed_count: usize,
    pub(crate) aggregate: SymbolRetrievalEvalBatchMetrics,
    pub(crate) cases: Vec<SymbolRetrievalEvalBatchCaseResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalEvalBatchMetrics {
    pub(crate) requirement_count: usize,
    pub(crate) hit_count_at_k: usize,
    pub(crate) missing_requirement_count: usize,
    pub(crate) mean_recall_at_k: f64,
    pub(crate) mean_reciprocal_rank: f64,
    pub(crate) has_test_context_rate: f64,
    pub(crate) total_token_count_at_k: usize,
    pub(crate) average_token_count_at_k: f64,
    pub(crate) candidate_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalEvalBatchCaseResponse {
    pub(crate) id: String,
    pub(crate) ok: bool,
    pub(crate) error: Option<String>,
    pub(crate) result: Option<SymbolRetrievalEvalResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalEvalResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolRetrievalEvalQueryEcho,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) metrics: SymbolRetrievalEvalMetrics,
    pub(crate) candidates: Vec<SymbolRetrievalEvalCandidate>,
    pub(crate) missing_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalEvalQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) q: String,
    pub(crate) must_include: Vec<String>,
    pub(crate) k: usize,
    pub(crate) symbol_limit: usize,
    pub(crate) chunk_limit: usize,
    pub(crate) vector_model: Option<String>,
    pub(crate) vector_limit: usize,
    pub(crate) depth: usize,
    pub(crate) impact_limit: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalEvalMetrics {
    pub(crate) requirement_count: usize,
    pub(crate) hit_count_at_k: usize,
    pub(crate) recall_at_k: f64,
    pub(crate) mean_reciprocal_rank: f64,
    pub(crate) first_relevant_rank: Option<usize>,
    pub(crate) top_k_candidate_count: usize,
    pub(crate) symbol_candidate_count: usize,
    pub(crate) chunk_candidate_count: usize,
    pub(crate) vector_candidate_count: usize,
    pub(crate) graph_candidate_count: usize,
    pub(crate) test_candidate_count_at_k: usize,
    pub(crate) total_token_count_at_k: usize,
    pub(crate) average_token_count_at_k: f64,
    pub(crate) has_test_context_at_k: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalEvalCandidate {
    pub(crate) rank: usize,
    pub(crate) source: String,
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
    pub(crate) score: f64,
    pub(crate) token_count: usize,
    pub(crate) matched_terms: Vec<String>,
    pub(crate) reasons: Vec<String>,
    pub(crate) matched_requirements: Vec<String>,
    pub(crate) is_test_context: bool,
}
