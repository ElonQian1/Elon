use anyhow::{anyhow, Result};
use rusqlite::params;
use serde::Serialize;
use serde_json::Value;

use super::{now, Store};

#[derive(Debug, Clone)]
pub(crate) struct ExternalAppToolExecutionWrite<'a> {
    pub(crate) execution: &'a Value,
    pub(crate) app_id: &'a str,
    pub(crate) main_group_id: &'a str,
    pub(crate) external_group_id: &'a str,
    pub(crate) main_user_id: &'a str,
    pub(crate) external_user_id: Option<&'a str>,
    pub(crate) context_audit_id: Option<&'a str>,
    pub(crate) topic_hint: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminExternalAppToolExecutionSummary {
    pub app_id: String,
    pub days: i64,
    pub total_executions: i64,
    pub ready_executions: i64,
    pub partial_executions: i64,
    pub unavailable_executions: i64,
    pub planned_count: i64,
    pub result_count: i64,
    pub ready_result_count: i64,
    pub grounded_result_count: i64,
    pub weak_result_count: i64,
    pub unsafe_result_count: i64,
    pub source_id_count: i64,
    pub avg_duration_ms: f64,
    pub grounding_rate: f64,
    pub weak_rate: f64,
    pub unsafe_rate: f64,
    pub last_execution_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminExternalAppToolExecutionRow {
    pub execution_id: String,
    pub app_id: String,
    pub main_group_id: Option<String>,
    pub external_group_id: String,
    pub main_user_id: Option<String>,
    pub external_user_id: Option<String>,
    pub context_audit_id: Option<String>,
    pub topic_hint: Option<String>,
    pub status: String,
    pub planned_count: i64,
    pub result_count: i64,
    pub ready_count: i64,
    pub grounded_result_count: i64,
    pub weak_result_count: i64,
    pub unsafe_result_count: i64,
    pub source_id_count: i64,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminExternalAppToolExecutionReport {
    pub summary: AdminExternalAppToolExecutionSummary,
    pub rows: Vec<AdminExternalAppToolExecutionRow>,
}

impl Store {
    pub(crate) fn record_external_app_tool_execution(
        &self,
        input: ExternalAppToolExecutionWrite<'_>,
    ) -> Result<()> {
        let execution_id = input
            .execution
            .get("execution_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("external app tool execution_id is missing"))?;
        let status = input
            .execution
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let plan = input.execution.get("plan").cloned().unwrap_or(Value::Null);
        let results = input
            .execution
            .get("results")
            .cloned()
            .unwrap_or(Value::Null);
        let audit = input.execution.get("audit").cloned().unwrap_or(Value::Null);

        self.conn()?.execute(
            "INSERT INTO external_app_tool_executions (
                execution_id, app_id, main_group_id, external_group_id,
                main_user_id, external_user_id, context_audit_id, topic_hint,
                status, planned_count, result_count, ready_count,
                grounded_result_count, weak_result_count, unsafe_result_count,
                source_id_count, duration_ms,
                plan_json, results_json, audit_json, execution_json, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                     ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
             ON CONFLICT(execution_id) DO UPDATE SET
                status = excluded.status,
                planned_count = excluded.planned_count,
                result_count = excluded.result_count,
                ready_count = excluded.ready_count,
                grounded_result_count = excluded.grounded_result_count,
                weak_result_count = excluded.weak_result_count,
                unsafe_result_count = excluded.unsafe_result_count,
                source_id_count = excluded.source_id_count,
                duration_ms = excluded.duration_ms,
                plan_json = excluded.plan_json,
                results_json = excluded.results_json,
                audit_json = excluded.audit_json,
                execution_json = excluded.execution_json",
            params![
                execution_id,
                input.app_id,
                input.main_group_id,
                input.external_group_id,
                input.main_user_id,
                input.external_user_id,
                input.context_audit_id,
                input.topic_hint,
                status,
                i64_field(&audit, "planned_count"),
                i64_field(&audit, "result_count"),
                i64_field(&audit, "ready_count"),
                i64_field(&audit, "grounded_result_count"),
                i64_field(&audit, "weak_result_count"),
                i64_field(&audit, "unsafe_result_count"),
                i64_field(&audit, "source_id_count"),
                i64_field(&audit, "duration_ms"),
                serde_json::to_string(&plan)?,
                serde_json::to_string(&results)?,
                serde_json::to_string(&audit)?,
                serde_json::to_string(input.execution)?,
                now()
            ],
        )?;
        Ok(())
    }

    pub fn admin_external_app_tool_execution_report(
        &self,
        app_id: &str,
        days: i64,
        limit: i64,
        external_group_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<AdminExternalAppToolExecutionReport> {
        let app_id = app_id.trim();
        if app_id.is_empty() {
            anyhow::bail!("app_id is required");
        }
        let days = days.clamp(1, 365);
        let limit = limit.clamp(1, 500);
        let since = format!("-{} days", days);
        let external_group_id = clean_filter(external_group_id);
        let status = clean_filter(status);
        let conn = self.conn()?;

        let summary = conn.query_row(
            r#"SELECT COUNT(*),
                      COALESCE(SUM(CASE WHEN status = 'ready' THEN 1 ELSE 0 END), 0),
                      COALESCE(SUM(CASE WHEN status = 'partial' THEN 1 ELSE 0 END), 0),
                      COALESCE(SUM(CASE WHEN status IN ('unavailable', 'not_configured') THEN 1 ELSE 0 END), 0),
                      COALESCE(SUM(planned_count), 0),
                      COALESCE(SUM(result_count), 0),
                      COALESCE(SUM(ready_count), 0),
                      COALESCE(SUM(grounded_result_count), 0),
                      COALESCE(SUM(weak_result_count), 0),
                      COALESCE(SUM(unsafe_result_count), 0),
                      COALESCE(SUM(source_id_count), 0),
                      COALESCE(AVG(duration_ms), 0),
                      MAX(created_at)
               FROM external_app_tool_executions
               WHERE app_id = ?1
                 AND created_at >= datetime('now', ?2)
                 AND (?3 IS NULL OR external_group_id = ?3)
                 AND (?4 IS NULL OR status = ?4)"#,
            params![app_id, since, external_group_id, status],
            |row| {
                let total_executions: i64 = row.get(0)?;
                let ready_result_count: i64 = row.get(6)?;
                let grounded_result_count: i64 = row.get(7)?;
                let weak_result_count: i64 = row.get(8)?;
                let unsafe_result_count: i64 = row.get(9)?;
                Ok(AdminExternalAppToolExecutionSummary {
                    app_id: app_id.to_string(),
                    days,
                    total_executions,
                    ready_executions: row.get(1)?,
                    partial_executions: row.get(2)?,
                    unavailable_executions: row.get(3)?,
                    planned_count: row.get(4)?,
                    result_count: row.get(5)?,
                    ready_result_count,
                    grounded_result_count,
                    weak_result_count,
                    unsafe_result_count,
                    source_id_count: row.get(10)?,
                    avg_duration_ms: row.get(11)?,
                    grounding_rate: ratio(grounded_result_count, ready_result_count),
                    weak_rate: ratio(weak_result_count, ready_result_count),
                    unsafe_rate: ratio(unsafe_result_count, total_executions.max(1)),
                    last_execution_at: row.get(12)?,
                })
            },
        )?;

        let mut stmt = conn.prepare(
            r#"SELECT execution_id, app_id, main_group_id, external_group_id,
                      main_user_id, external_user_id, context_audit_id, topic_hint,
                      status, planned_count, result_count, ready_count,
                      grounded_result_count, weak_result_count, unsafe_result_count,
                      source_id_count, duration_ms, created_at
               FROM external_app_tool_executions
               WHERE app_id = ?1
                 AND created_at >= datetime('now', ?2)
                 AND (?3 IS NULL OR external_group_id = ?3)
                 AND (?4 IS NULL OR status = ?4)
               ORDER BY created_at DESC
               LIMIT ?5"#,
        )?;
        let rows = stmt
            .query_map(
                params![app_id, since, external_group_id, status, limit],
                |row| {
                    Ok(AdminExternalAppToolExecutionRow {
                        execution_id: row.get(0)?,
                        app_id: row.get(1)?,
                        main_group_id: row.get(2)?,
                        external_group_id: row.get(3)?,
                        main_user_id: row.get(4)?,
                        external_user_id: row.get(5)?,
                        context_audit_id: row.get(6)?,
                        topic_hint: row.get(7)?,
                        status: row.get(8)?,
                        planned_count: row.get(9)?,
                        result_count: row.get(10)?,
                        ready_count: row.get(11)?,
                        grounded_result_count: row.get(12)?,
                        weak_result_count: row.get(13)?,
                        unsafe_result_count: row.get(14)?,
                        source_id_count: row.get(15)?,
                        duration_ms: row.get(16)?,
                        created_at: row.get(17)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(AdminExternalAppToolExecutionReport { summary, rows })
    }
}

fn i64_field(value: &Value, field: &str) -> i64 {
    value
        .get(field)
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        })
        .unwrap_or(0)
}

fn clean_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn ratio(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        0.0
    } else {
        numerator.max(0) as f64 / denominator as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_external_app_tool_exec_{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn records_external_app_tool_execution_for_later_evaluation() {
        let store = temp_store();
        let execution = serde_json::json!({
            "schema": "external_app.executed_tools.v1",
            "execution_id": "fb2_exec_test",
            "app_id": "fb2",
            "status": "partial",
            "plan": {"planned_count": 2},
            "results": [
                {"tool_name": "search_matches", "status": "ready", "grounding": {"status": "grounded"}},
                {"tool_name": "search_user_orders", "status": "skipped"}
            ],
            "audit": {
                "planned_count": 2,
                "result_count": 2,
                "ready_count": 1,
                "grounded_result_count": 1,
                "weak_result_count": 0,
                "unsafe_result_count": 0,
                "source_id_count": 3,
                "duration_ms": 42
            }
        });

        store
            .record_external_app_tool_execution(ExternalAppToolExecutionWrite {
                execution: &execution,
                app_id: "fb2",
                main_group_id: "grp_main",
                external_group_id: "fb2_group",
                main_user_id: "usr_main",
                external_user_id: Some("fb2_user"),
                context_audit_id: Some("audit-1"),
                topic_hint: Some("今天比赛"),
            })
            .expect("execution should record");

        let conn = store.conn().expect("db connection");
        let row: (String, i64, i64, String) = conn
            .query_row(
                "SELECT status, ready_count, grounded_result_count, topic_hint
                 FROM external_app_tool_executions
                 WHERE execution_id = 'fb2_exec_test'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("execution row should exist");

        assert_eq!(row.0, "partial");
        assert_eq!(row.1, 1);
        assert_eq!(row.2, 1);
        assert_eq!(row.3, "今天比赛");
    }

    #[test]
    fn reports_external_app_tool_execution_quality_without_raw_payloads() {
        let store = temp_store();
        let first = serde_json::json!({
            "schema": "external_app.executed_tools.v1",
            "execution_id": "fb2_exec_report_1",
            "app_id": "fb2",
            "status": "ready",
            "plan": {"planned_count": 1},
            "results": [{"tool_name": "search_matches", "status": "ready"}],
            "audit": {
                "planned_count": 1,
                "result_count": 1,
                "ready_count": 1,
                "grounded_result_count": 1,
                "weak_result_count": 0,
                "unsafe_result_count": 0,
                "source_id_count": 2,
                "duration_ms": 20
            }
        });
        let second = serde_json::json!({
            "schema": "external_app.executed_tools.v1",
            "execution_id": "fb2_exec_report_2",
            "app_id": "fb2",
            "status": "partial",
            "plan": {"planned_count": 1},
            "results": [{"tool_name": "search_user_orders", "status": "ready"}],
            "audit": {
                "planned_count": 1,
                "result_count": 1,
                "ready_count": 1,
                "grounded_result_count": 0,
                "weak_result_count": 1,
                "unsafe_result_count": 0,
                "source_id_count": 0,
                "duration_ms": 40
            }
        });

        for (execution, status) in [(&first, "ready"), (&second, "partial")] {
            store
                .record_external_app_tool_execution(ExternalAppToolExecutionWrite {
                    execution,
                    app_id: "fb2",
                    main_group_id: "grp_main",
                    external_group_id: "fb2_group",
                    main_user_id: "usr_main",
                    external_user_id: Some("fb2_user"),
                    context_audit_id: None,
                    topic_hint: Some(status),
                })
                .expect("execution should record");
        }

        let report = store
            .admin_external_app_tool_execution_report("fb2", 7, 10, Some("fb2_group"), None)
            .expect("report should load");

        assert_eq!(report.summary.total_executions, 2);
        assert_eq!(report.summary.ready_executions, 1);
        assert_eq!(report.summary.partial_executions, 1);
        assert_eq!(report.summary.grounded_result_count, 1);
        assert_eq!(report.summary.weak_result_count, 1);
        assert_eq!(report.summary.source_id_count, 2);
        assert_eq!(report.summary.grounding_rate, 0.5);
        assert_eq!(report.rows.len(), 2);
        assert!(report
            .rows
            .iter()
            .any(|row| row.execution_id == "fb2_exec_report_1"));
    }
}
