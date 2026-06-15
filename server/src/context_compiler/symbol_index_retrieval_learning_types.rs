use serde::Serialize;

const DEFAULT_LEARNING_RUN_LIMIT: usize = 50;
const MAX_LEARNING_RUN_LIMIT: usize = 500;
const DEFAULT_LEARNING_MIN_SAMPLES: usize = 3;
const MAX_LEARNING_MIN_SAMPLES: usize = 100;
const DEFAULT_LEARNING_TOP_K: usize = 10;
const MAX_LEARNING_TOP_K: usize = 100;

#[derive(Debug, Clone, Default)]
pub(crate) struct SymbolRetrievalLearningQuery {
    pub(crate) trace_id: Option<String>,
    pub(crate) limit: usize,
    pub(crate) min_samples: usize,
    pub(crate) top_k: usize,
}

impl SymbolRetrievalLearningQuery {
    pub(crate) fn limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_LEARNING_RUN_LIMIT
        } else {
            self.limit.min(MAX_LEARNING_RUN_LIMIT)
        }
    }

    pub(crate) fn min_samples(&self) -> usize {
        if self.min_samples == 0 {
            DEFAULT_LEARNING_MIN_SAMPLES
        } else {
            self.min_samples.min(MAX_LEARNING_MIN_SAMPLES)
        }
    }

    pub(crate) fn top_k(&self) -> usize {
        if self.top_k == 0 {
            DEFAULT_LEARNING_TOP_K
        } else {
            self.top_k.min(MAX_LEARNING_TOP_K)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalLearningResponse {
    pub(crate) db_path: String,
    pub(crate) query: SymbolRetrievalLearningQueryEcho,
    pub(crate) learning_status: String,
    pub(crate) run_count: usize,
    pub(crate) case_count: usize,
    pub(crate) evaluated_count: usize,
    pub(crate) candidate_count: usize,
    pub(crate) source_profiles: Vec<SymbolRetrievalSourceLearningProfile>,
    pub(crate) intent_profiles: Vec<SymbolRetrievalIntentLearningProfile>,
    pub(crate) recommended_weights: Vec<SymbolRetrievalPolicyRecommendation>,
    pub(crate) recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalLearningQueryEcho {
    pub(crate) trace_id: Option<String>,
    pub(crate) limit: usize,
    pub(crate) min_samples: usize,
    pub(crate) top_k: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalIntentLearningProfile {
    pub(crate) intent: String,
    pub(crate) evaluated_count: usize,
    pub(crate) candidate_count: usize,
    pub(crate) mean_recall_at_k: f64,
    pub(crate) mean_reciprocal_rank: f64,
    pub(crate) mean_noise_rate_at_k: f64,
    pub(crate) source_profiles: Vec<SymbolRetrievalSourceLearningProfile>,
    pub(crate) recommended_weights: Vec<SymbolRetrievalPolicyRecommendation>,
    pub(crate) recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalSourceLearningProfile {
    pub(crate) source: String,
    pub(crate) candidate_count: usize,
    pub(crate) hit_count: usize,
    pub(crate) noise_count: usize,
    pub(crate) precision_at_k: f64,
    pub(crate) noise_rate_at_k: f64,
    pub(crate) mean_rank: f64,
    pub(crate) total_token_count: usize,
    pub(crate) average_token_count: f64,
    pub(crate) confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolRetrievalPolicyRecommendation {
    pub(crate) scope: String,
    pub(crate) target: String,
    pub(crate) action: String,
    pub(crate) multiplier: f64,
    pub(crate) confidence: f64,
    pub(crate) reason: String,
}
