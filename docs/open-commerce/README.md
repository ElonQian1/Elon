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
8. `docs/open-commerce/decision-register.md`：区分已接受决定、试验方向和未决问题。
9. `docs/decisions/task-shadow-settlement-v1.md`：查看链外影子经济层已经接受的事实来源、账本和 Sui 边界。
10. `docs/open-commerce/connector-sdk.md`：查看厂商无关连接器契约、数据边界和兼容性门禁。
11. `docs/open-commerce/consumer-developer-sandbox.md`：查看消费者发现、App 注册、授权审批和测试调用闭环。
12. `docs/open-commerce/ai-resource-control.md`：查看 AI 资源盘点、项目策略和不执行任务的路由预演。

## 专题地图

| 专题 | 状态 | 权威入口 |
|---|---|---|
| AI 应用开发与发布 | 已实现并持续演进 | `AI_PROJECT.md`、`AI_ARCHITECTURE.md` |
| 多人、多 AI 协同开发 | 已有实现，持续收口 | `docs/群体ai开发/群体AI开发功能需求与架构设计.md` |
| 商户节点、能力、授权、调用和审计 | V1 已接受并实现 | `docs/decisions/open-commerce-network-v1-architecture.md`、`docs/open-commerce-network-v1-api.md` |
| 商户自有 ERP 受控运行时 | 参考实现已完成，生产配置依赖环境 | `docs/open-commerce/merchant-runtime.md`、`docs/open-commerce-merchant-runtime-v1-acceptance.md` |
| 商户数据来源、健康度和同步回执 | 控制面已实现，具体平台适配器待逐项验收 | `docs/decisions/open-commerce-integration-control-plane.md`、`docs/open-commerce-integration-control-plane-acceptance.md` |
| 连接器 SDK 与兼容性门禁 | 已实现 V1，尚不包含具体大厂适配器 | `sdk/open-commerce-connector/`、`docs/open-commerce/connector-sdk.md` |
| 消费者发现与第三方应用接入 | 项目内沙盒已实现，公共网络未实现 | `docs/open-commerce/consumer-developer-sandbox.md`、`server/src/open_commerce_client_api.rs` |
| AI 资源盘点、策略与路由预演 | 控制面已实现，尚未接管真实任务调度 | `docs/open-commerce/ai-resource-control.md`、`server/src/ai_resource_control/` |
| API Token 保管与节点计算计量 | 已有实现，尚不是公开交易市场 | `docs/token消费统计.md`、`server/src/store/node_ledger.rs` |
| 链外影子用量、验收后双分录和 Sui 信封 | V1 已实现、默认关闭，不移动资金 | `docs/decisions/task-shadow-settlement-v1.md`、`docs/task-shadow-settlement-v1-api.md` |
| 商户 AI 经营、营销内容和业务应用生成 | 产品方向，按真实连接器逐步实现 | `docs/drafts/requirements/open-commerce-network.md` |
| 消费者 AI 与任意商户节点互联 | 项目内闭环可验证，跨项目公共网络仍是目标架构 | `docs/open-commerce/consumer-developer-sandbox.md`、`docs/drafts/open-commerce-network-roadmap.md` |
| 闲置算力公开市场 | 提案，尚未实现 | `docs/drafts/open-commerce-network-sui-agent-economy.md` |
| Sui 链上结算、网络资产和收入权益 | 提案；当前仅有未提交网络的链外信封 | `docs/drafts/open-commerce-network-sui-agent-economy.md` |

## 文档治理边界

- `docs/decisions/` 只记录已经接受的决定。
- `docs/open-commerce/` 记录稳定的能力地图、融合架构和决策状态，不承诺未实现功能。
- `docs/drafts/` 保存需求、路线图、Sui 和经济层提案，默认不作为当前实现事实。
- `docs/inbox/conversations/` 保存讨论来源或讨论摘要，只用于追溯，不直接定义产品。
- 功能是否已经实现，以实现引用、测试和验收记录为准，不能只依据讨论文案判断。

## 长期维护规则

新增能力先更新 `capability-baseline.md` 的状态和证据，再更新对应专题文档。跨领域变化只在 `integration-architecture.md` 记录连接关系，详细协议仍放在各自模块中。单篇文档接近模块化审查阈值时，优先建立目录入口并按职责拆分，不继续扩张为巨型总文档。
