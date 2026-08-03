---
title: ADR：开放商业接入器机器凭据 V1
status: accepted
owner: backend
date: 2026-08-03
---

# ADR：开放商业接入器机器凭据 V1

## 背景

待衔接队列和人工回执已经能追踪 ERP/CRM 处理结果，但真实接入器若继续复用项目成员 Token，就无法区分“人确认”与“机器提交”，也无法只撤销某个接入器。生产平台适配器尚未实现，不应为解决机器身份而提前开放经营数据读取、消费者能力或通用项目权限。

## 决策

1. 每个已登记数据接入最多保留一个可轮换机器凭据，凭据身份固定绑定项目、商户和接入器。
2. V1 权限固定为 `business_handoff.write`，只能向专用端点提交业务衔接回执；不能读取经营原始数据、调用消费者能力或使用项目管理 API。
3. 明文 Token 只在签发或轮换时返回一次。服务端只保存 SHA-256 和末尾提示，不提供找回接口，PC 不写入 `localStorage` 或 `sessionStorage`。
4. 签发、轮换和撤销需要项目编辑权限。HTTP 与 MCP 都要求明确确认；MCP 参数固定 `confirmed_by_user=true`。
5. 轮换沿用凭据 ID、递增版本并立即替换摘要；撤销替换摘要并标记 `revoked`。停用所属接入后，即使凭据仍为活动状态也拒绝鉴权。
6. 机器提交不接受请求方自报项目、商户或接入器，全部从 Bearer 凭据派生；回执权威标记为 `adapter_token_authenticated`，`confirmed_by_user=false`。
7. 每条机器回执固化凭据 ID 和提交时版本，且插入时再次核对该版本仍活动，防止鉴权后并发轮换产生旧身份写入。
8. 人工回执继续保持 `project_editor_asserted`，历史记录不静默升级。两类回执共用证据摘要、结果校验、幂等和队列派生规则。

## 安全边界

- Bearer Token 必须通过 HTTPS 传输。当前未实现 mTLS、非对称签名或硬件密钥；限时有效由后续 ADR 补充。
- 机器鉴权只能证明请求持有当时有效的接入器凭据，不能证明外部 ERP 数据真实，也不是外部平台签名或独立回读。
- `applied` 仍必须绑定成功调用、有效标准业务回执和目标记录号摘要；所有回执固定 `funds_moved=false`。
- 大型平台官方授权、生产写入事务、回读验证、速率治理和密钥托管仍按具体适配器逐项实现。

## 后续演进

`docs/decisions/open-commerce-adapter-credential-expiration-v1.md` 已增加 1–366 天服务端强制有效期。原有机器身份、固定权限、轮换、撤销和历史回执版本语义不变；到期后必须显式轮换，不会自动续期。

`docs/decisions/open-commerce-adapter-handoff-claims-v1.md` 进一步把“默认只写”和“可领取任务”拆成两个权限。历史凭据及未勾选领取能力的新凭据仍只有 `business_handoff.write`；项目编辑者明确轮换并选择任务领取后，凭据才同时获得 `business_handoff.claim`，且只能通过短时租约读取一条绑定任务，不能使用项目查询接口或任意读取经营数据。本 ADR 的 V1 固定只写结论保留为初始安全基线，不应被理解为所有后续凭据永远无法显式扩权。

## 实现证据

- 凭据模型与迁移：`server/src/open_commerce_adapter_model.rs`、`server/src/open_commerce_adapter_migration.rs`
- 凭据存储与鉴权：`server/src/store/open_commerce_adapter_credentials.rs`
- HTTP 与 MCP：`server/src/open_commerce_adapter_api.rs`、`server/src/open_commerce_adapter_mcp.rs`
- 回执绑定：`server/src/open_commerce_business_handoff_service.rs`、`server/src/store/open_commerce_business_handoffs.rs`
- PC 工作台：`pc-frontend/src/features/open-commerce/OpenCommerceAdapterCredentialManager.tsx`
- 测试：`server/src/open_commerce_adapter_tests.rs`、`scripts/test-open-commerce-pc-workspace.js`
