use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::{
    symbol_index_query::find_symbol_index_db,
    symbol_index_retrieval_learning_json::{
        bool_field, candidate_sources, number_field, parse_json, result_intent, result_top_k,
        string_items, top_candidates,
    },
    symbol_index_retrieval_learning_scoring::{
        baseline_for, global_recommendations, intent_recommendations, policy_recommendations,
        source_profiles, IntentAccumulator, SourceAccumulator,
    },
    symbol_index_retrieval_learning_types::{
        SymbolRetrievalIntentLearningProfile, SymbolRetrievalLearningQuery,
        SymbolRetrievalLearningQueryEcho, SymbolRetrievalLearningResponse,
    },
};

pub(crate) fn build_latest_symbol_retrieval_learning_report(
    data_dir: &Path,
    query: &SymbolRetrievalLearningQuery,
) -> Result<SymbolRetrievalLearningResponse> {
    let db_path = find_symbol_index_db(data_dir, query.trace_id.as_deref())
        .context("没有找到可学习的 symbol_index.sqlite，请先运行一次 context compiler")?;
    build_symbol_retrieval_learning_report_db(&db_path, query)
}

pub(crate) fn build_symbol_retrieval_learning_report_db(
    db_path: &Path,
    query: &SymbolRetrievalLearningQuery,
) -> Result<SymbolRetrievalLearningResponse> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
    let db_path_text = db_path.to_string_lossy().replace('\\', "/");
    if !retrieval_runs_table_exists(&conn)? {
        return Ok(empty_response(db_path_text, query));
    }

    let mut accumulator = LearningAccumulator::default();
    let mut stmt = conn.prepare(
        r#"
        SELECT id, selected_chunks_json
        FROM retrieval_runs
        ORDER BY created_at DESC, id DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([i64::try_from(query.limit()).unwrap_or(i64::MAX)], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (_id, selected_chunks_json) = row?;
        let selected_chunks = parse_json(&selected_chunks_json);
        accumulator.add_run(&selected_chunks, query.top_k());
    }

    Ok(accumulator.finish(db_path_text, query))
}

#[derive(Debug, Default)]
struct LearningAccumulator {
    run_count: usize,
    case_count: usize,
    evaluated_count: usize,
    sources: BTreeMap<String, SourceAccumulator>,
    intents: BTreeMap<String, IntentAccumulator>,
}

impl LearningAccumulator {
    fn add_run(&mut self, selected_chunks: &Value, default_top_k: usize) {
        self.run_count += 1;
        for case in selected_chunks.as_array().into_iter().flatten() {
            self.case_count += 1;
            if !bool_field(case, "ok") {
                continue;
            }
            let Some(result) = super::symbol_index_retrieval_learning_json::field(case, "result")
            else {
                continue;
            };
            self.evaluated_count += 1;
            let intent = result_intent(result);
            let case_top_k = result_top_k(result)
                .unwrap_or(default_top_k)
                .min(default_top_k);
            let metrics = super::symbol_index_retrieval_learning_json::field(result, "metrics")
                .unwrap_or(&Value::Null);
            let intent_entry = self.intents.entry(intent).or_default();
            intent_entry.evaluated_count += 1;
            intent_entry.recall_total += number_field(metrics, "recallAtK").unwrap_or_default();
            intent_entry.reciprocal_rank_total +=
                number_field(metrics, "meanReciprocalRank").unwrap_or_default();
            intent_entry.noise_rate_total +=
                number_field(metrics, "noiseRateAtK").unwrap_or_default();

            for candidate in top_candidates(result, case_top_k) {
                let hit = !string_items(candidate, "matchedRequirements").is_empty();
                let rank = number_field(candidate, "rank").unwrap_or(1.0);
                let token_count = super::symbol_index_retrieval_learning_json::usize_field(
                    candidate,
                    "tokenCount",
                )
                .unwrap_or_default();
                for source in candidate_sources(candidate) {
                    self.sources
                        .entry(source.clone())
                        .or_default()
                        .add_candidate(hit, rank, token_count);
                    intent_entry
                        .sources
                        .entry(source)
                        .or_default()
                        .add_candidate(hit, rank, token_count);
                }
            }
        }
    }

    fn finish(
        self,
        db_path: String,
        query: &SymbolRetrievalLearningQuery,
    ) -> SymbolRetrievalLearningResponse {
        let min_samples = query.min_samples();
        let source_profiles = source_profiles(&self.sources, min_samples);
        let baseline = baseline_for(&self.sources);
        let recommended_weights =
            policy_recommendations("global", "all", &source_profiles, baseline, min_samples);
        let intent_profiles = self
            .intents
            .iter()
            .map(|(intent, accumulator)| intent_profile(intent, accumulator, min_samples))
            .collect::<Vec<_>>();
        let candidate_count = self
            .sources
            .values()
            .map(|source| source.candidate_count)
            .sum();
        let learning_status = if self.evaluated_count >= min_samples {
            "ready"
        } else {
            "collecting"
        }
        .to_string();
        let recommendations = global_recommendations(
            self.evaluated_count,
            min_samples,
            &recommended_weights,
            &intent_profiles,
        );

        SymbolRetrievalLearningResponse {
            db_path,
            query: SymbolRetrievalLearningQueryEcho {
                trace_id: query.trace_id.clone(),
                limit: query.limit(),
                min_samples,
                top_k: query.top_k(),
            },
            learning_status,
            run_count: self.run_count,
            case_count: self.case_count,
            evaluated_count: self.evaluated_count,
            candidate_count,
            source_profiles,
            intent_profiles,
            recommended_weights,
            recommendations,
        }
    }
}

fn intent_profile(
    intent: &str,
    accumulator: &IntentAccumulator,
    min_samples: usize,
) -> SymbolRetrievalIntentLearningProfile {
    let source_profiles = source_profiles(&accumulator.sources, min_samples);
    let baseline = baseline_for(&accumulator.sources);
    let recommended_weights =
        policy_recommendations("intent", intent, &source_profiles, baseline, min_samples);
    let recommendations = intent_recommendations(
        intent,
        accumulator.evaluated_count,
        min_samples,
        &recommended_weights,
    );

    SymbolRetrievalIntentLearningProfile {
        intent: intent.to_string(),
        evaluated_count: accumulator.evaluated_count,
        candidate_count: accumulator
            .sources
            .values()
            .map(|source| source.candidate_count)
            .sum(),
        mean_recall_at_k: accumulator.mean_recall_at_k(),
        mean_reciprocal_rank: accumulator.mean_reciprocal_rank(),
        mean_noise_rate_at_k: accumulator.mean_noise_rate_at_k(),
        source_profiles,
        recommended_weights,
        recommendations,
    }
}

fn empty_response(
    db_path: String,
    query: &SymbolRetrievalLearningQuery,
) -> SymbolRetrievalLearningResponse {
    SymbolRetrievalLearningResponse {
        db_path,
        query: SymbolRetrievalLearningQueryEcho {
            trace_id: query.trace_id.clone(),
            limit: query.limit(),
            min_samples: query.min_samples(),
            top_k: query.top_k(),
        },
        learning_status: "collecting".to_string(),
        run_count: 0,
        case_count: 0,
        evaluated_count: 0,
        candidate_count: 0,
        source_profiles: Vec::new(),
        intent_profiles: Vec::new(),
        recommended_weights: Vec::new(),
        recommendations: vec![
            "retrieval_runs 表不存在或暂无数据，请先运行 eval-batch 并启用 recordRuns。"
                .to_string(),
        ],
    }
}

fn retrieval_runs_table_exists(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'retrieval_runs')",
        [],
        |row| row.get::<_, bool>(0),
    )
}
