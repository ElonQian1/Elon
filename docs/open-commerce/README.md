# 一龙开放商业与 AI 经济能力包

本目录是开放商业方向的稳定入口。它不替代现有架构决策、API 契约或实施草案，只负责说明这些材料之间的关系，并防止 AI 把“已经实现”“已接受但仍在扩展”和“尚处于提案阶段”的内容混在一起。

## 一句话定位

一龙不是再做一个封闭式美团，而是在现有 AI 应用开发、群体协作和共享节点能力之上，为商户建立可自主控制的数据与 AI 经营节点，使经过授权的消费者 AI、第三方应用和商户应用能够发现、调用并交易商业能力。

## 当前阅读顺序

1. `docs/open-commerce/capability-baseline.md`：先确认项目已经具备什么、还缺什么。
2. `docs/open-commerce/integration-architecture.md`：理解应用开发、群体 AI、开放商业、共享资源和 Sui 提案如何共用一套主干。
3. `docs/decisions/open-commerce-network-principles.md`：查看已经接受的产品原则。
4. `docs/decisions/open-commerce-network-v1-architecture.md`：查看 V1 已接受的架构边界。
5. `docs/open-commerce-network-v1-api.md`：查看当前 HTTP 与 MCP 契约。
6. `docs/open-commerce/merchant-runtime.md`：查看商户自有 ERP 运行时、签名验证、报价与订单边界。
7. `docs/decisions/open-commerce-integration-control-plane.md`：查看多平台接入状态、同步回执和 AI 开发上下文边界。
8. `docs/erp/README.md`：查看通用 ERP 蓝图、独立商户实例、AI 通用提案和可回滚升级治理。
9. `docs/open-commerce/decision-register.md`：区分已接受决定、试验方向和未决问题。
10. `docs/decisions/task-shadow-settlement-v1.md`：查看链外影子经济层已经接受的事实来源、账本和 Sui 边界。
11. `docs/decisions/sui-offchain-projection-packages-v1.md`：查看 Sui 链下投影包、双摘要、目标网络和完整性复核边界。
12. `docs/decisions/task-shadow-settlement-disputes-v1.md`：查看争议案件、追加式事件和投影阻断边界。
13. `docs/decisions/task-shadow-settlement-corrections-v1.md`：查看纠正 Matter、原子冲销与替换、净额和普通 Sui 投影阻断边界。
14. `docs/decisions/sui-correction-projection-packages-v1.md`：查看冲销与替换共同组成链下原子包、摘要复核和网络未提交边界。
15. `docs/decisions/task-shadow-settlement-lineage-v1.md`：查看从任意凭证解析根、纠正步骤和当前有效凭证的只读边界。
16. `docs/open-commerce/connector-sdk.md`：查看厂商无关连接器契约、数据边界和兼容性门禁。
17. `docs/open-commerce/consumer-developer-sandbox.md`：查看消费者发现、App 注册、授权审批和测试调用闭环。
18. `docs/open-commerce/ai-resource-control.md`：查看 AI 资源盘点、项目策略和不执行任务的路由预演。
19. `docs/decisions/open-commerce-directory-publication-v1.md`：查看商户主动发布、脱敏目录和撤回边界。
20. `docs/open-commerce-directory-v1-acceptance.md`：查看目录 V1 的验证证据和未完成边界。
21. `docs/decisions/open-commerce-developer-lifecycle-v1.md`：查看沙盒 App 停用、重新启用和授权申请撤回边界。
22. `docs/open-commerce-developer-lifecycle-v1-acceptance.md`：查看开发者生命周期 V1 的验证证据。
23. `docs/decisions/open-commerce-rate-limits-v1.md`：查看商户可控调用配额、幂等和超限审计边界。
24. `docs/open-commerce-rate-limits-v1-acceptance.md`：查看调用配额 V1 的验证证据。
25. `docs/decisions/open-commerce-app-blocks-v1.md`：查看商户级 App 封禁、授权紧急撤销和解除后不恢复信任的边界。
26. `docs/open-commerce-app-blocks-v1-acceptance.md`：查看商户级 App 封禁 V1 的验证证据。
27. `docs/decisions/open-commerce-grant-budgets-v1.md`：查看单个 Grant 总调用和总计量预算的原子预留边界。
28. `docs/open-commerce-grant-budgets-v1-acceptance.md`：查看授权生命周期预算 V1 的验证证据。
29. `docs/decisions/node-compute-sharing-supply-v1.md`：查看节点所有者显式开放模型算力及原子调度边界。
30. `docs/node-compute-sharing-supply-v1-acceptance.md`：查看节点模型共享 V1 的验证证据和未完成范围。
31. `docs/node-compute-sharing-supply-v1-api.md`：查看策略字段、状态码和发现/调用契约。

## 专题地图

| 专题 | 状态 | 权威入口 |
|---|---|---|
| AI 应用开发与发布 | 已实现并持续演进 | `AI_PROJECT.md`、`AI_ARCHITECTURE.md` |
| 多人、多 AI 协同开发 | 已有实现，持续收口 | `docs/群体ai开发/群体AI开发功能需求与架构设计.md` |
| 商户节点、能力、授权、调用和审计 | V1 已接受并实现 | `docs/decisions/open-commerce-network-v1-architecture.md`、`docs/open-commerce-network-v1-api.md` |
| 商户自有 ERP 受控运行时 | 参考实现已完成，生产配置依赖环境 | `docs/open-commerce/merchant-runtime.md`、`docs/open-commerce-merchant-runtime-v1-acceptance.md` |
| 通用 ERP 蓝图与智能提案 | V1 已实现，代码执行仍走既有项目流程 | `docs/erp/README.md`、`docs/erp/acceptance-v1.md` |
| 商户数据来源、健康度和同步回执 | 控制面已实现，具体平台适配器待逐项验收 | `docs/decisions/open-commerce-integration-control-plane.md`、`docs/open-commerce-integration-control-plane-acceptance.md` |
| 连接器 SDK 与兼容性门禁 | 已实现 V1，尚不包含具体大厂适配器 | `sdk/open-commerce-connector/`、`docs/open-commerce/connector-sdk.md` |
| 消费者发现与第三方应用接入 | 跨项目基础目录、授权沙盒、App/申请生命周期、调用配额、Grant 总预算及商户级手动封禁已实现，生产公共网络未完成 | `docs/decisions/open-commerce-directory-publication-v1.md`、`docs/decisions/open-commerce-developer-lifecycle-v1.md`、`docs/decisions/open-commerce-rate-limits-v1.md`、`docs/decisions/open-commerce-grant-budgets-v1.md`、`docs/decisions/open-commerce-app-blocks-v1.md` |
| AI 资源盘点、策略与路由预演 | 控制面已实现，尚未接管真实任务调度 | `docs/open-commerce/ai-resource-control.md`、`server/src/ai_resource_control/` |
| API Token 保管与节点计算计量 | 已有实现，尚不是公开 Token 交易市场 | `docs/token消费统计.md`、`server/src/store/node_ledger.rs` |
| 节点模型算力显式共享 | V1 已实现，包含模型白名单、并发和每日阈值；尚不是完整算力市场 | `docs/decisions/node-compute-sharing-supply-v1.md`、`docs/node-compute-sharing-supply-v1-acceptance.md` |
| 链外影子用量、验收后双分录、争议纠正和 Sui 投影包 | V1 已实现、默认关闭；纠正可追加冲销与替换、解析当前有效凭证并保存链下原子包；所有读取失败关闭，所有包均不提交网络、不移动资金 | `docs/decisions/task-shadow-settlement-v1.md`、`docs/decisions/task-shadow-settlement-corrections-v1.md`、`docs/decisions/task-shadow-settlement-lineage-v1.md`、`docs/decisions/sui-correction-projection-packages-v1.md`、`docs/task-shadow-settlement-v1-api.md` |
| 商户 AI 经营、营销内容和业务应用生成 | 产品方向，按真实连接器逐步实现 | `docs/drafts/requirements/open-commerce-network.md` |
| 消费者 AI 与任意商户节点互联 | 商户主动发布、跨项目发现、授权、调用、固定时间窗配额和手动 App 封禁已实现；生产身份、自动风控和联邦治理仍待完成 | `docs/open-commerce/consumer-developer-sandbox.md`、`docs/drafts/open-commerce-network-roadmap.md` |
| 闲置算力公开市场 | 模型推理供给控制已实现；异构任务、竞价、赔付和真实结算仍是提案 | `docs/decisions/node-compute-sharing-supply-v1.md`、`docs/drafts/open-commerce-network-sui-agent-economy.md` |
| Sui 链上结算、网络资产和收入权益 | 提案；当前仅有未提交网络的链外信封和可复核投影包 | `docs/drafts/open-commerce-network-sui-agent-economy.md`、`docs/decisions/sui-offchain-projection-packages-v1.md` |

## 文档治理边界

- `docs/decisions/` 只记录已经接受的决定。
- `docs/open-commerce/` 记录稳定的能力地图、融合架构和决策状态，不承诺未实现功能。
- `docs/drafts/` 保存需求、路线图、Sui 和经济层提案，默认不作为当前实现事实。
- `docs/inbox/conversations/` 保存讨论来源或讨论摘要，只用于追溯，不直接定义产品。
- 功能是否已经实现，以实现引用、测试和验收记录为准，不能只依据讨论文案判断。

## 长期维护规则

新增能力先更新 `capability-baseline.md` 的状态和证据，再更新对应专题文档。跨领域变化只在 `integration-architecture.md` 记录连接关系，详细协议仍放在各自模块中。单篇文档接近模块化审查阈值时，优先建立目录入口并按职责拆分，不继续扩张为巨型总文档。
