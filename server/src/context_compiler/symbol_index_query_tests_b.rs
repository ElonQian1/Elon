use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection};

use super::{
    symbol_index::{SymbolEdge, SymbolIndex, SymbolRecord},
    symbol_index_chunks::{search_symbol_chunks_db, SymbolChunkSearch},
    symbol_index_embeddings::{load_symbol_embedding_status_db, SymbolEmbeddingStatus},
    symbol_index_eval::{evaluate_latest_symbol_retrieval, RetrievalEvalQuery},
    symbol_index_eval_runs::{
        evaluate_latest_symbol_retrieval_batch, list_latest_retrieval_runs,
        load_latest_retrieval_run,
    },
    symbol_index_eval_types::{
        SymbolRetrievalEvalBatchCaseQuery, SymbolRetrievalEvalBatchQuery,
        SymbolRetrievalRunHistoryQuery, SymbolRetrievalRunLookupQuery,
    },
    symbol_index_graph_query::{load_symbol_graph_db, SymbolGraphQuery, SymbolRelationDirection},
    symbol_index_impact_pack::{build_symbol_impact_pack, normalize_pack_max_chars},
    symbol_index_impact_query::load_symbol_impact_db,
    symbol_index_impact_types::SymbolImpactQuery,
    symbol_index_patch_check::{check_symbol_patch_diff, PatchDiffCheckStatus},
    symbol_index_patch_generation_types::{PatchApplyReadinessLevel, PatchGenerationMode},
    symbol_index_query::{find_symbol_index_db, search_symbol_index_db, SymbolIndexSearch},
    symbol_index_retrieval_plan::QueryIntent,
    symbol_index_store::{write_symbol_index_sqlite, SYMBOL_INDEX_DB_FILE},
    symbol_index_task_pack::{build_latest_symbol_task_pack, SymbolTaskPackQuery},
    symbol_index_vector::{
        backfill_symbol_vectors_db, search_symbol_vectors_db, SymbolVectorBackfill,
        SymbolVectorSearchQuery,
    },
    symbol_index_vector_types::LOCAL_HASH_VECTOR_MODEL,
};

use super::symbol_index_query_tests::{
    load_first_embedding_test_chunks, sample_index, symbol, temp_dir, write_bundle,
};

#[test]
fn task_pack_uses_retrieval_plan_defaults_for_refactor_queries() {
    let dir = temp_dir("elon_symbol_task_pack_refactor_plan");
    let _db = write_bundle(
        &dir,
        "20260614",
        "213012-trace-task-pack-refactor-plan-user",
        sample_index(),
    );

    let response = build_latest_symbol_task_pack(
        &dir,
        &SymbolTaskPackQuery {
            text: Some("重构 build_context_pack callers".to_string()),
            path: Some("context_pack.rs".to_string()),
            search_limit: 5,
            chunk_limit: 10,
            impact_limit: 20,
            max_chars: 12_000,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.retrieval_plan.intent, QueryIntent::Refactor);
    assert_eq!(response.query.depth, 2);
    assert!(response.ranked_context.iter().any(|item| item
        .reasons
        .iter()
        .any(|reason| reason.contains("retrieval_plan=refactor"))));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn chunk_search_uses_fts_for_symbol_module_and_test_chunks() {
    let dir = temp_dir("elon_symbol_chunks");
    let db = write_bundle(&dir, "20260614", "213013-trace-chunks-user", sample_index());

    let response = search_symbol_chunks_db(
        &db,
        &SymbolChunkSearch {
            text: Some("context pack test".to_string()),
            limit: 10,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(response
        .metadata
        .get("schema_version")
        .is_some_and(|version| version == "5"));
    assert!(response
        .metadata
        .get("chunk_count")
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|count| count >= 5));
    assert!(response
        .chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "symbol"
            && chunk
                .qualified_name
                .as_deref()
                .is_some_and(|name| name.contains("build_context_pack"))));
    assert!(response
        .chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "module"
            && chunk.file_path == "server/src/context_compiler/context_pack.rs"));
    assert!(response
        .chunks
        .iter()
        .any(|chunk| chunk.chunk_type == "test"
            && chunk.file_path == "server/src/context_compiler/context_pack_tests.rs"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn embedding_status_reports_missing_and_stale_chunks() {
    let dir = temp_dir("elon_symbol_embedding_status");
    let db = write_bundle(
        &dir,
        "20260614",
        "213014-trace-embedding-status-user",
        sample_index(),
    );
    let query = SymbolEmbeddingStatus {
        model: Some("mock-embed".to_string()),
        limit: 10,
        ..Default::default()
    };

    let initial = load_symbol_embedding_status_db(&db, &query).unwrap();
    assert!(initial.totals.embeddings_table_available);
    assert_eq!(initial.totals.embedded_count, 0);
    assert_eq!(initial.totals.missing_count, initial.totals.chunk_count);
    assert_eq!(initial.totals.stale_count, 0);
    assert!(!initial.missing_chunks.is_empty());
    assert!(initial.models.is_empty());

    let conn = Connection::open(&db).unwrap();
    let chunks = load_first_embedding_test_chunks(&conn);
    conn.execute(
        r#"
        INSERT INTO embeddings(chunk_id, model, dim, vector, content_hash, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            chunks[0].0.as_str(),
            "mock-embed",
            3_i64,
            vec![0_u8; 12],
            chunks[0].1.as_str(),
            123_i64
        ],
    )
    .unwrap();
    conn.execute(
        r#"
        INSERT INTO embeddings(chunk_id, model, dim, vector, content_hash, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            chunks[1].0.as_str(),
            "mock-embed",
            3_i64,
            vec![1_u8; 12],
            "stale-content-hash",
            124_i64
        ],
    )
    .unwrap();
    drop(conn);

    let after = load_symbol_embedding_status_db(&db, &query).unwrap();
    assert_eq!(after.totals.chunk_count, initial.totals.chunk_count);
    assert_eq!(after.totals.embedded_count, 1);
    assert_eq!(after.totals.missing_count, initial.totals.chunk_count - 1);
    assert_eq!(after.totals.stale_count, 1);
    assert!(after.totals.coverage > 0.0);
    assert_eq!(after.models.len(), 1);
    assert_eq!(after.models[0].model, "mock-embed");
    assert_eq!(after.models[0].embedding_count, 2);
    assert!(after
        .missing_chunks
        .iter()
        .all(|chunk| chunk.id != chunks[0].0));
    assert!(after
        .missing_chunks
        .iter()
        .any(|chunk| chunk.id == chunks[1].0 && chunk.has_embedding));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vector_backfill_searches_embedded_chunks_and_updates_status() {
    let dir = temp_dir("elon_symbol_vector_search");
    let db = write_bundle(
        &dir,
        "20260614",
        "213014-trace-vector-search-user",
        sample_index(),
    );

    let backfill = backfill_symbol_vectors_db(
        &db,
        &SymbolVectorBackfill {
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert_eq!(backfill.model, LOCAL_HASH_VECTOR_MODEL);
    assert!(!backfill.job_id.is_empty());
    assert_eq!(backfill.dim, 256);
    assert!(backfill.upserted_count >= 5);
    assert_eq!(backfill.skipped_count, 0);
    assert!(backfill.input_token_count > 0);
    assert_eq!(backfill.estimated_cost_micro_usd, 0);

    let status = load_symbol_embedding_status_db(
        &db,
        &SymbolEmbeddingStatus {
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            limit: 5,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(status.totals.embedded_count, status.totals.chunk_count);
    assert_eq!(status.totals.missing_count, 0);
    assert_eq!(status.totals.stale_count, 0);
    assert_eq!(status.project_status.status, "indexed");
    assert_eq!(status.queue.succeeded, 1);
    assert_eq!(status.costs.len(), 1);
    assert_eq!(status.costs[0].model, LOCAL_HASH_VECTOR_MODEL);
    assert_eq!(
        status.costs[0].input_token_count,
        backfill.input_token_count
    );

    let search = search_symbol_vectors_db(
        &db,
        &SymbolVectorSearchQuery {
            text: Some("build context pack".to_string()),
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            limit: 5,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert_eq!(search.model, LOCAL_HASH_VECTOR_MODEL);
    assert!(search
        .chunks
        .iter()
        .any(|chunk| chunk.id.contains("build_context_pack")));
    assert!(search.chunks.iter().all(|chunk| chunk.score > 0.0));

    let second = backfill_symbol_vectors_db(
        &db,
        &SymbolVectorBackfill {
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert_eq!(second.upserted_count, 0);
    assert_eq!(second.skipped_count, second.scanned_count);
    assert_eq!(second.input_token_count, 0);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn eval_reports_recall_mrr_and_missing_context_requirements() {
    let dir = temp_dir("elon_symbol_eval");
    let _db = write_bundle(&dir, "20260614", "213014-trace-eval-user", sample_index());

    let response = evaluate_latest_symbol_retrieval(
        &dir,
        &RetrievalEvalQuery {
            text: Some("build context pack".to_string()),
            must_include: vec![
                "context_pack.rs".to_string(),
                "context_pack_tests.rs".to_string(),
                "build_context_pack".to_string(),
            ],
            k: 10,
            symbol_limit: 5,
            chunk_limit: 10,
            depth: 1,
            impact_limit: 20,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.metrics.requirement_count, 3);
    assert_eq!(response.metrics.recall_at_k, 1.0);
    assert!(response.metrics.mean_reciprocal_rank > 0.0);
    assert!(response.metrics.has_test_context_at_k);
    assert!(response.metrics.noise_rate_at_k >= 0.0);
    assert!(response
        .metrics
        .decision_counts
        .contains_key("must_include"));
    assert!(response.missing_requirements.is_empty());
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.source == "symbol"));
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.source == "full_text"));
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.reasons.iter().any(|reason| reason == "fts_bm25")));
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.source.starts_with("graph_")));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn eval_uses_refactor_rank_profile_for_refactor_queries() {
    let dir = temp_dir("elon_symbol_eval_refactor_profile");
    let _db = write_bundle(
        &dir,
        "20260614",
        "213014-trace-eval-refactor-profile-user",
        sample_index(),
    );

    let response = evaluate_latest_symbol_retrieval(
        &dir,
        &RetrievalEvalQuery {
            text: Some("重构 build_context_pack callers".to_string()),
            must_include: vec!["build_context_pack".to_string()],
            k: 10,
            symbol_limit: 5,
            chunk_limit: 10,
            impact_limit: 20,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(response.ranking_profile.name, "refactor");
    assert_eq!(response.retrieval_plan.intent, QueryIntent::Refactor);
    assert!(response.retrieval_plan.graph_policy.include_references);
    assert_eq!(response.retrieval_plan.graph_policy.max_depth, 2);
    assert_eq!(response.query.depth, 2);
    assert!(response.candidates.iter().any(|candidate| candidate
        .reasons
        .iter()
        .any(|reason| reason.contains("rank_profile=refactor"))));
    assert!(response.candidates.iter().any(|candidate| candidate
        .reasons
        .iter()
        .any(|reason| reason.contains("retrieval_plan=refactor"))));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn eval_merges_vector_candidates_when_model_is_requested() {
    let dir = temp_dir("elon_symbol_eval_vector");
    let db = write_bundle(
        &dir,
        "20260614",
        "213014-trace-eval-vector-user",
        sample_index(),
    );
    backfill_symbol_vectors_db(
        &db,
        &SymbolVectorBackfill {
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let response = evaluate_latest_symbol_retrieval(
        &dir,
        &RetrievalEvalQuery {
            text: Some("build context pack".to_string()),
            must_include: vec!["build_context_pack".to_string()],
            vector_model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            vector_limit: 10,
            k: 10,
            symbol_limit: 5,
            chunk_limit: 10,
            depth: 1,
            impact_limit: 20,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(
        response.query.vector_model.as_deref(),
        Some(LOCAL_HASH_VECTOR_MODEL)
    );
    assert!(response.metrics.vector_candidate_count > 0);
    assert!(response
        .candidates
        .iter()
        .any(|candidate| candidate.sources.iter().any(|source| source == "vector")));
    assert!(response.candidates.iter().any(|candidate| candidate
        .reasons
        .iter()
        .any(|reason| reason.starts_with("merged_sources="))));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn eval_batch_aggregates_cases_and_records_retrieval_run() {
    let dir = temp_dir("elon_symbol_eval_batch");
    let db = write_bundle(
        &dir,
        "20260614",
        "213015-trace-eval-batch-user",
        sample_index(),
    );

    let response = evaluate_latest_symbol_retrieval_batch(
        &dir,
        &SymbolRetrievalEvalBatchQuery {
            trace_id: Some("trace-eval-batch".to_string()),
            record_runs: true,
            cases: vec![
                SymbolRetrievalEvalBatchCaseQuery {
                    id: "context-pack".to_string(),
                    query: RetrievalEvalQuery {
                        text: Some("解释 build context pack 流程".to_string()),
                        must_include: vec![
                            "context_pack.rs".to_string(),
                            "context_pack_tests.rs".to_string(),
                            "build_context_pack".to_string(),
                        ],
                        k: 10,
                        symbol_limit: 5,
                        chunk_limit: 10,
                        depth: 1,
                        impact_limit: 20,
                        ..Default::default()
                    },
                },
                SymbolRetrievalEvalBatchCaseQuery {
                    id: "compile-preflight".to_string(),
                    query: RetrievalEvalQuery {
                        text: Some("重构 compile_preflight_note callers".to_string()),
                        must_include: vec![
                            "mod.rs".to_string(),
                            "compile_preflight_note".to_string(),
                        ],
                        k: 10,
                        symbol_limit: 5,
                        chunk_limit: 10,
                        depth: 1,
                        impact_limit: 20,
                        ..Default::default()
                    },
                },
            ],
        },
    )
    .unwrap();

    assert_eq!(response.case_count, 2);
    assert_eq!(response.evaluated_count, 2);
    assert_eq!(response.failed_count, 0);
    assert!(response.recorded);
    assert!(response.record_error.is_none());
    assert_eq!(response.aggregate.requirement_count, 5);
    assert_eq!(response.aggregate.mean_recall_at_k, 1.0);
    assert!(response.aggregate.mean_reciprocal_rank > 0.0);
    assert!(response.aggregate.has_test_context_rate > 0.0);
    assert!(response.aggregate.mean_noise_rate_at_k >= 0.0);
    assert_eq!(response.intent_groups.len(), 2);
    assert!(response.intent_groups.iter().any(|group| {
        group.intent == QueryIntent::Explain.as_str() && group.evaluated_count == 1
    }));
    assert!(response.intent_groups.iter().any(|group| {
        group.intent == QueryIntent::Refactor.as_str() && group.evaluated_count == 1
    }));
    assert!(response
        .cases
        .iter()
        .all(|case| case.ok && case.result.is_some()));

    let conn = Connection::open(&db).unwrap();
    let selected_chunks_json: String = conn
        .query_row(
            "SELECT selected_chunks_json FROM retrieval_runs WHERE id = ?1",
            [response.run_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(selected_chunks_json.contains("build_context_pack"));
    assert!(selected_chunks_json.contains("compile_preflight_note"));

    let history = list_latest_retrieval_runs(
        &dir,
        &SymbolRetrievalRunHistoryQuery {
            trace_id: Some("trace-eval-batch".to_string()),
            limit: 10,
        },
    )
    .unwrap();
    assert_eq!(history.runs.len(), 1);
    assert_eq!(history.runs[0].id.as_str(), response.run_id.as_str());
    assert_eq!(history.runs[0].scores["caseCount"].as_u64(), Some(2));
    assert!(history.runs[0].scores["intentGroups"].is_array());

    let detail = load_latest_retrieval_run(
        &dir,
        &SymbolRetrievalRunLookupQuery {
            trace_id: Some("trace-eval-batch".to_string()),
            id: response.run_id.clone(),
        },
    )
    .unwrap();
    assert_eq!(detail.run.id.as_str(), response.run_id.as_str());
    assert_eq!(detail.run.scores["evaluatedCount"].as_u64(), Some(2));
    assert!(detail.run.scores["intentGroups"].is_array());
    assert!(detail
        .run
        .selected_chunks
        .to_string()
        .contains("context_pack_tests.rs"));

    let status = load_symbol_embedding_status_db(
        &db,
        &SymbolEmbeddingStatus {
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            limit: 10,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(status.eval_set.case_count, 2);
    assert!(status
        .eval_set
        .latest_cases
        .iter()
        .any(|case| case.id == "context-pack"
            && case.source == "real_task"
            && case.last_run_id.as_deref() == Some(response.run_id.as_str())));

    drop(conn);
    fs::remove_dir_all(dir).unwrap();
}
