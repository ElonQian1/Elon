use anyhow::{anyhow, Result};
use rusqlite::params;
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
}
