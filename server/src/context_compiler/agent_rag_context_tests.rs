    use serde_json::json;

    use super::*;

    #[test]
    fn exposes_agent_rag_tools() {
        let names = tool_definitions()
            .into_iter()
            .filter_map(|tool| {
                tool.get("function")?
                    .get("name")?
                    .as_str()
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();

        assert!(names.contains(&TOOL_CONTEXT_STATUS.to_string()));
        assert!(names.contains(&TOOL_SYMBOL_SEARCH.to_string()));
        assert!(names.contains(&TOOL_CONTEXT_TASK_PACK.to_string()));
    }

    #[test]
    fn task_pack_tool_schema_exposes_vector_model() {
        let task_pack_tool = tool_definitions()
            .into_iter()
            .find(|tool| {
                tool.pointer("/function/name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| name == TOOL_CONTEXT_TASK_PACK)
            })
            .expect("task pack tool");

        let vector_model = task_pack_tool
            .pointer("/function/parameters/properties/vectorModel/description")
            .and_then(Value::as_str)
            .expect("vector model description");

        assert!(vector_model.contains(LOCAL_HASH_VECTOR_MODEL));
    }

    #[test]
    fn task_pack_query_auto_enables_vector_for_semantic_tasks() {
        let query = task_pack_query(
            &json!({
                "q": "新增 refresh token",
                "maxChars": 12000
            }),
            Some("trace-1"),
            None,
        )
        .expect("query");

        assert_eq!(query.text.as_deref(), Some("新增 refresh token"));
        assert_eq!(query.trace_id.as_deref(), Some("trace-1"));
        assert_eq!(query.max_chars, 12000);
        assert_eq!(query.vector_model.as_deref(), Some(LOCAL_HASH_VECTOR_MODEL));
    }

    #[test]
    fn task_pack_query_auto_disables_vector_for_precision_tasks() {
        let query = task_pack_query(
            &json!({
                "q": "登录失败为什么返回 500？"
            }),
            Some("server-trace"),
            None,
        )
        .expect("query");

        assert_eq!(query.text.as_deref(), Some("登录失败为什么返回 500？"));
        assert_eq!(query.trace_id.as_deref(), Some("server-trace"));
        assert_eq!(query.vector_model, None);
    }

    #[test]
    fn task_pack_query_can_force_vector_retrieval() {
        let query = task_pack_query(
            &json!({
                "query": "登录失败为什么返回 500？",
                "useVector": true
            }),
            Some("server-trace"),
            None,
        )
        .expect("query");

        assert_eq!(query.text.as_deref(), Some("登录失败为什么返回 500？"));
        assert_eq!(query.vector_model.as_deref(), Some(LOCAL_HASH_VECTOR_MODEL));
    }

    #[test]
    fn task_pack_query_can_disable_vector_retrieval() {
        let query = task_pack_query(
            &json!({
                "query": "inspect only",
                "useVector": false
            }),
            Some("server-trace"),
            None,
        )
        .expect("query");

        assert_eq!(query.text.as_deref(), Some("inspect only"));
        assert_eq!(query.trace_id.as_deref(), Some("server-trace"));
        assert_eq!(query.vector_model, None);
    }

    #[test]
    fn task_pack_query_rejects_unsupported_vector_model() {
        let err = task_pack_query(
            &json!({
                "q": "inspect",
                "vectorModel": "bge-m3"
            }),
            Some("server-trace"),
            None,
        )
        .expect_err("unsupported vector model");

        assert!(err.to_string().contains("暂未配置 provider"));
        assert!(err.to_string().contains(LOCAL_HASH_VECTOR_MODEL));
    }

    #[test]
    fn task_pack_query_accepts_remote_embedding_with_provider_context() {
        let provider_context = SymbolEmbeddingProviderContext::from_agent(
            "https://api.example.com/v1",
            "sk-user",
            "user_api_key_proxy",
        );
        let query = task_pack_query(
            &json!({
                "q": "解释登录流程",
                "useVector": true,
                "vectorModel": "openai:text-embedding-3-small"
            }),
            Some("server-trace"),
            Some(&provider_context),
        )
        .expect("remote embedding query");

        assert_eq!(
            query.vector_model.as_deref(),
            Some("openai:text-embedding-3-small")
        );
    }

    #[test]
    fn task_pack_query_uses_configured_embedding_model_by_default() {
        let provider_context = SymbolEmbeddingProviderContext::from_agent(
            "https://api.example.com/v1",
            "sk-user",
            "user_api_key_proxy",
        )
        .with_embedding_model(Some("openai:text-embedding-3-small"));

        let query = task_pack_query(
            &json!({
                "q": "解释登录流程"
            }),
            Some("server-trace"),
            Some(&provider_context),
        )
        .expect("remote embedding query");

        assert_eq!(
            query.vector_model.as_deref(),
            Some("openai:text-embedding-3-small")
        );
    }

    #[test]
    fn remote_vector_backfill_limit_is_bounded() {
        assert_eq!(
            vector_backfill_limit(&json!({}), "openai:text-embedding-3-small"),
            DEFAULT_REMOTE_VECTOR_BACKFILL_LIMIT
        );
        assert_eq!(
            vector_backfill_limit(&json!({ "vectorBackfillLimit": 20_000 }), "remote:bge-m3"),
            MAX_REMOTE_VECTOR_BACKFILL_LIMIT
        );
        assert_eq!(
            vector_backfill_limit(&json!({}), LOCAL_HASH_VECTOR_MODEL),
            0
        );
    }

    #[test]
    fn task_pack_query_ignores_unsupported_vector_model_when_vector_disabled() {
        let query = task_pack_query(
            &json!({
                "q": "解释登录流程",
                "useVector": false,
                "vectorModel": "bge-m3"
            }),
            Some("server-trace"),
            None,
        )
        .expect("query");

        assert_eq!(query.vector_model, None);
    }

    #[test]
    fn task_pack_query_prefers_server_trace_over_tool_args() {
        let query = task_pack_query(
            &json!({
                "q": "inspect",
                "traceId": "model-supplied"
            }),
            Some("server-trace"),
            None,
        )
        .expect("query");

        assert_eq!(query.trace_id.as_deref(), Some("server-trace"));
    }

    #[test]
    fn task_pack_query_rejects_model_supplied_trace_without_server_trace() {
        let err = task_pack_query(
            &json!({
                "q": "inspect",
                "traceId": "model-supplied"
            }),
            None,
            None,
        )
        .expect_err("server trace is required");

        assert!(err.to_string().contains("缺少 trace_id"));
    }
