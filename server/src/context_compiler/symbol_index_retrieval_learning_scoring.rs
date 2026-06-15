use std::collections::BTreeMap;

use super::symbol_index_retrieval_learning_types::{
    SymbolRetrievalIntentLearningProfile, SymbolRetrievalPolicyRecommendation,
    SymbolRetrievalSourceLearningProfile,
};

#[derive(Debug, Default, Clone)]
pub(super) struct SourceAccumulator {
    pub(super) candidate_count: usize,
    hit_count: usize,
    noise_count: usize,
    rank_total: f64,
    token_count: usize,
}

#[derive(Debug, Default)]
pub(super) struct IntentAccumulator {
    pub(super) evaluated_count: usize,
    pub(super) recall_total: f64,
    pub(super) reciprocal_rank_total: f64,
    pub(super) noise_rate_total: f64,
    pub(super) sources: BTreeMap<String, SourceAccumulator>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SourceBaseline {
    precision: f64,
    noise_rate: f64,
}

impl SourceAccumulator {
    pub(super) fn add_candidate(&mut self, hit: bool, rank: f64, token_count: usize) {
        self.candidate_count += 1;
        self.rank_total += rank;
        self.token_count += token_count;
        if hit {
            self.hit_count += 1;
        } else {
            self.noise_count += 1;
        }
    }

    fn profile(&self, source: &str, min_samples: usize) -> SymbolRetrievalSourceLearningProfile {
        SymbolRetrievalSourceLearningProfile {
            source: source.to_string(),
            candidate_count: self.candidate_count,
            hit_count: self.hit_count,
            noise_count: self.noise_count,
            precision_at_k: round4(average(self.hit_count as f64, self.candidate_count)),
            noise_rate_at_k: round4(average(self.noise_count as f64, self.candidate_count)),
            mean_rank: round4(average(self.rank_total, self.candidate_count)),
            total_token_count: self.token_count,
            average_token_count: round4(average(self.token_count as f64, self.candidate_count)),
            confidence: confidence(self.candidate_count, min_samples),
        }
    }
}

impl IntentAccumulator {
    pub(super) fn mean_recall_at_k(&self) -> f64 {
        round4(average(self.recall_total, self.evaluated_count))
    }

    pub(super) fn mean_reciprocal_rank(&self) -> f64 {
        round4(average(self.reciprocal_rank_total, self.evaluated_count))
    }

    pub(super) fn mean_noise_rate_at_k(&self) -> f64 {
        round4(average(self.noise_rate_total, self.evaluated_count))
    }
}

pub(super) fn source_profiles(
    sources: &BTreeMap<String, SourceAccumulator>,
    min_samples: usize,
) -> Vec<SymbolRetrievalSourceLearningProfile> {
    sources
        .iter()
        .map(|(source, accumulator)| accumulator.profile(source, min_samples))
        .collect()
}

pub(super) fn policy_recommendations(
    scope: &str,
    target: &str,
    profiles: &[SymbolRetrievalSourceLearningProfile],
    baseline: SourceBaseline,
    min_samples: usize,
) -> Vec<SymbolRetrievalPolicyRecommendation> {
    profiles
        .iter()
        .map(|profile| {
            let (action, multiplier, reason) =
                recommendation_for_source(profile, baseline, min_samples);
            SymbolRetrievalPolicyRecommendation {
                scope: scope.to_string(),
                target: format!("{target}:{}", profile.source),
                action,
                multiplier,
                confidence: profile.confidence,
                reason,
            }
        })
        .collect()
}

pub(super) fn global_recommendations(
    evaluated_count: usize,
    min_samples: usize,
    recommendations: &[SymbolRetrievalPolicyRecommendation],
    intents: &[SymbolRetrievalIntentLearningProfile],
) -> Vec<String> {
    let mut out = Vec::new();
    if evaluated_count < min_samples {
        out.push(format!(
            "当前只有 {evaluated_count} 个有效 case，先继续用 eval-batch 记录 retrieval_runs，达到 {min_samples} 个后再采纳调权建议。"
        ));
    }
    for recommendation in recommendations {
        if recommendation.action == "increase" || recommendation.action == "decrease" {
            out.push(format!(
                "{} 建议 {}，multiplier={:.2}，原因：{}",
                recommendation.target,
                recommendation.action,
                recommendation.multiplier,
                recommendation.reason
            ));
        }
    }
    for intent in intents {
        if intent.evaluated_count >= min_samples
            && intent
                .recommended_weights
                .iter()
                .any(|item| item.action == "increase" || item.action == "decrease")
        {
            out.push(format!(
                "{} intent 已有足够样本，可优先把 recommendedWeights 接入该 intent 的 rank profile 灰度。",
                intent.intent
            ));
        }
    }
    if out.is_empty() {
        out.push("未发现明确调权信号，当前混合检索策略可以继续作为基线。".to_string());
    }
    out
}

pub(super) fn intent_recommendations(
    intent: &str,
    evaluated_count: usize,
    min_samples: usize,
    recommendations: &[SymbolRetrievalPolicyRecommendation],
) -> Vec<String> {
    let mut out = Vec::new();
    if evaluated_count < min_samples {
        out.push(format!(
            "{intent} intent 样本不足，先继续记录同类 query 的 eval run。"
        ));
        return out;
    }
    for recommendation in recommendations {
        if recommendation.action == "increase" || recommendation.action == "decrease" {
            out.push(format!(
                "{} 在 {intent} 中建议 {} 到 {:.2}x。",
                recommendation.target, recommendation.action, recommendation.multiplier
            ));
        }
    }
    if out.is_empty() {
        out.push(format!("{intent} intent 暂无明显调权动作。"));
    }
    out
}

pub(super) fn baseline_for(sources: &BTreeMap<String, SourceAccumulator>) -> SourceBaseline {
    let total_candidates = sources
        .values()
        .map(|source| source.candidate_count)
        .sum::<usize>();
    let total_hits = sources
        .values()
        .map(|source| source.hit_count)
        .sum::<usize>();
    let total_noise = sources
        .values()
        .map(|source| source.noise_count)
        .sum::<usize>();
    SourceBaseline {
        precision: round4(average(total_hits as f64, total_candidates)),
        noise_rate: round4(average(total_noise as f64, total_candidates)),
    }
}

fn recommendation_for_source(
    profile: &SymbolRetrievalSourceLearningProfile,
    baseline: SourceBaseline,
    min_samples: usize,
) -> (String, f64, String) {
    if profile.candidate_count < min_samples {
        return (
            "collect_more".to_string(),
            1.0,
            format!(
                "{} 样本不足，当前 {} 个候选，至少需要 {} 个再调权。",
                profile.source, profile.candidate_count, min_samples
            ),
        );
    }

    let raw_multiplier = 1.0 + (profile.precision_at_k - baseline.precision) * 0.9
        - (profile.noise_rate_at_k - baseline.noise_rate) * 0.35;
    let multiplier = round4(raw_multiplier.clamp(0.6, 1.4));
    if multiplier >= 1.08 {
        (
            "increase".to_string(),
            multiplier,
            format!(
                "{} 命中率 {:.2} 高于基线 {:.2}，可作为该策略的正向信号。",
                profile.source, profile.precision_at_k, baseline.precision
            ),
        )
    } else if multiplier <= 0.92 {
        (
            "decrease".to_string(),
            multiplier,
            format!(
                "{} 噪声率 {:.2} / 命中率 {:.2} 弱于基线，建议降权或缩小召回。",
                profile.source, profile.noise_rate_at_k, profile.precision_at_k
            ),
        )
    } else {
        (
            "keep".to_string(),
            multiplier,
            format!("{} 接近当前基线，继续观察。", profile.source),
        )
    }
}

fn confidence(candidate_count: usize, min_samples: usize) -> f64 {
    if min_samples == 0 {
        return 1.0;
    }
    round4((candidate_count as f64 / (min_samples as f64 * 3.0)).clamp(0.0, 1.0))
}

fn average(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
