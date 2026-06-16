use std::path::Path;

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};

use crate::types::AgentConfig;

use super::{
    agent_rag_project_docs::{load_agent_project_docs, prepend_project_docs_to_pack},
    agent_rag_vector_policy::{choose_agent_vector_policy, AgentVectorPolicy},
    symbol_index_embedding_provider::{
        is_remote_embedding_model, resolve_embedding_provider, SymbolEmbeddingProviderContext,
    },
    symbol_index_embeddings::{load_latest_symbol_embedding_status, SymbolEmbeddingStatus},
    symbol_index_query::{
        find_symbol_index_db, load_metadata, search_latest_symbol_index, SymbolIndexSearch,
    },
    symbol_index_task_pack::{build_latest_symbol_task_pack_with_context, SymbolTaskPackQuery},
    symbol_index_vector::{backfill_latest_symbol_vectors_with_context, SymbolVectorBackfill},
    symbol_index_vector_types::{LOCAL_HASH_VECTOR_MODEL, SUPPORTED_EMBEDDING_MODELS},
};

const TOOL_CONTEXT_STATUS: &str = "repo_context_status";
const TOOL_SYMBOL_SEARCH: &str = "repo_symbol_search";
const TOOL_CONTEXT_TASK_PACK: &str = "repo_context_task_pack";
const DEFAULT_REMOTE_VECTOR_BACKFILL_LIMIT: usize = 64;
const MAX_REMOTE_VECTOR_BACKFILL_LIMIT: usize = 512;

pub(crate) fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": TOOL_CONTEXT_STATUS,
                "description": "查看当前项目的 repo map、符号索引、RAG chunk 与本地向量索引状态。已有项目或跨文件任务开始时优先调用。",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": TOOL_SYMBOL_SEARCH,
                "description": "在当前项目符号索引中搜索函数、结构体、模块、trait、调用边和引用关系；用于定位定义、入口和影响面。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "q": {
                            "type": "string",
                            "description": "搜索文本，例如函数名、模块名、错误关键词或用户任务关键词。"
                        },
                        "kind": {
                            "type": "string",
                            "description": "可选。过滤符号类型，例如 function、struct、enum、trait、impl、module。"
                        },
                        "path": {
                            "type": "string",
                            "description": "可选。按文件路径片段过滤。"
                        },
                        "edgeKind": {
                            "type": "string",
                            "description": "可选。过滤关系类型，例如 calls、implements、references。"
                        },
                        "includeEdges": {
                            "type": "boolean",
                            "description": "是否返回相关边。需要理解引用/调用关系时设为 true。"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "返回符号数量，默认 20，最大 100。"
                        }
                    },
                    "required": ["q"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": TOOL_CONTEXT_TASK_PACK,
                "description": "为当前用户任务生成压缩的 RAG 上下文包，先融合 AI 项目说明文档，再融合符号搜索、全文 chunk、本地向量检索、影响分析、补丁计划和验证线索。跨文件修改前优先调用。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "q": {
                            "type": "string",
                            "description": "当前任务或要调查的问题。"
                        },
                        "kind": {
                            "type": "string",
                            "description": "可选。种子符号类型过滤。"
                        },
                        "path": {
                            "type": "string",
                            "description": "可选。限定到某个文件或目录片段。"
                        },
                        "edgeKind": {
                            "type": "string",
                            "description": "可选。影响分析关系类型过滤。"
                        },
                        "maxChars": {
                            "type": "integer",
                            "description": "上下文包字符预算。默认由后端决定，建议 8000-16000。"
                        },
                        "useVector": {
                            "type": "boolean",
                            "description": "是否启用向量检索；不填时由任务意图自动决定，Explain/AddFeature/Unknown 默认开启，Debug/Refactor/Test/Locate 默认关闭。"
                        },
                        "vectorModel": {
                            "type": "string",
                            "description": "可选。embedding 模型；默认 local-hash-v1。需要真实语义 embedding 时显式传 openai:<embedding模型>、remote:<embedding模型> 或 agent:<embedding模型>，并要求当前用户/环境已配置 API key。"
                        },
                        "searchLimit": {
                            "type": "integer",
                            "description": "符号候选数量。"
                        },
                        "chunkLimit": {
                            "type": "integer",
                            "description": "全文 chunk 候选数量。"
                        },
                        "vectorLimit": {
                            "type": "integer",
                            "description": "向量 chunk 候选数量。"
                        },
                        "vectorBackfillLimit": {
                            "type": "integer",
                            "description": "向量回填 chunk 数量。仅远程 embedding 默认限制为 64，最大 512，防止一次任务产生过多远程请求。"
                        },
                        "depth": {
                            "type": "integer",
                            "description": "符号影响图深度。"
                        },
                        "impactLimit": {
                            "type": "integer",
                            "description": "影响分析最大结果数。"
                        }
                    },
                    "required": ["q"]
                }
            }
        }),
    ]
}

pub(crate) fn is_rag_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        TOOL_CONTEXT_STATUS | TOOL_SYMBOL_SEARCH | TOOL_CONTEXT_TASK_PACK
    )
}

pub(crate) fn execute_rag_tool(
    data_dir: &Path,
    workspace: &Path,
    agent: Option<&AgentConfig>,
    tool_name: &str,
    args: &Value,
    default_trace_id: Option<&str>,
) -> Result<String> {
    let provider_context = agent.map(agent_embedding_context);
    match tool_name {
        TOOL_CONTEXT_STATUS => {
            context_status(data_dir, args, default_trace_id, provider_context.as_ref())
        }
        TOOL_SYMBOL_SEARCH => symbol_search(data_dir, args, default_trace_id),
        TOOL_CONTEXT_TASK_PACK => context_task_pack(
            data_dir,
            workspace,
            args,
            default_trace_id,
            provider_context.as_ref(),
        ),
        _ => bail!("未知 RAG 工具: {tool_name}"),
    }
}

fn context_status(
    data_dir: &Path,
    _args: &Value,
    default_trace_id: Option<&str>,
    provider_context: Option<&SymbolEmbeddingProviderContext>,
) -> Result<String> {
    let Some(trace_id) = effective_trace_id(default_trace_id) else {
        return compact_json(json!({
            "ok": false,
            "indexed": false,
            "embeddingCapabilities": embedding_capabilities(provider_context),
            "message": "当前 API agent 会话缺少 trace_id，无法安全选择项目索引。请先让服务器为本任务运行 context compiler，或改用 list_dir/read_file。"
        }));
    };
    let Some(db_path) = find_symbol_index_db(data_dir, Some(trace_id.as_str())) else {
        return compact_json(json!({
            "ok": false,
            "indexed": false,
            "embeddingCapabilities": embedding_capabilities(provider_context),
            "message": "当前还没有可查询的 symbol_index.sqlite。先运行一次项目开发/预检流程，或先用 list_dir/read_file 做低保真读取。",
            "recommendedNext": ["repo_context_task_pack", "repo_symbol_search", "list_dir", "read_file"]
        }));
    };

    let status = load_latest_symbol_embedding_status(
        data_dir,
        &SymbolEmbeddingStatus {
            trace_id: Some(trace_id.clone()),
            model: Some(LOCAL_HASH_VECTOR_MODEL.to_string()),
            limit: 5,
        },
    );

    match status {
        Ok(status) => compact_json(json!({
            "ok": true,
            "indexed": true,
            "dbPath": status.db_path,
            "metadata": status.metadata,
            "embeddingCapabilities": embedding_capabilities(provider_context),
            "embeddings": {
                "defaultModel": LOCAL_HASH_VECTOR_MODEL,
                "supportedModels": supported_embedding_models(),
                "providerMode": "local_hash",
                "remoteProviderConfigured": provider_context.is_some_and(|context| context.remote_provider_configured()),
                "totals": status.totals,
                "models": status.models,
                "sampleMissingChunks": status.missing_chunks
            },
            "recommendedTools": [
                TOOL_CONTEXT_TASK_PACK,
                TOOL_SYMBOL_SEARCH
            ]
        })),
        Err(error) => {
            let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .with_context(|| format!("打开符号索引数据库失败: {}", db_path.display()))?;
            let metadata = load_metadata(&conn).unwrap_or_default();
            compact_json(json!({
                "ok": true,
                "indexed": true,
                "dbPath": db_path.to_string_lossy().replace('\\', "/"),
                "metadata": metadata,
                "embeddingCapabilities": embedding_capabilities(provider_context),
                "embeddingStatusError": error.to_string(),
                "recommendedTools": [
                    TOOL_CONTEXT_TASK_PACK,
                    TOOL_SYMBOL_SEARCH
                ]
            }))
        }
    }
}

fn symbol_search(data_dir: &Path, args: &Value, default_trace_id: Option<&str>) -> Result<String> {
    let query = required_query(args)?;
    let trace_id = effective_trace_id(default_trace_id).ok_or_else(|| {
        anyhow::anyhow!("当前 API agent 会话缺少 trace_id，无法安全查询项目符号索引")
    })?;
    let response = search_latest_symbol_index(
        data_dir,
        &SymbolIndexSearch {
            trace_id: Some(trace_id),
            text: Some(query),
            kind: string_arg(args, &["kind"]),
            path: string_arg(args, &["path"]),
            edge_kind: string_arg(args, &["edgeKind", "edge_kind"]),
            include_edges: bool_arg(args, &["includeEdges", "include_edges"]).unwrap_or(false),
            limit: usize_arg(args, &["limit"]).unwrap_or_default(),
        },
    )?;
    compact_json(json!({
        "ok": true,
        "result": response
    }))
}

fn context_task_pack(
    data_dir: &Path,
    workspace: &Path,
    args: &Value,
    default_trace_id: Option<&str>,
    provider_context: Option<&SymbolEmbeddingProviderContext>,
) -> Result<String> {
    let vector_policy = task_pack_vector_policy(args)?;
    let query = task_pack_query(args, default_trace_id, provider_context)?;
    let vector_backfill = query
        .vector_model
        .as_ref()
        .map(|model| {
            backfill_latest_symbol_vectors_with_context(
                data_dir,
                &SymbolVectorBackfill {
                    trace_id: query.trace_id.clone(),
                    model: Some(model.clone()),
                    limit: vector_backfill_limit(args, model),
                    force: false,
                },
                provider_context,
            )
        })
        .transpose();
    let vector_backfill = match vector_backfill {
        Ok(value) => value.map(|response| {
            json!({
                "ok": true,
                "model": response.model,
                "dim": response.dim,
                "scannedCount": response.scanned_count,
                "upsertedCount": response.upserted_count,
                "skippedCount": response.skipped_count
            })
        }),
        Err(error) => Some(json!({
            "ok": false,
            "warning": error.to_string()
        })),
    };

    let response = build_latest_symbol_task_pack_with_context(data_dir, &query, provider_context)?;
    let project_docs = load_agent_project_docs(workspace);
    let pack = prepend_project_docs_to_pack(&project_docs, &response.pack);
    let char_count = pack.chars().count();
    let truncated = response.truncated || project_docs.truncated;
    compact_json(json!({
        "ok": true,
        "dbPath": response.db_path,
        "query": response.query,
        "metadata": response.metadata,
        "retrievalPlan": response.retrieval_plan,
        "rankingProfile": response.ranking_profile,
        "chosenSeed": {
            "id": response.chosen_seed.id,
            "name": response.chosen_seed.name,
            "qualifiedName": response.chosen_seed.qualified_name,
            "kind": response.chosen_seed.kind,
            "filePath": response.chosen_seed.file_path,
            "startLine": response.chosen_seed.start_line,
            "endLine": response.chosen_seed.end_line,
            "source": response.chosen_seed_source
        },
        "counts": {
            "candidateSymbols": response.candidate_symbols.len(),
            "textChunks": response.text_chunks.len(),
            "vectorChunks": response.vector_chunks.len(),
            "rankedContext": response.ranked_context.len(),
            "projectDocs": project_docs.included_count,
            "impactedSymbols": response.impacted_symbol_count,
            "impactedFiles": response.impacted_file_count,
            "edges": response.edge_count,
            "testHints": response.test_hint_count
        },
        "charCount": char_count,
        "truncated": truncated,
        "projectDocs": project_docs,
        "vectorPolicy": vector_policy,
        "vectorBackfill": vector_backfill,
        "pack": pack
    }))
}

fn task_pack_query(
    args: &Value,
    default_trace_id: Option<&str>,
    provider_context: Option<&SymbolEmbeddingProviderContext>,
) -> Result<SymbolTaskPackQuery> {
    let vector_policy = task_pack_vector_policy(args)?;
    task_pack_query_with_policy(args, default_trace_id, &vector_policy, provider_context)
}

fn task_pack_vector_policy(args: &Value) -> Result<AgentVectorPolicy> {
    let query = required_query(args)?;
    Ok(choose_agent_vector_policy(
        &query,
        bool_arg(args, &["useVector", "use_vector"]),
        string_arg(args, &["vectorModel", "vector_model"]),
    ))
}

fn task_pack_query_with_policy(
    args: &Value,
    default_trace_id: Option<&str>,
    vector_policy: &AgentVectorPolicy,
    provider_context: Option<&SymbolEmbeddingProviderContext>,
) -> Result<SymbolTaskPackQuery> {
    let vector_model = if let Some(model) = vector_policy.model.as_deref() {
        resolve_embedding_provider(model, provider_context)?;
        Some(model.to_string())
    } else {
        None
    };

    Ok(SymbolTaskPackQuery {
        trace_id: Some(effective_trace_id(default_trace_id).ok_or_else(|| {
            anyhow::anyhow!("当前 API agent 会话缺少 trace_id，无法安全生成项目 RAG 上下文")
        })?),
        text: Some(required_query(args)?),
        kind: string_arg(args, &["kind"]),
        path: string_arg(args, &["path"]),
        edge_kind: string_arg(args, &["edgeKind", "edge_kind"]),
        depth: usize_arg(args, &["depth"]).unwrap_or_default(),
        search_limit: usize_arg(args, &["searchLimit", "search_limit"]).unwrap_or_default(),
        chunk_limit: usize_arg(args, &["chunkLimit", "chunk_limit"]).unwrap_or_default(),
        vector_model,
        vector_limit: usize_arg(args, &["vectorLimit", "vector_limit"]).unwrap_or_default(),
        impact_limit: usize_arg(args, &["impactLimit", "impact_limit"]).unwrap_or_default(),
        max_chars: usize_arg(args, &["maxChars", "max_chars"]).unwrap_or_default(),
    })
}

fn effective_trace_id(default_trace_id: Option<&str>) -> Option<String> {
    default_trace_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn required_query(args: &Value) -> Result<String> {
    string_arg(args, &["q", "query"]).ok_or_else(|| anyhow::anyhow!("q 不能为空"))
}

fn string_arg(args: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|name| args.get(*name))
        .find_map(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn bool_arg(args: &Value, names: &[&str]) -> Option<bool> {
    names
        .iter()
        .filter_map(|name| args.get(*name))
        .find_map(|value| value.as_bool())
}

fn usize_arg(args: &Value, names: &[&str]) -> Option<usize> {
    names
        .iter()
        .filter_map(|name| args.get(*name))
        .find_map(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
}

fn vector_backfill_limit(args: &Value, model: &str) -> usize {
    let requested = usize_arg(args, &["vectorBackfillLimit", "vector_backfill_limit"])
        .filter(|value| *value > 0);
    if !is_remote_embedding_model(model) {
        return requested.unwrap_or_default();
    }

    requested
        .unwrap_or(DEFAULT_REMOTE_VECTOR_BACKFILL_LIMIT)
        .min(MAX_REMOTE_VECTOR_BACKFILL_LIMIT)
}

fn supported_embedding_models() -> Vec<&'static str> {
    SUPPORTED_EMBEDDING_MODELS.to_vec()
}

fn agent_embedding_context(agent: &AgentConfig) -> SymbolEmbeddingProviderContext {
    SymbolEmbeddingProviderContext::from_agent(
        &agent.api_base,
        &agent.api_key,
        agent.usage_mode().to_string(),
    )
}

fn embedding_capabilities(provider_context: Option<&SymbolEmbeddingProviderContext>) -> Value {
    let remote_provider_configured =
        provider_context.is_some_and(|context| context.remote_provider_configured());
    let remote_provider_source =
        provider_context.and_then(|context| context.remote_provider_source());
    json!({
        "defaultModel": LOCAL_HASH_VECTOR_MODEL,
        "supportedModels": supported_embedding_models(),
        "providerMode": if remote_provider_configured { "local_hash_plus_openai_compatible" } else { "local_hash" },
        "remoteProviderConfigured": remote_provider_configured,
        "remoteProviderSource": remote_provider_source,
        "remoteModelSyntax": ["openai:<embedding-model>", "remote:<embedding-model>", "agent:<embedding-model>"],
        "message": if remote_provider_configured {
            "默认使用本地 hash embedding；显式传 vectorModel=openai:<embedding模型> 可使用当前用户/环境的 OpenAI-compatible embedding provider。"
        } else {
            "默认只有本地 hash embedding；显式远程 embedding 需要用户 API key 或 ELON_EMBEDDING_API_KEY。"
        }
    })
}

fn compact_json(value: Value) -> Result<String> {
    serde_json::to_string(&value).context("序列化 RAG 工具结果失败")
}

#[cfg(test)]
mod tests {
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
}
