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

use super::super::{
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
    symbol_index_graph_query::{
        load_latest_symbol_graph, SymbolGraphQuery, SymbolRelationDirection,
    },
    symbol_index_impact_pack::{build_symbol_impact_pack, normalize_pack_max_chars},
    symbol_index_impact_query::load_latest_symbol_impact,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_query::{search_latest_symbol_index, SymbolIndexSearch},
    symbol_index_retrieval_learning::build_latest_symbol_retrieval_learning_report,
    symbol_index_retrieval_learning_types::SymbolRetrievalLearningQuery,
    symbol_index_task_pack::{build_latest_symbol_task_pack, SymbolTaskPackQuery},
    symbol_index_vector::{
        backfill_latest_symbol_vectors, search_latest_symbol_vectors, SymbolVectorBackfill,
        SymbolVectorSearchQuery,
    },
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
    #[serde(alias = "vectorModel")]
    pub(crate) vector_model: Option<String>,
    #[serde(alias = "vectorLimit")]
    pub(crate) vector_limit: Option<usize>,
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
pub(crate) struct SymbolEmbeddingStatusParams {
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolVectorBackfillBody {
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) limit: Option<usize>,
    pub(crate) force: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolVectorSearchParams {
    pub(crate) q: Option<String>,
    pub(crate) query: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) model: Option<String>,
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
    #[serde(alias = "vectorModel")]
    pub(crate) vector_model: Option<String>,
    #[serde(alias = "vectorLimit")]
    pub(crate) vector_limit: Option<usize>,
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
    #[serde(alias = "vectorModel")]
    pub(crate) vector_model: Option<String>,
    #[serde(alias = "vectorLimit")]
    pub(crate) vector_limit: Option<usize>,
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
    #[serde(alias = "vectorModel")]
    pub(crate) vector_model: Option<String>,
    #[serde(alias = "vectorLimit")]
    pub(crate) vector_limit: Option<usize>,
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

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolRetrievalLearningParams {
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) limit: Option<usize>,
    #[serde(alias = "minSamples", alias = "min_samples")]
    pub(crate) min_samples: Option<usize>,
    #[serde(alias = "topK", alias = "top_k")]
    pub(crate) top_k: Option<usize>,
}

impl SymbolIndexSearchParams {
    pub(super) fn into_search(self) -> SymbolIndexSearch {
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
    pub(super) fn into_query(self) -> Result<SymbolGraphQuery, String> {
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
    pub(super) fn into_query(self) -> Result<SymbolImpactQuery, String> {
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
    pub(super) fn into_query(self) -> Result<(SymbolImpactQuery, usize), String> {
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
    pub(super) fn into_query(self) -> Result<SymbolTaskPackQuery, String> {
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
            vector_model: clean(self.vector_model),
            vector_limit: self.vector_limit.unwrap_or_default(),
            impact_limit: self.impact_limit.unwrap_or_default(),
            max_chars: self.max_chars.unwrap_or_default(),
        })
    }
}

impl SymbolChunkSearchParams {
    pub(super) fn into_search(self) -> SymbolChunkSearch {
        SymbolChunkSearch {
            trace_id: clean(self.trace_id),
            text: clean(self.q).or_else(|| clean(self.query)),
            path: clean(self.path),
            chunk_type: clean(self.chunk_type),
            limit: self.limit.unwrap_or_default(),
        }
    }
}

impl SymbolEmbeddingStatusParams {
    pub(super) fn into_query(self) -> SymbolEmbeddingStatus {
        SymbolEmbeddingStatus {
            trace_id: clean(self.trace_id),
            model: clean(self.model),
            limit: self.limit.unwrap_or_default(),
        }
    }
}

impl SymbolVectorBackfillBody {
    pub(super) fn into_query(self) -> SymbolVectorBackfill {
        SymbolVectorBackfill {
            trace_id: clean(self.trace_id),
            model: clean(self.model),
            limit: self.limit.unwrap_or_default(),
            force: self.force.unwrap_or(false),
        }
    }
}

impl SymbolVectorSearchParams {
    pub(super) fn into_query(self) -> Result<SymbolVectorSearchQuery, String> {
        let text = clean(self.q).or_else(|| clean(self.query));
        if text.is_none() {
            return Err("q 不能为空".to_string());
        }
        Ok(SymbolVectorSearchQuery {
            trace_id: clean(self.trace_id),
            text,
            model: clean(self.model),
            path: clean(self.path),
            chunk_type: clean(self.chunk_type),
            limit: self.limit.unwrap_or_default(),
        })
    }
}

impl SymbolRetrievalEvalParams {
    pub(super) fn into_query(self) -> Result<RetrievalEvalQuery, String> {
        let text = clean(self.q).or_else(|| clean(self.query));
        if text.is_none() {
            return Err("q 不能为空".to_string());
        }
        Ok(RetrievalEvalQuery {
            trace_id: clean(self.trace_id),
            text,
            must_include: split_must_include(self.must_include.as_deref()),
            vector_model: clean(self.vector_model),
            k: self.k.unwrap_or_default(),
            symbol_limit: self.symbol_limit.unwrap_or_default(),
            chunk_limit: self.chunk_limit.unwrap_or_default(),
            vector_limit: self.vector_limit.unwrap_or_default(),
            depth: self.depth.unwrap_or_default(),
            impact_limit: self.impact_limit.unwrap_or_default(),
        })
    }
}

impl SymbolRetrievalEvalBatchBody {
    pub(super) fn into_query(self) -> Result<SymbolRetrievalEvalBatchQuery, String> {
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
        let batch_vector_model = clean(self.vector_model);
        let batch_vector_limit = self.vector_limit.unwrap_or_default();
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
                        vector_model: clean(case.vector_model)
                            .or_else(|| batch_vector_model.clone()),
                        k: case.k.unwrap_or(batch_k),
                        symbol_limit: case.symbol_limit.unwrap_or(batch_symbol_limit),
                        chunk_limit: case.chunk_limit.unwrap_or(batch_chunk_limit),
                        vector_limit: case.vector_limit.unwrap_or(batch_vector_limit),
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
    pub(super) fn into_query(self) -> SymbolRetrievalRunHistoryQuery {
        SymbolRetrievalRunHistoryQuery {
            trace_id: clean(self.trace_id),
            limit: self.limit.unwrap_or_default(),
        }
    }
}

impl SymbolRetrievalRunParams {
    pub(super) fn into_query(self) -> Result<SymbolRetrievalRunLookupQuery, String> {
        let id = clean(self.id)
            .or_else(|| clean(self.run_id))
            .ok_or_else(|| "id 不能为空".to_string())?;
        Ok(SymbolRetrievalRunLookupQuery {
            trace_id: clean(self.trace_id),
            id,
        })
    }
}

impl SymbolRetrievalLearningParams {
    pub(super) fn into_query(self) -> SymbolRetrievalLearningQuery {
        SymbolRetrievalLearningQuery {
            trace_id: clean(self.trace_id),
            limit: self.limit.unwrap_or_default(),
            min_samples: self.min_samples.unwrap_or_default(),
            top_k: self.top_k.unwrap_or_default(),
        }
    }
}

pub(super) struct ImpactQueryParts {
    id: Option<String>,
    symbol_id: Option<String>,
    trace_id: Option<String>,
    path: Option<String>,
    edge_kind: Option<String>,
    depth: Option<usize>,
    limit: Option<usize>,
}

pub(super) fn build_impact_query(parts: ImpactQueryParts) -> Result<SymbolImpactQuery, String> {
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


use super::query_helpers::*;
