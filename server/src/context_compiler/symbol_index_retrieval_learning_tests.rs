use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection};
use serde_json::json;

use super::{
    symbol_index_retrieval_learning::build_symbol_retrieval_learning_report_db,
    symbol_index_retrieval_learning_types::SymbolRetrievalLearningQuery,
    symbol_index_store::create_retrieval_runs_schema,
};

#[test]
fn retrieval_learning_recommends_strong_source_for_intent() {
    let db_path = temp_db_path("strong-source");
    seed_runs(&db_path, sample_cases()).expect("seed retrieval runs");

    let response = build_symbol_retrieval_learning_report_db(
        &db_path,
        &SymbolRetrievalLearningQuery {
            min_samples: 1,
            top_k: 3,
            limit: 10,
            ..Default::default()
        },
    )
    .expect("learning report");

    assert_eq!(response.run_count, 1);
    assert_eq!(response.evaluated_count, 2);
    assert_eq!(response.learning_status, "ready");
    assert!(
        response
            .recommended_weights
            .iter()
            .any(|item| item.target == "all:vector" && item.action == "increase"),
        "vector should be increased: {:?}",
        response.recommended_weights
    );
    assert!(
        response.intent_profiles.iter().any(|profile| {
            profile.intent == "explanation"
                && profile
                    .recommended_weights
                    .iter()
                    .any(|item| item.target == "explanation:vector" && item.action == "increase")
        }),
        "explanation intent should favor vector: {:?}",
        response.intent_profiles
    );

    let _ = fs::remove_file(db_path);
}

#[test]
fn retrieval_learning_handles_missing_runs_table() {
    let db_path = temp_db_path("empty");
    Connection::open(&db_path).expect("create empty db");

    let response = build_symbol_retrieval_learning_report_db(
        &db_path,
        &SymbolRetrievalLearningQuery {
            min_samples: 2,
            ..Default::default()
        },
    )
    .expect("empty report");

    assert_eq!(response.run_count, 0);
    assert_eq!(response.learning_status, "collecting");
    assert_eq!(response.recommendations.len(), 1);

    let _ = fs::remove_file(db_path);
}

fn seed_runs(db_path: &PathBuf, selected_chunks: serde_json::Value) -> rusqlite::Result<()> {
    let conn = Connection::open(db_path)?;
    create_retrieval_runs_schema(&conn)?;
    conn.execute(
        r#"
        INSERT INTO retrieval_runs(id, query, selected_chunks_json, scores_json, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params!["run-1", "sample", selected_chunks.to_string(), "{}", 1_i64,],
    )?;
    Ok(())
}

fn sample_cases() -> serde_json::Value {
    json!([
        {
            "id": "case-1",
            "ok": true,
            "result": {
                "query": { "q": "权限校验在哪里", "k": 3 },
                "retrievalPlan": { "intent": "explanation" },
                "metrics": {
                    "recallAtK": 1.0,
                    "meanReciprocalRank": 1.0,
                    "noiseRateAtK": 0.3333
                },
                "candidates": [
                    {
                        "rank": 1,
                        "source": "vector",
                        "sources": ["vector"],
                        "tokenCount": 80,
                        "matchedRequirements": ["auth policy"]
                    },
                    {
                        "rank": 2,
                        "source": "full_text",
                        "sources": ["full_text"],
                        "tokenCount": 40,
                        "matchedRequirements": []
                    },
                    {
                        "rank": 3,
                        "source": "symbol",
                        "sources": ["symbol"],
                        "tokenCount": 30,
                        "matchedRequirements": []
                    }
                ]
            }
        },
        {
            "id": "case-2",
            "ok": true,
            "result": {
                "query": { "q": "鉴权流程", "k": 3 },
                "retrievalPlan": { "intent": "explanation" },
                "metrics": {
                    "recallAtK": 1.0,
                    "meanReciprocalRank": 1.0,
                    "noiseRateAtK": 0.3333
                },
                "candidates": [
                    {
                        "rank": 1,
                        "source": "vector",
                        "sources": ["vector", "graph_file"],
                        "tokenCount": 90,
                        "matchedRequirements": ["auth flow"]
                    },
                    {
                        "rank": 2,
                        "source": "full_text",
                        "sources": ["full_text"],
                        "tokenCount": 60,
                        "matchedRequirements": []
                    }
                ]
            }
        }
    ])
}

fn temp_db_path(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elon-symbol-retrieval-learning-{label}-{stamp}.sqlite"
    ))
}
