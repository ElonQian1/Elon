# Elon AI Index

本文件是给 AI 的高信噪比入口索引。需要规则时先读 `AGENTS.md`；判断当前实现、提案或禁用路线时先读 `AI_CURRENT.md`；需要项目事实时读 `AI_PROJECT.md`；需要架构分层时读 `AI_ARCHITECTURE.md`；需要具体实现时再按本文件定位源码。

## 后端核心入口

| 领域 | 入口 |
|---|---|
| Rust 服务启动和路由 | `server/src/main.rs` |
| 项目 API / 项目空间 | `server/src/project_api.rs` 及同领域拆分模块 |
| AI CLI 调度 | `server/src/ai_cli/`、`server/src/agent.rs` |
| Codex 桌面监督、一龙 PC 执行、用户任务与平台改进双闭环（自 2026-07-26 起暂停） | `docs/supervised-pc-project-development.md`、`docs/system-architecture.md` 的“PC 节点 AI 运行路线” |
| PC 节点 AI 三层架构 / Codex JSON、pipe sidecar 与 PTY 分工 | `AI_ARCHITECTURE.md` 的“PC 节点 AI 运行路线”、`docs/符号索引讨论/我们项目的cli能力.md` |
| Codex 桌面监督 / PC 本机执行 / 验收、能力修复与续跑（自 2026-07-26 起暂停） | `docs/codex-desktop-pc-supervision.md`、`.agents/skills/codex-pc-supervisor/`、`server/src/node_agent_local_task_supervision.rs` |
| Codex 桌面低 token 增量 Wait / Resume 上下文 / 终态 / A/B 度量 | `docs/codex-desktop-workflow-efficiency.md`、`server/src/node_agent_supervision_protocol.rs`、`scripts/compare-ai-workflow-efficiency.ps1` |
| Win 节点轻量工具箱 / Codex CLI 临时 PATH / 工具收录策略 | `docs/win-node-toolbox.md`、`server/src/node_agent_cli_env.rs`、`server/src/node_agent_cli_tool_catalog.rs` |
| PC 节点项目数据架构体检 / 共享缓存分析 / 渐进治理 | `docs/pc-node-data-root.md`、`server/pc-dev-runtime/src/node_data_paths.rs`、`server/src/node_agent_data_root/`、`server/src/node_agent_cache_advisor.rs` |
| Windows 节点升级兼容 / 自动迁移 / 灰度 / 事故处置 | `docs/node-agent-upgrade-compatibility.md` |
| PWA 真实无头像素捕获 / `yilong_ui_live` MCP / route-source-PNG 验证 | `docs/system-architecture.md` 的“PWA Runtime 像素证据”、`server/src/node_agent_pwa_runtime/`、`server/src/node_agent_source_preview/pwa_runtime.rs`、`pc-frontend/src/features/ui-tuner/source-preview/` |
| Web/PWA/Tauri/Android 后台设计 / revisioned DesignIntentPlan、task lease、动作回执、暂停/重规划 / 事件 checkpoint / DraftOperation v2 / 绑定漂移健康 / 分平台写回审批 / 确定性 source patch、回滚审查 / 节点本机视觉语义回归比较 / Tauri 分层证据 / PC 边聊边看 | `docs/headless-ui-design-mcp.md`、`server/src/node_agent_android_live/design_intent_plan.rs`、`server/src/node_agent_android_live/design_intent_execution.rs`、`server/src/node_agent_android_live/design_task_binding.rs`、`server/src/node_agent_android_live/design_event_stream.rs`、`server/src/node_agent_android_live/design_event_checkpoint.rs`、`server/src/node_agent_android_live/design_draft_operations.rs`、`server/src/node_agent_android_live/design_source_binding.rs`、`server/src/node_agent_android_live/design_binding_health.rs`、`server/src/node_agent_android_live/design_writeback_plan.rs`、`server/src/node_agent_android_live/design_source_patch.rs`、`server/src/node_agent_android_live/design_regression_contract.rs`、`server/src/node_agent_android_live/design_regression_runner.rs`、`server/src/node_agent_android_live/design_verification_matrix.rs`、`server/src/node_agent_android_live/tauri_host_runtime.rs`、`server/src/node_agent_android_live/design_drafts.rs`、`pc-frontend/src/features/ui-tuner/headless-design/` |
| 项目知识首页 / 产品功能图 / 技术架构图 / 主题树 / 讨论推理图 / 独立治理属性 / 低 token MCP / 跨 PC 项目记忆 | `docs/README.md`、`.github/instructions/document-authority.instructions.md`、`docs/project-document-governance-mcp.md`、`docs/project-memory-agent-integration.md`、`docs/discussion-knowledge-compiler.md`、`plugins/yilong-project-memory/`、`scripts/project-memory-ci.ps1`、`scripts/project-memory-app-server-observer.mjs`、`pc-frontend/src/features/project-docs/`、`server/src/project_document_knowledge_graph*.rs`、`server/src/project_discussion_graph*.rs`、`server/src/project_document_governance*.rs`、`server/src/project_document_native_context*.rs`、`server/src/node_agent_project_docs_mcp*.rs` |
| AI 原生开放商业网络 V1 / 商户节点、能力、授权、调用、计量、审计和 MCP | `docs/decisions/open-commerce-network-v1-architecture.md`、`docs/open-commerce-network-v1-api.md`、`server/src/open_commerce_*.rs`、`server/src/store/open_commerce_*.rs`、`pc-frontend/src/features/open-commerce/` |
| 开放商业能力契约强制执行 / 输入输出 Schema / 422 / 零金额失败 | `docs/decisions/open-commerce-capability-contract-enforcement-v1.md`、`server/src/open_commerce_capability_schema.rs`、`server/src/open_commerce_capability_contract_service.rs`、`docs/open-commerce-capability-contract-enforcement-v1-acceptance.md` |
| 消费者 PC Schema 调用表单 / 可选值省略 / 动作确认 | `docs/decisions/open-commerce-schema-driven-invocation-form-v1.md`、`pc-frontend/src/features/open-commerce/capabilityInvocationSchema.ts`、`pc-frontend/src/features/open-commerce/CapabilitySchemaField.tsx`、`pc-frontend/src/features/open-commerce/CapabilityInvocationComposer.tsx` |
| 服务端动作确认 / 5 分钟凭证 / 本人状态恢复与主动取消 / 输入与幂等绑定 / 原子一次消费 | `docs/decisions/open-commerce-server-action-confirmation-v1.md`、`server/src/open_commerce_action_confirmation_*.rs`、`server/src/store/open_commerce_action_confirmations.rs`、`docs/open-commerce-server-action-confirmation-v1-acceptance.md` |
| 消费者账户级调用凭证 / 本人终态结果 / SHA-256 下载复核 | `docs/decisions/open-commerce-consumer-invocation-receipts-v1.md`、`server/src/open_commerce_consumer_receipt_service.rs`、`server/src/open_commerce_consumer_receipt_api.rs`、`server/src/open_commerce_consumer_receipt_mcp.rs`、`pc-frontend/src/features/open-commerce/ConsumerInvocationReceipts.tsx` |
| 开发者 App 终态调用事件流 / App 绑定游标 / 结果断点续读 | `docs/decisions/open-commerce-developer-terminal-events-v1.md`、`server/src/open_commerce_developer_event_*.rs`、`server/src/store/open_commerce_developer_events.rs`、`pc-frontend/src/features/open-commerce/DeveloperInvocationEvents.tsx` |
| 开发者 App 签名 Webhook / 耐久投递 / 重试与死信 / SDK 验签 | `docs/decisions/open-commerce-developer-webhooks-v1.md`、`server/src/open_commerce_webhook_*.rs`、`server/src/store/open_commerce_developer_webhooks.rs`、`pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx`、`sdk/open-commerce-connector/src/webhook-signature.js` |
| Webhook 回调控制验证 / challenge 回显 / 验证后激活 | `docs/decisions/open-commerce-developer-webhook-verification-v1.md`、`server/src/open_commerce_webhook_verification*.rs`、`pc-frontend/src/features/open-commerce/DeveloperWebhookPanel.tsx` |
| Webhook 单订阅密钥轮换 / 版本并发保护 / 重新验证 | `docs/decisions/open-commerce-developer-webhook-secret-rotation-v1.md`、`server/src/open_commerce_webhook_lifecycle_migration.rs`、`server/src/store/open_commerce_developer_webhook_secret.rs` |
| Webhook 死信人工重试 / 原投递重新排队 / 重试轮次证据 | `docs/decisions/open-commerce-developer-webhook-dead-letter-retry-v1.md`、`server/src/open_commerce_webhook_replay_migration.rs`、`server/src/store/open_commerce_developer_webhook_replays.rs` |
| Webhook 成功/失败事件筛选 / 触发器入队前过滤 | `docs/decisions/open-commerce-developer-webhook-event-filter-v1.md`、`server/src/open_commerce_webhook_event_filter_migration.rs`、`server/src/store/open_commerce_developer_webhook_rows.rs` |
| Webhook 有界历史补发 / 确定性投递去重 / 补发来源证据 | `docs/decisions/open-commerce-developer-webhook-history-replay-v1.md`、`server/src/open_commerce_webhook_history_migration.rs`、`server/src/store/open_commerce_developer_webhook_history.rs` |
| 开发者 App 资料清单 / 修订提交 / 平台审核 / 非生产准入 | `docs/decisions/open-commerce-developer-app-manifest-review-v1.md`、`server/src/open_commerce_developer_manifest_*.rs`、`server/src/store/open_commerce_developer_app_manifests.rs`、`pc-frontend/src/features/open-commerce/DeveloperAppManifestPanel.tsx` |
| 开发者 App 主页域名控制证明 / well-known challenge / 精确白名单 | `docs/decisions/open-commerce-developer-app-domain-verification-v1.md`、`server/src/open_commerce_developer_domain_*.rs`、`server/src/store/open_commerce_developer_app_domains.rs`、`pc-frontend/src/features/open-commerce/DeveloperAppManifestPanel.tsx` |
| 开发者 App 可撤销准入审查 / 主体声明 / 人工风险层级 / 紧急暂停 | `docs/decisions/open-commerce-developer-app-admission-v1.md`、`server/src/open_commerce_developer_admission_*.rs`、`server/src/store/open_commerce_developer_app_admissions.rs`、`pc-frontend/src/features/open-commerce/DeveloperAppAdmission*.tsx` |
| 开发者 App 限权生产凭据 / 一次性密钥 / 范围与期限 / 紧急撤销 | `docs/decisions/open-commerce-developer-production-credentials-v1.md`、`server/src/open_commerce_developer_credential_*.rs`、`server/src/store/open_commerce_developer_credentials.rs`、`pc-frontend/src/features/open-commerce/DeveloperProductionCredentialPanel.tsx` |
| 开放商业调用凭据来源 / 沙箱与生产隔离 / 幂等与事件边界 | `docs/decisions/open-commerce-invocation-credential-provenance-v1.md`、`server/src/open_commerce_invocation_service.rs`、`server/src/store/open_commerce_invocations.rs`、`docs/open-commerce-invocation-credential-provenance-v1-acceptance.md` |
| 开放商业环境绑定生产 Webhook / 双开关 / 生产资格联动 / 环境一致投递 | `docs/decisions/open-commerce-production-webhooks-v1.md`、`server/src/open_commerce_production_webhook*.rs`、`server/src/store/open_commerce_production_webhooks.rs`、`docs/open-commerce-production-webhooks-v1-acceptance.md` |
| 开发者 App 生产就绪总览 / 固定阻断顺序 / 调用与通知独立结论 | `docs/decisions/open-commerce-developer-production-readiness-v1.md`、`server/src/open_commerce_developer_readiness_*.rs`、`pc-frontend/src/features/open-commerce/DeveloperProductionReadinessPanel.tsx` |
| 开放商业 Webhook 运行健康 / 环境聚合 / 积压死信 / 生产阻断码 | `docs/decisions/open-commerce-webhook-operational-health-v1.md`、`server/src/open_commerce_webhook_health_*.rs`、`server/src/store/open_commerce_developer_webhook_health.rs`、`pc-frontend/src/features/open-commerce/DeveloperWebhookHealthSummary.tsx` |
| 开放商业 Webhook 死信确认 / 不可覆盖处理证据 / 健康告警收口 | `docs/decisions/open-commerce-webhook-dead-letter-acknowledgement-v1.md`、`server/src/open_commerce_webhook_dead_letter_*.rs`、`server/src/store/open_commerce_developer_webhook_dead_letters.rs`、`pc-frontend/src/features/open-commerce/DeveloperWebhookDeadLetterActions.tsx` |
| 开放商业 HTTPS 出站公网地址固定 / DNS 私网拒绝 / 禁代理 / 防重绑定 | `docs/decisions/open-commerce-outbound-public-address-pinning-v1.md`、`server/src/open_commerce_outbound_security.rs`、`server/src/open_commerce_webhook_verification.rs`、`server/src/open_commerce_webhook_worker.rs` |
| 受控商户运行时 / 商户自有 ERP / HMAC / Manifest / 报价下单闭环 | `docs/open-commerce/merchant-runtime.md`、`docs/decisions/open-commerce-merchant-runtime-v1.md`、`server/src/open_commerce_runtime_*.rs`、`server/src/store/open_commerce_runtime_bindings.rs`、`contracts/open-commerce/merchant-runtime-v1.json` |
| 商户运行时公网地址固定 / 调用时白名单复核 / DNS 私网拒绝 | `docs/decisions/open-commerce-merchant-runtime-egress-pinning-v1.md`、`server/src/open_commerce_runtime_security.rs`、`server/src/open_commerce_runtime_client.rs`、`server/src/open_commerce_outbound_security.rs` |
| 商户业务调用证据 / 标准业务回执 / ERP 实例关联 / ERP-CRM 入库边界 | `docs/decisions/open-commerce-merchant-business-evidence-v1.md`、`server/src/open_commerce_merchant_evidence_*.rs`、`server/src/store/open_commerce_merchant_evidence.rs`、`pc-frontend/src/features/open-commerce/MerchantBusinessEvidencePanel.tsx` |
| 通用 ERP 蓝图 / 版本能力目录 / 独立商户配置 / 初始化 Matter / 物化合同与状态 / AI 通用提案 / 证据化升级与完整回滚 | `docs/erp/README.md`、`docs/erp/operating-workflow-v1.md`、`docs/erp/api-and-agent-tools-v1.md`、`server/src/erp_blueprint/`、`server/src/erp_blueprint_api.rs`、`server/src/erp_blueprint_mcp*.rs`、`server/src/store/erp_*.rs`、`contracts/erp/`、`pc-frontend/src/features/open-commerce/erp-blueprint/` |
| 消费者发现 / 候选范围摘要 / 用户选择透明非付费排序器 / 商户声明来源与新鲜度 / 内部同步回执来源绑定 / 消费者来源厂商、数据域、最大回执年龄及候选建议 / 币种安全价格筛选 / 能力类型与访问级别筛选 / 偏好硬约束 / 消费者声明期筛选 / 可复核未签名单次排序凭证 / 第三方应用沙盒 / 一次性测试 Token / 授权审批与 Grant 总预算 | `docs/open-commerce/consumer-developer-sandbox.md`、`docs/decisions/open-commerce-consumer-candidate-scope-v1.md`、`docs/decisions/open-commerce-pluggable-ranking-v1.md`、`docs/decisions/open-commerce-public-data-provenance-v1.md`、`docs/decisions/open-commerce-capability-source-link-v1.md`、`docs/decisions/open-commerce-consumer-source-requirement-v1.md`、`docs/decisions/open-commerce-consumer-source-filters-v1.md`、`docs/decisions/open-commerce-consumer-source-age-v1.md`、`docs/decisions/open-commerce-consumer-source-filter-options-v1.md`、`docs/decisions/open-commerce-consumer-price-currency-v1.md`、`docs/decisions/open-commerce-consumer-capability-filters-v1.md`、`docs/decisions/open-commerce-consumer-preference-constraints-v1.md`、`docs/decisions/open-commerce-consumer-freshness-filter-v1.md`、`docs/decisions/open-commerce-consumer-ranking-receipts-v1.md`、`docs/open-commerce-consumer-candidate-scope-v1-acceptance.md`、`docs/open-commerce-pluggable-ranking-v1-acceptance.md`、`docs/open-commerce-public-data-provenance-v1-acceptance.md`、`docs/open-commerce-capability-source-link-v1-acceptance.md`、`docs/open-commerce-consumer-source-requirement-v1-acceptance.md`、`docs/open-commerce-consumer-source-filters-v1-acceptance.md`、`docs/open-commerce-consumer-source-age-v1-acceptance.md`、`docs/open-commerce-consumer-source-filter-options-v1-acceptance.md`、`docs/open-commerce-consumer-price-currency-v1-acceptance.md`、`docs/open-commerce-consumer-capability-filters-v1-acceptance.md`、`docs/open-commerce-consumer-preference-constraints-v1-acceptance.md`、`docs/open-commerce-consumer-freshness-filter-v1-acceptance.md`、`docs/open-commerce-consumer-ranking-receipts-v1-acceptance.md`、`docs/decisions/open-commerce-grant-budgets-v1.md`、`server/src/open_commerce_capability_source_*.rs`、`server/src/store/open_commerce_capability_sources.rs`、`server/src/open_commerce_directory_model.rs`、`server/src/open_commerce_consumer_ranking.rs`、`server/src/open_commerce_consumer_source_options.rs`、`server/src/open_commerce_consumer_constraints.rs`、`server/src/open_commerce_consumer.rs`、`server/src/open_commerce_client_*.rs`、`server/src/open_commerce_grant_budget_*.rs`、`pc-frontend/src/features/open-commerce/consumerRankingReceipt.ts`、`ConsumerCandidateScopeSummary.tsx`、`ConsumerSourceFilterFields.tsx`、`ConsumerPriceFilterFields.tsx`、`ConsumerCapabilityFilterFields.tsx`、`ConsumerPreferenceConstraintFields.tsx`、`ConsumerCommerceSandbox.tsx`、`DeveloperCommercePortal.tsx` |
| 消费者发现请求规范化 / 第三方 App 输入边界 | `docs/decisions/open-commerce-consumer-discovery-inputs-v1.md`、`docs/open-commerce-consumer-discovery-inputs-v1-acceptance.md`、`server/src/open_commerce_consumer.rs`、`server/src/store/open_commerce_directory.rs` |
| 消费者 AI 完整发现 MCP / 默认公开身份 / 显式 App 身份 | `docs/decisions/open-commerce-consumer-discovery-mcp-v1.md`、`docs/open-commerce-consumer-discovery-mcp-v1-acceptance.md`、`server/src/open_commerce_consumer_discovery_mcp.rs`、`server/src/open_commerce_mcp.rs` |
| 消费者能力执行计划 / 输入预检 / 多 Grant 预算选择 / 授权与动作下一步 | `docs/decisions/open-commerce-consumer-capability-execution-plan-v1.md`、`docs/open-commerce-consumer-capability-execution-plan-v1-acceptance.md`、`server/src/open_commerce_consumer_execution_plan.rs`、`server/src/open_commerce_grant_readiness.rs`、`server/src/open_commerce_consumer_discovery_mcp.rs` |
| 消费者 AI 单能力授权与续额申请 / 用户确认 / 商户人工决定 | `docs/decisions/open-commerce-consumer-authorization-mcp-v1.md`、`docs/open-commerce-consumer-authorization-mcp-v1-acceptance.md`、`server/src/open_commerce_consumer_discovery_mcp.rs`、`server/src/store/open_commerce_authorization_requests.rs`、`server/src/store/open_commerce_grants.rs` |
| 消费者 AI 本人 App 目录 / MCP 身份选择 / 无 Token 响应 | `docs/decisions/open-commerce-consumer-app-directory-mcp-v1.md`、`docs/open-commerce-consumer-app-directory-mcp-v1-acceptance.md`、`server/src/open_commerce_consumer_app_mcp.rs`、`server/src/open_commerce_mcp_protocol.rs` |
| 消费者本人授权申请、当前有效 Grant 与 pending 撤回 / 用户项目隔离 / 剩余预算 | `docs/decisions/open-commerce-consumer-authorization-status-mcp-v1.md`、`docs/open-commerce-consumer-authorization-status-mcp-v1-acceptance.md`、`server/src/open_commerce_consumer_authorization_mcp.rs`、`server/src/store/open_commerce_authorization_requests.rs`、`server/src/store/open_commerce_grants.rs` |
| 消费者偏好档案 / 关系级字段披露 / 商户匿名偏好收件箱 | `docs/decisions/open-commerce-consumer-preference-disclosures-v1.md`、`server/src/open_commerce_consumer_preference_*.rs`、`server/src/store/open_commerce_consumer_preferences.rs`、`pc-frontend/src/features/open-commerce/ConsumerPreferenceProfilePanel.tsx`、`MerchantPreferenceInbox.tsx` |
| 消费者客户端加密敏感数据保险箱 / 密文修订 / 本人下载与删除 | `docs/decisions/open-commerce-consumer-data-vault-v1.md`、`docs/open-commerce-consumer-data-vault-v1-acceptance.md`、`server/src/open_commerce_consumer_vault_*.rs`、`server/src/store/open_commerce_consumer_vault.rs`、`pc-frontend/src/features/open-commerce/consumerDataVaultCrypto.ts`、`ConsumerDataVaultPanel.tsx` |
| 消费者关联数据删除请求 / 有界催办与升级关注 / 商户完成声明 / 追加式未核验外部证明 | `docs/decisions/open-commerce-consumer-data-erasure-requests-v1.md`、`docs/decisions/open-commerce-consumer-data-request-followups-v1.md`、`docs/decisions/open-commerce-consumer-data-erasure-evidence-v1.md`、`server/src/open_commerce_data_request_*.rs`、`server/src/open_commerce_data_erasure_evidence_*.rs`、`server/src/store/open_commerce_data_request_followups.rs`、`server/src/store/open_commerce_data_erasure_evidence.rs`、`pc-frontend/src/features/open-commerce/ConsumerDataRequestManager.tsx`、`MerchantDataRequestInbox.tsx` |
| 消费者可携带数据包 V5 / 删除证明 / 商户身份声明 / 隔离导入 / 运营方 RSA 签名 / 单包字段级采用 / 多来源偏好冲突选择与回滚 / 本地口令加密归档 / 关系映射与重新授权 | `docs/decisions/open-commerce-consumer-portability-exports-v5.md`、`docs/decisions/open-commerce-consumer-portability-exports-v4.md`、`docs/decisions/open-commerce-consumer-portability-imports-v1.md`、`docs/decisions/open-commerce-consumer-portability-trust-v1.md`、`docs/decisions/open-commerce-consumer-portability-adoption-v1.md`、`docs/decisions/open-commerce-consumer-portability-selective-adoption-v1.md`、`docs/decisions/open-commerce-consumer-portability-multi-source-merge-v1.md`、`docs/decisions/open-commerce-consumer-portability-encrypted-archives-v1.md`、`docs/decisions/open-commerce-consumer-portability-reauthorization-v1.md`、`server/src/open_commerce_portability_*.rs`、`server/src/store/open_commerce_consumer_portability*.rs`、`server/src/store/open_commerce_portability_reauthorization.rs`、`sdk/open-commerce-connector/src/portability-signature.js`、`portability-archive.js`、`pc-frontend/src/features/open-commerce/ConsumerPortabilityExports.tsx`、`ConsumerPortabilityImports.tsx`、`ConsumerPortabilityTrustKeys.tsx`、`ConsumerPortabilityAdoptions.tsx`、`ConsumerPortabilityMergePanel.tsx`、`ConsumerPortabilityReauthorization.tsx`、`portabilityArchive.ts` |
| 商户可携带身份 / RSA 私钥持有证明 / 公开目录指纹 | `docs/decisions/open-commerce-merchant-portable-identity-v1.md`、`server/src/open_commerce_merchant_identity_*.rs`、`server/src/store/open_commerce_merchant_identity.rs`、`pc-frontend/src/features/open-commerce/MerchantPortableIdentityPanel.tsx`、`sdk/open-commerce-connector/src/merchant-identity.js` |
| 项目 AI 资源控制面 / 资源策略 / 路由预演 | `docs/open-commerce/ai-resource-control.md`、`server/src/ai_resource_control/`、`pc-frontend/src/features/open-commerce/AiResourceControlPanel.tsx` |
| 节点模型算力显式共享 / 模型白名单 / 并发与日阈值 / 原子调度 / 流租约 | `docs/decisions/node-compute-sharing-supply-v1.md`、`docs/node-compute-sharing-supply-v1-api.md`、`server/src/node_compute_sharing.rs`、`server/src/node_llm_stream.rs`、`server/src/store/node_compute_sharing.rs`、`pc-frontend/src/features/node/NodeComputeSharingCard.tsx` |
| 任务级分布式算力联邦 / Provider、Offer、Job、Reservation、Attempt、验证回执与 CNY 账本 / `implementation_uncompiled` v169-v201 注册表、可信结算、挑战纠正、available 释放、到期释放批处理、Provider 提款、终态管理及 Provider/平台账户审计视图 | `docs/distributed-compute/README.md`、`docs/distributed-compute/market-and-settlement.md`、`docs/distributed-compute/attempt-settlement-release-api.md`、`docs/distributed-compute/settlement-release-batch-api.md`、`docs/distributed-compute/settlement-withdrawal-request-api.md`、`docs/distributed-compute/settlement-withdrawal-terminal-api.md`、`docs/distributed-compute/settlement-account-view-api.md`、`docs/decisions/distributed-compute-federation-v1.md`、`server/src/compute_federation/`、`server/src/store/compute_attempt_settlement_releases.rs`、`server/src/store/compute_settlement_release_candidates.rs`、`server/src/store/compute_settlement_withdrawal_requests.rs`、`server/src/store/compute_settlement_withdrawal_terminals.rs`、`server/src/store/compute_settlement_account_views.rs`、`server/src/store/compute_platform_settlement_account_view.rs`、`pc-frontend/src/features/compute-settlement/`、`server/src/node_agent_compute_plugin_host/` |
| 分布式算力共享 CapacityPool / 追加式容量账本 / Store-canonical 请求摘要、供给、Claim、审计、恢复、生命周期与 epoch 轮换 / 未编译 v165、v167-v168 Store | `docs/decisions/distributed-compute-capacity-ledger-v1.md`、`docs/distributed-compute/capacity-ledger.md`、`docs/distributed-compute/architecture.md`、`docs/distributed-compute/market-and-settlement.md`、`server/src/compute_federation/capacity.rs`、`server/src/compute_federation/capacity/reducer.rs`、`server/src/compute_capacity_migration.rs`、`server/src/store/compute_capacity_request_digest.rs`、`server/src/store/compute_capacity_claims.rs`、`server/src/store/compute_capacity_claim_transitions.rs`、`server/src/store/compute_capacity_ledger.rs`、`server/src/store/compute_capacity_audit.rs`、`server/src/store/compute_capacity_expiry_recovery.rs`、`server/src/store/compute_capacity_pool_lifecycle.rs`、`server/src/store/compute_capacity_pool_epoch.rs` |
| AI 任务与开放商业链外影子结算 / 双分录 / 争议纠正 / 有效凭证 / Sui 链下投影包 | `docs/decisions/task-shadow-settlement-v1.md`、`docs/decisions/task-shadow-settlement-disputes-v1.md`、`docs/decisions/task-shadow-settlement-corrections-v1.md`、`docs/decisions/task-shadow-settlement-lineage-v1.md`、`docs/decisions/sui-offchain-projection-packages-v1.md`、`docs/decisions/sui-correction-projection-packages-v1.md`、`docs/task-shadow-settlement-v1-api.md`、`server/src/task_settlement/lineage_service.rs`、`server/src/store/task_settlement_corrections.rs`、`server/src/store/task_sui_projection_packages.rs`、`server/src/store/task_sui_correction_projection_packages.rs` |
| Sui 离线适配器交接 / 标准与纠正统一契约 / 重新复核 / 零网络副作用 | `docs/decisions/sui-adapter-offline-handoff-v1.md`、`server/src/task_settlement/sui_adapter_handoff_*.rs`、`pc-frontend/src/features/open-commerce/suiAdapterHandoffDownload.ts` |
| Sui 离线预检适配器 / 显式任务 / 短时租约 / 摘要漂移阻断 / 追加式报告 / 参考 CLI | `docs/decisions/sui-offline-preflight-adapters-v1.md`、`docs/decisions/sui-offline-preflight-job-leases-v1.md`、`server/src/task_settlement/sui_preflight_*.rs`、`server/src/store/task_sui_preflight_*.rs`、`pc-frontend/src/features/open-commerce/SuiPreflightAdaptersPanel.tsx`、`SuiPreflightJobsPanel.tsx`、`sdk/open-commerce-connector/src/sui-preflight.js`、`sdk/open-commerce-connector/bin/sui-preflight.mjs` |
| 开放商业能力包 / 现有能力、群体 AI、共享节点、Sui 提案和决策状态 | `docs/open-commerce/README.md`、`docs/open-commerce/capability-baseline.md`、`docs/open-commerce/integration-architecture.md`、`docs/open-commerce/decision-register.md` |
| 模型供应商和自定义模型 | `server/src/model_*`、`server/src/agent_model_*` |
| 用户等级、经验条、token 消耗/分享算力经验 | `server/src/user_progression.rs`、`server/src/store/user_progression.rs`、`server/src/token_usage_api.rs`、`server/src/store/node_ledger.rs` |
| context compiler / repo map | `server/src/context_compiler/` |
| 项目 RAG 工具上下文 | `server/src/context_compiler/agent_rag_context.rs` |
| 符号索引 API | `server/src/context_compiler/symbol_index_api.rs` |
| task pack / impact pack | `server/src/context_compiler/symbol_index_task_pack.rs`、`symbol_index_impact_pack.rs` |
| 向量检索 | `server/src/context_compiler/symbol_index_vector.rs` |
| embedding provider | `server/src/context_compiler/symbol_index_embedding_provider.rs` |
| SQLite 符号库 schema | `server/src/context_compiler/symbol_index_store.rs`、`symbol_index_embeddings.rs` |
| fb2 AI Center / 子项目聊天语音和业务上下文 | `docs/fb2-ai-center/`、`server/src/external_app_*`、`android/chat-voice-kit/` |
| 已否决的独立预言家 AI / Demo Oracle 议案 | `docs/decisions/reject-demo-oracle-role.md`（查询相关旧概念时必须先读） |
| 已否决的 AI-to-AI Skill / Skill 市场路线 | `docs/decisions/reject-ai-to-ai-skill-route.md`（旧 Git 文档不得恢复为当前架构） |

## Android 核心入口

| 领域 | 入口 |
|---|---|
| APK 主界面和导航 | `android/app/src/main/kotlin/com/elon/app/MainActivity.kt` |
| 应用更新 | `android/app/src/main/kotlin/com/elon/app/update/` |
| 网络/API/WebSocket | `android/app/src/main/kotlin/com/elon/app/net/` |
| 项目相关 UI | `android/app/src/main/kotlin/com/elon/app/project/` |
| `elon-self` 共享真机身份、无线 ADB 最近端点和连接约定 | `AI_PROJECT.md` 的“当前共享 Android 真机（项目记忆）”、`docs/shared-android-device-host.md`、`server/src/node_agent_android_inspector/` |
| 同节点多会话提交级合并、固定真机调试包、代次部署状态 | `AI_ARCHITECTURE.md` 的“PC 节点 AI 运行路线”、`docs/system-architecture.md`、`server/src/node_agent_android_live/debug_integration.rs`、`debug_package.rs` |

## Web/静态资源入口

| 领域 | 入口 |
|---|---|
| Web 项目页 | `server/src/assets/web_page.html` |
| PC 工作台当前入口 | `pc-frontend/`（React/Vite，承接 `/pc`；`/pc-next` 为同源兼容入口） |
| PC 工作台旧版对照 | `/pc-legacy` 由发布脚本从历史提交导出只读快照；仓库不再保留 `server/src/assets/pc_*` 源码 |
| PC 静态资源服务端托管 | `server/src/web.rs`、`server/src/router.rs` |
| PC 前端迁移规则 | `.github/instructions/pc-frontend-migration.instructions.md`、`docs/pc-frontend-migration.md` |
| 代码归属与 legacy 迁移规则 | `docs/architecture/source-of-truth.md`、`docs/architecture/legacy-inventory.md`、`docs/architecture/feature-parity-matrix.md`、`scripts/check-source-ownership.ps1` |
| 项目广场/项目主页脚本 | `server/src/assets/project_*.js` |
| 节点管理本地页 | `server/src/node_agent_admin.html` |

## 脚本入口

| 任务 | 命令 |
|---|---|
| 任务预检并创建隔离 worktree | `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree` |
| 发布前代码已推送检查 | `powershell -ExecutionPolicy Bypass -File scripts\check-task-complete.ps1 -Kind CodePushed` |
| 后端发布 | `powershell -ExecutionPolicy Bypass -File scripts\publish-server.ps1` |
| PC 前端本地预览 | `powershell -ExecutionPolicy Bypass -File scripts\start-pc-frontend-dev.ps1` |
| APK 发布 | `powershell -ExecutionPolicy Bypass -File scripts\publish-apk.ps1 -Changelog "<用户可见改动>"` |
| 统一收尾（同步 main、审计文件、清理 worktree） | `powershell -ExecutionPolicy Bypass -File scripts\finish-ai-task.ps1 -Kind <Kind>` |

## context compiler 产物

常见产物包括：

- `repo_map.md`
- `summaries.md`
- `symbols.jsonl`
- `symbol_index.jsonl`
- `symbol_edges.jsonl`
- `symbol_lookup.json`
- `symbol_index.sqlite`
- `chunks.jsonl`
- `tests.jsonl`
- `lsp_locations.jsonl`
- `semantic_facts.jsonl`
- `context_budget.json` / `context_budget.md`

面向 agent 的推荐入口优先级：

1. `repo_context_status`
2. `repo_context_task_pack`
3. `repo_symbol_search`
4. `list_dir` / `read_file` 作为兜底

## 修改前搜索建议

- 精确文案、函数名、错误信息：用 `rg`。
- Rust 类型、trait、调用关系：先查符号索引或 rust-analyzer 事实。
- 自然语言业务描述：优先 `repo_context_task_pack`，必要时启用 vector。
- 修改后影响面：查 impact pack，再跑建议测试。
