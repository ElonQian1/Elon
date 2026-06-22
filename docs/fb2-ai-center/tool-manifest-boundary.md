# fb2 Tool Manifest 边界

本文件固定主项目和 fb2 之间的工具发现边界，避免把 Context Pack、聊天自动工具、质量诊断端点和反馈写回端点混成同一种能力。

## 结论

- `tool_ids` 表示 fb2 实时 manifest 暴露的能力全集摘要，不等于主项目聊天 AI 都会自动执行。
- `chat_auto_executable_tool_ids` 才表示主项目群聊 AI 可以在回答中自动规划并通过 `/api/main-project/tools/execute` 调用的业务工具。
- `manifest_only_tool_ids` 表示已发现但不自动执行的能力，通常是回调、诊断、刷新、管理或后续接入候选。
- `integration_only_endpoints` 表示主项目 smoke 或后台巡检直接读取的受保护 HTTP 端点，不要求出现在聊天工具 ID 列表中。

## 工具分层

### 1. 聊天自动工具

这些工具可以进入主项目聊天 planner，但必须遵守 fb2 manifest 的权限、scope 和返回 grounding 规则：

- `match_analysis_brief`
- `group_opinion_summary`
- `search_matches`
- `get_match_detail`
- `search_user_orders`
- `get_order_detail`
- `search_group_opinions`
- `opinion_memories`
- `list_opinion_adoptions`
- `opinion_adoption_summary`
- `opinion_result_reviews`
- `opinion_result_review_summary`
- `platform_orders`
- `get_context_audit`
- `context_audit_summary`

其中用户订单工具必须带 `external_user_id` 和同值 `X-FB2-AI-CONTEXT-USER-ID`；平台订单工具必须带 `X-FB2-AI-CONTEXT-SCOPE: platform_order_summary`。

### 2. Manifest-only 能力

以下能力可以被 manifest 暴露，供主项目发现、审计或后续接入，但默认不进入聊天自动 planner：

- `record_context_feedback`
- `list_context_feedbacks`
- `context_feedback_summary`
- `record_opinion_adoption`
- `refresh_opinion_result_reviews`
- 管理类刷新、批处理或写入类工具

主项目可以在回答后由后处理流程调用 feedback/adoption 写回，但这不是模型自由选择的聊天工具。

### 3. Integration-only 端点

这些端点是主项目 smoke、最终验收或后台巡检直接读取的受保护 HTTP 端点。它们可以在 fb2 内部也有 tools/execute 包装，但主项目不要求它们成为聊天自动 tool id：

- `/api/main-project/context/readiness`
- `/api/main-project/context/tool-manifest`
- `/api/main-project/context/feedback-summary`
- `/api/main-project/context/feedbacks`
- `/api/main-project/context/quality-summary`
- `/api/main-project/context/permission-summary`

因此 `context_quality_summary`、`context_permission_summary` 如果出现在 fb2 `/tools/execute`，主项目应视为诊断/manifest-only 能力；如果它们没有出现在 live tool ids，但 `/integration` 和受保护 HTTP 端点可用，也不应判定为聊天工具缺失。

## answer_policy 读取位置

主项目读取 answer policy 的优先级固定为：

1. `data.answer_policy`
2. `data.tool_contract.answer_policy`
3. 主项目 `/api/external/apps/fb2/context-contract` 的默认 `answer_policy_contract`

fb2 当前可以把 answer policy 放在 `tool_contract.answer_policy` 下；只要 Context Pack 和工具结果能支撑主项目默认回答规则，就不阻塞 data-only 验收。

## source registry 与质量闭环

`citation_sources` / `source_registry` 用于回答引用、权限审计和 feedback 匹配。业务事实类来源应尽量进入 source registry：

- `context_audit`
- `match`
- `odds`
- `user_order`
- `ticket`
- `group_message`
- `opinion_memory`
- `platform_order_summary`

`feedback` 和 `opinion_adoption` 是质量闭环路线，不要求每次 Context Pack 都作为可引用业务事实 source kind 输出。它们必须通过以下路径可查询和可审计：

- feedback 写回：`record_context_feedback`
- feedback 查询：`list_context_feedbacks`、`feedback-summary`
- 观点采纳写回：`record_opinion_adoption`
- 观点采纳汇总：`opinion-adoption-summary`
- 质量汇总：`quality-summary`

如果后续 fb2 想把历史 feedback 或观点采纳样本作为模型可引用事实使用，应单独给出 `kind=feedback` 或 `kind=opinion_adoption` 的 source registry 条目，并标明 `scope=quality_history`，避免把质量指标误当比赛事实。

## 工具结果信封

主项目执行 fb2 工具后，不把原始工具响应直接塞进 prompt，而是先归一化为 `external_app.normalized_tool_result.v1`。`/api/external/apps/fb2/context-contract` 的 `tool_result_envelope_contract` 固定该信封：

- `schema=fb2.tool_result_envelope.v1`
- 必需字段：`schema`、`tool_name`、`request_id`、`status`、`success`、`data`、`error`、`generated_at`、`source_ids`、`visibility`、`metrics`、`grounding`、`reason`
- `grounding.status=grounded/weak` 时可作为事实，其中 `weak` 必须说明证据缺口
- `grounding.status=unsafe/unavailable` 时不能作为事实
- `source_registry.business_source_kinds` 只包含业务事实来源；`quality_history_kinds` 只表示历史反馈/采纳记录

feedback 回写只能采用 AI 回复正文显式提到的 `source_ids`，且工具结果必须 `success=true`、`grounding.status=grounded/weak`。未被回答提到、权限 visibility 不匹配或工具失败的 source id 不能写回 fb2 cited sources。

## 主项目验收口径

- live manifest 漂移检查关注 `chat_auto_executable_tool_ids` 和主项目静态 allowlist 是否对齐。
- `/integration` 负责证明 integration-only 端点存在。
- `-CheckQuality`、`-CheckPermissionBoundaries`、`-RequireNonSyntheticQualityReadiness` 负责证明受保护质量/权限端点可读并有审计数据。
- 最终 summary 的 `feedback_coverage` 和 `visible_direct_read_evidence` 负责证明真实群聊回答、回读和质量写回已经进入同一批验收证据。
