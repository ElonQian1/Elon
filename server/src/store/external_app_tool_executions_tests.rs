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
