use serde_json::{Value, json};

use super::{
    symbol_index_eval_compare::compare_retrieval_run_details,
    symbol_index_eval_types::SymbolRetrievalRunDetail,
};

#[test]
fn compare_retrieval_runs_reports_case_regressions_and_improvements() {
    let baseline = run_detail(
        "baseline",
        scores(json!({
            "requirementCount": 4,
            "hitCountAtK": 3,
            "missingRequirementCount": 1,
            "meanRecallAtK": 0.75,
            "meanReciprocalRank": 0.75,
            "hasTestContextRate": 0.5,
            "noiseCountAtK": 3,
            "meanNoiseRateAtK": 0.15,
            "totalTokenCountAtK": 1600,
            "averageTokenCountAtK": 800,
            "candidateCount": 12
        })),
        json!([
            case_value(
                "case-a",
                true,
                "解释 context pack",
                json!({
                    "recallAtK": 0.5,
                    "meanReciprocalRank": 0.5,
                    "noiseRateAtK": 0.2,
                    "totalTokenCountAtK": 800
                }),
                json!(["context_pack_tests.rs"]),
                "server/src/context_compiler/context_pack.rs",
                "build_context_pack"
            ),
            case_value(
                "case-b",
                true,
                "重构 compile preflight",
                json!({
                    "recallAtK": 1.0,
                    "meanReciprocalRank": 1.0,
                    "noiseRateAtK": 0.1,
                    "totalTokenCountAtK": 700
                }),
                json!([]),
                "server/src/context_compiler/mod.rs",
                "compile_preflight_note"
            )
        ]),
    );
    let current = run_detail(
        "current",
        scores(json!({
            "requirementCount": 4,
            "hitCountAtK": 3,
            "missingRequirementCount": 1,
            "meanRecallAtK": 0.75,
            "meanReciprocalRank": 0.7,
            "hasTestContextRate": 0.5,
            "noiseCountAtK": 5,
            "meanNoiseRateAtK": 0.3,
            "totalTokenCountAtK": 1500,
            "averageTokenCountAtK": 750,
            "candidateCount": 12
        })),
        json!([
            case_value(
                "case-a",
                true,
                "解释 context pack",
                json!({
                    "recallAtK": 1.0,
                    "meanReciprocalRank": 1.0,
                    "noiseRateAtK": 0.1,
                    "totalTokenCountAtK": 700
                }),
                json!([]),
                "server/src/context_compiler/context_pack.rs",
                "build_context_pack"
            ),
            case_value(
                "case-b",
                true,
                "重构 compile preflight",
                json!({
                    "recallAtK": 0.5,
                    "meanReciprocalRank": 0.4,
                    "noiseRateAtK": 0.5,
                    "totalTokenCountAtK": 800
                }),
                json!(["compile_preflight_note"]),
                "server/src/context_compiler/context_pack_tests.rs",
                "build_context_pack_test"
            )
        ]),
    );

    let response =
        compare_retrieval_run_details("symbol_index.sqlite".to_string(), baseline, current, 20);

    assert_eq!(response.verdict, "regressed");
    assert_eq!(response.regression_count, 1);
    assert_eq!(response.improvement_count, 1);
    assert_eq!(response.returned_case_count, 2);
    assert_eq!(response.case_deltas[0].id, "case-b");
    assert_eq!(response.case_deltas[0].status, "regressed");
    assert_eq!(
        response.case_deltas[0].new_missing_requirements,
        vec!["compile_preflight_note".to_string()]
    );
    assert_eq!(response.case_deltas[1].id, "case-a");
    assert_eq!(response.case_deltas[1].status, "improved");
    assert_eq!(
        response.case_deltas[1].resolved_missing_requirements,
        vec!["context_pack_tests.rs".to_string()]
    );

    let noise_delta = response
        .aggregate_deltas
        .iter()
        .find(|delta| delta.metric == "meanNoiseRateAtK")
        .unwrap();
    assert_eq!(noise_delta.status, "regressed");
    assert!(
        response
            .recommendations
            .iter()
            .any(|item| item.contains("回归 case"))
    );
}

#[test]
fn compare_retrieval_runs_limits_cases_after_prioritizing_regressions() {
    let baseline = run_detail(
        "baseline",
        scores(json!({
            "requirementCount": 2,
            "hitCountAtK": 2,
            "missingRequirementCount": 0,
            "meanRecallAtK": 1.0,
            "meanReciprocalRank": 1.0,
            "hasTestContextRate": 0.0,
            "noiseCountAtK": 0,
            "meanNoiseRateAtK": 0.0,
            "totalTokenCountAtK": 200,
            "averageTokenCountAtK": 100,
            "candidateCount": 2
        })),
        json!([
            case_value(
                "case-a",
                true,
                "a",
                json!({"recallAtK": 1.0}),
                json!([]),
                "a.rs",
                "a"
            ),
            case_value(
                "case-b",
                true,
                "b",
                json!({"recallAtK": 1.0}),
                json!([]),
                "b.rs",
                "b"
            )
        ]),
    );
    let current = run_detail(
        "current",
        scores(json!({
            "requirementCount": 2,
            "hitCountAtK": 1,
            "missingRequirementCount": 1,
            "meanRecallAtK": 0.5,
            "meanReciprocalRank": 0.5,
            "hasTestContextRate": 0.0,
            "noiseCountAtK": 0,
            "meanNoiseRateAtK": 0.0,
            "totalTokenCountAtK": 200,
            "averageTokenCountAtK": 100,
            "candidateCount": 2
        })),
        json!([
            case_value(
                "case-a",
                true,
                "a",
                json!({"recallAtK": 1.0}),
                json!([]),
                "a.rs",
                "a"
            ),
            case_value(
                "case-b",
                true,
                "b",
                json!({"recallAtK": 0.0}),
                json!(["b.rs"]),
                "b.rs",
                "b"
            )
        ]),
    );

    let response =
        compare_retrieval_run_details("symbol_index.sqlite".to_string(), baseline, current, 1);

    assert_eq!(response.total_compared_cases, 2);
    assert_eq!(response.returned_case_count, 1);
    assert_eq!(response.case_deltas[0].id, "case-b");
    assert_eq!(response.case_deltas[0].status, "regressed");
}

fn run_detail(id: &str, scores: Value, selected_chunks: Value) -> SymbolRetrievalRunDetail {
    SymbolRetrievalRunDetail {
        id: id.to_string(),
        query: format!("{id} query"),
        selected_chunks,
        scores,
        created_at: 100,
    }
}

fn scores(aggregate: Value) -> Value {
    json!({
        "caseCount": 2,
        "evaluatedCount": 2,
        "failedCount": 0,
        "aggregate": aggregate,
        "intentGroups": [{
            "intent": "explain",
            "evaluatedCount": 2,
            "requirementCount": aggregate["requirementCount"].clone(),
            "hitCountAtK": aggregate["hitCountAtK"].clone(),
            "missingRequirementCount": aggregate["missingRequirementCount"].clone(),
            "meanRecallAtK": aggregate["meanRecallAtK"].clone(),
            "meanReciprocalRank": aggregate["meanReciprocalRank"].clone(),
            "hasTestContextRate": aggregate["hasTestContextRate"].clone(),
            "noiseCountAtK": aggregate["noiseCountAtK"].clone(),
            "meanNoiseRateAtK": aggregate["meanNoiseRateAtK"].clone(),
            "totalTokenCountAtK": aggregate["totalTokenCountAtK"].clone(),
            "averageTokenCountAtK": aggregate["averageTokenCountAtK"].clone(),
            "candidateCount": aggregate["candidateCount"].clone()
        }]
    })
}

fn case_value(
    id: &str,
    ok: bool,
    query: &str,
    metrics: Value,
    missing_requirements: Value,
    file_path: &str,
    label: &str,
) -> Value {
    json!({
        "id": id,
        "ok": ok,
        "error": null,
        "result": {
            "query": {"q": query},
            "metrics": metrics,
            "missingRequirements": missing_requirements,
            "candidates": [{
                "rank": 1,
                "filePath": file_path,
                "label": label
            }]
        }
    })
}
