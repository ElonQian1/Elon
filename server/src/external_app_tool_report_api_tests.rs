use super::*;

fn summary() -> AdminExternalAppToolExecutionSummary {
    AdminExternalAppToolExecutionSummary {
        app_id: "fb2".to_string(),
        days: 7,
        total_executions: 10,
        ready_executions: 4,
        partial_executions: 6,
        unavailable_executions: 1,
        planned_count: 12,
        result_count: 10,
        ready_result_count: 8,
        grounded_result_count: 4,
        weak_result_count: 3,
        unsafe_result_count: 1,
        source_id_count: 3,
        avg_duration_ms: 3500.0,
        grounding_rate: 0.5,
        weak_rate: 0.375,
        unsafe_rate: 0.1,
        last_execution_at: Some("2026-06-21T09:00:00Z".to_string()),
    }
}

#[test]
fn recommendations_explain_no_data_before_traffic_exists() {
    let mut summary = summary();
    summary.total_executions = 0;

    let recommendations = quality_recommendations(&summary);

    assert_eq!(recommendations.len(), 1);
    assert_eq!(recommendations[0].code, "no_tool_execution_data");
}

#[test]
fn recommendations_prioritize_unsafe_and_grounding_gaps() {
    let recommendations = quality_recommendations(&summary());
    let codes = recommendations
        .iter()
        .map(|item| item.code)
        .collect::<Vec<_>>();

    assert_eq!(codes[0], "unsafe_tool_results_present");
    assert!(codes.contains(&"low_grounding_rate"));
    assert!(codes.contains(&"low_source_id_coverage"));
    assert!(codes.contains(&"tool_latency_high"));
}

#[test]
fn recommendations_mark_healthy_when_metrics_are_good() {
    let mut summary = summary();
    summary.ready_executions = 10;
    summary.partial_executions = 0;
    summary.unavailable_executions = 0;
    summary.ready_result_count = 10;
    summary.grounded_result_count = 10;
    summary.weak_result_count = 0;
    summary.unsafe_result_count = 0;
    summary.source_id_count = 12;
    summary.avg_duration_ms = 600.0;
    summary.grounding_rate = 1.0;
    summary.weak_rate = 0.0;
    summary.unsafe_rate = 0.0;

    let recommendations = quality_recommendations(&summary);

    assert_eq!(recommendations.len(), 1);
    assert_eq!(recommendations[0].code, "tool_quality_healthy");
}

#[test]
fn report_filters_clamp_and_trim_query_values() {
    let q = ExternalAppToolExecutionReportQuery {
        app_id: Some("other".to_string()),
        days: 0,
        limit: 999,
        external_group_id: Some(" fb2-main ".to_string()),
        status: Some("   ".to_string()),
    };

    let filters = report_filters_from_query(&q);

    assert_eq!(filters.days, 1);
    assert_eq!(filters.limit, 500);
    assert_eq!(filters.external_group_id, Some("fb2-main"));
    assert_eq!(filters.status, None);
}
