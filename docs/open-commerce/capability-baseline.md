# 一龙开放商业能力基线

本文回答“项目现在到底已经实现了什么”。它是规划和融资材料的事实底座，不把讨论目标写成现有能力。

## 状态定义

| 状态 | 含义 |
|---|---|
| 已实现 | 仓库存在可运行代码、接口或验收证据 |
| 部分实现 | 主干能力存在，但行业连接器、产品体验或规模化能力仍不完整 |
| 已接受 | 已形成正式架构决定，可以作为后续实现约束 |
| 提案 | 仍需产品、技术或经济验证，不得作为当前功能宣传 |

## 当前能力

| 能力 | 状态 | 当前事实 | 主要证据 |
|---|---|---|---|
| 手机与 PC 自然语言开发应用 | 已实现 | 用户可从项目空间驱动 AI 在真实 Git 工作区修改、验证和构建项目 | `AI_PROJECT.md`、`docs/system-architecture.md`、`server/src/ai_cli/` |
| APK、网页和项目产物交付 | 已实现 | 平台已有项目执行、构建和发布链路，不等于所有第三方平台均已接入 | `AI_PROJECT.md`、`scripts/publish-apk.ps1` |
| 多成员、多 AI 协同开发 | 已实现并持续收口 | Matter、Assignment、执行节点、审核、产物和事件形成协作主干 | `server/src/group_ai/`、`docs/群体ai开发/群体AI开发功能需求与架构设计.md` |
| API Token 保管与使用统计 | 已实现 | 支持 Codex 凭据保险箱、额度与使用估算；凭据不应直接分发到普通客户端 | `server/src/codex_vault_api.rs`、`server/src/codex_vault_emergency_api.rs`、`docs/token消费统计.md` |
| 节点计算调用、供给授权和账本证据 | 已实现并持续收口 | 节点所有者可默认关闭或按模型、并发和每日 Token 预算开放推理供给；指定与自动调度会原子检查今日实耗、活动预留和本次预留。可信终态先冻结实际用量，过期执行会失败关闭并幂等释放预授权；所有者控制面派生近 24 小时失败、预留超出和过期租约健康快照 | `server/src/node_compute_sharing.rs`、`server/src/store/node_compute_sharing.rs`、`server/src/store/node_compute_runs.rs`、`server/src/store/node_compute_sharing_health.rs`、`server/src/store/node_ledger.rs` |
| 链外影子经济回执、争议与纠正 | 已实现、默认关闭 | 真实节点成本可形成幂等用量凭证，Matter 人工验收后生成双分录；争议阻断投影，已接受争议通过独立 Matter 原子追加冲销和替换。任意凭证可只读解析根、全部已过账步骤和当前有效凭证，循环或分叉失败关闭 | `server/src/task_settlement/`、`docs/decisions/task-shadow-settlement-corrections-v1.md`、`docs/decisions/task-shadow-settlement-lineage-v1.md`、`docs/task-shadow-settlement-v1-acceptance.md` |
| Sui 凭证投影与链下包 | 已实现、无网络副作用 | 标准凭证可保存单笔投影包；已过账纠正可把冲销与替换共同保存为不可拆分的原子包。两类包均绑定目标网络和来源摘要、可复核完整性，并固定标记 `not_submitted` | `server/src/task_settlement/sui_projection_service.rs`、`server/src/task_settlement/sui_correction_projection_service.rs`、`docs/decisions/sui-correction-projection-packages-v1.md` |
| 开放商业网络 V1 | 已实现 | 已有商户节点、商业能力、授权、调用、计量和审计的 HTTP/MCP 主路径 | `server/src/open_commerce_service.rs`、`server/src/open_commerce_mcp.rs`、`docs/open-commerce-network-v1-acceptance.md` |
| 能力输入与输出契约强制执行 | 已实现、范围受限 | 发布时校验有限 Schema 配置；无效输入在 Invocation 前返回 422，无效输出以零金额失败、释放 Grant 预算并使商户运行时降级；历史成功结果重放前按当前输出契约复验。当前不支持完整 JSON Schema，也不证明业务数据真实 | `server/src/open_commerce_capability_schema.rs`、`server/src/open_commerce_capability_contract_service.rs`、`docs/decisions/open-commerce-capability-contract-enforcement-v1.md`、`docs/open-commerce-capability-contract-enforcement-v1-acceptance.md` |
| PC Schema 驱动能力调用表单 | 已实现、范围受限 | 消费者可按商户发布的有限输入契约填写嵌套字段和列表；未声明默认值的可选字段默认省略，无法安全呈现的契约失败关闭，动作能力须对当前输入明确确认。当前无原始 JSON、文件和完整 JSON Schema 支持，也不证明订单或支付完成 | `pc-frontend/src/features/open-commerce/CapabilityInvocationComposer.tsx`、`pc-frontend/src/features/open-commerce/capabilityInvocationSchema.ts`、`docs/decisions/open-commerce-schema-driven-invocation-form-v1.md`、`docs/open-commerce-schema-driven-invocation-form-v1-acceptance.md` |
| 服务端动作能力一次性确认 | 已实现、范围受限 | `action` 必须先生成 5 分钟确认并由当前用户独立确认；凭证绑定用户、App、商户、能力、Grant、幂等键和输入摘要，Invocation 创建与确认消费原子完成，同一幂等请求可重放但不能产生第二次动作。当前不含 WebAuthn、跨设备可信展示或商户运行时内部订单确认 | `server/src/open_commerce_action_confirmation_service.rs`、`server/src/store/open_commerce_action_confirmations.rs`、`docs/decisions/open-commerce-server-action-confirmation-v1.md`、`docs/open-commerce-server-action-confirmation-v1-acceptance.md` |
| 受控商户自有运行时 | 已实现，出站加固代码未编译 | 平台可通过审核绑定和 HMAC 签名调用商户 ERP；`cofficethinking` 参考节点已实现商品、报价、显式确认下单、订单查询、幂等回执和库存事务。连接器 SDK 新增通用运行时内核，商户只需提供能力处理器与持久化幂等存储。平台每次运行调用会重新核对标准 443 HTTPS 白名单，并形成 DNS 私网拒绝、禁代理、禁重定向和单次地址固定代码；该加固批次尚未编译或网络验证 | `docs/open-commerce/merchant-runtime.md`、`server/src/open_commerce_outbound_security.rs`、`sdk/open-commerce-connector/src/merchant-runtime.js`、`docs/open-commerce-merchant-runtime-egress-pinning-v1-acceptance.md` |
| 通用 ERP 蓝图与独立实例 | 已实现 | 可登记官方蓝图及不可变版本，从蓝图创建独立商户项目，并以公共模块、行业插件、主题和私有扩展表达差异；初始化 Matter 携带物化合同，状态从既有任务与产物证据派生 | `server/src/erp_blueprint/`、`docs/erp/README.md`、`contracts/erp/` |
| AI 通用功能提案治理 | 已实现 | AI 先匹配能力目录；只有经商户授权的脱敏信号才按独立实例去重聚合，维护者接受后才创建正式 Matter | `server/src/erp_blueprint/proposal.rs`、`server/src/erp_blueprint/matter_bridge.rs`、`docs/decisions/erp-feature-proposal-governance-v1.md` |
| ERP 兼容检查、采用与回滚 | 已实现、只管理治理状态 | 不可变发布清单驱动兼容检查；实例版本与活动状态原子更新，私有扩展保持不变，但 V1 不自动执行 Git、迁移或部署 | `server/src/erp_blueprint/compatibility.rs`、`server/src/store/erp_upgrades.rs`、`docs/decisions/erp-release-upgrade-v1.md` |
| 商户数据接入控制面 | 已实现 | 可登记厂商无关的数据来源、授权范围和数据域，以幂等同步回执记录健康度，并向开发代理提供脱敏上下文 | `server/src/open_commerce_integration_model.rs`、`docs/open-commerce-integration-control-plane-acceptance.md` |
| 开放商业连接器 SDK | 已实现，新增工作器完整回归后置 | 提供厂商无关 Manifest、健康检查、分页同步、幂等回执、兼容性门禁，以及可自动领取、续租、完成、释放任务的通用 ERP/CRM 工作器；工作器已通过模块语法检查，不包含任何具体大厂生产适配器 | `sdk/open-commerce-connector/`、`docs/open-commerce/connector-sdk.md` |
| 商户主动发布与跨项目脱敏目录 | 已实现 | 商户默认私有；编辑者显式发布后，HTTP、MCP 和消费者沙盒只返回脱敏商户与能力契约，撤回后阻断外部调用 | `server/src/open_commerce_directory_service.rs`、`docs/decisions/open-commerce-directory-publication-v1.md` |
| 消费者发现与第三方应用沙盒 | 已实现、范围受限 | 可注册、轮换、停用和重新启用测试 App，一次性显示测试 Token，跨项目发现主动发布商户，查看或撤回申请并在审批后调用；App 身份绑定所有者 | `server/src/open_commerce_client_api.rs`、`server/src/open_commerce_client_lifecycle_service.rs`、`docs/open-commerce/consumer-developer-sandbox.md` |
| 开发者 App 资料清单与审核 | 代码已形成、未编译 | App 所有者或项目管理员可维护 HTTPS 主页、隐私政策、服务条款、支持邮箱和申请能力，以修订号提交；平台管理员可在 PC 待审工作台批准或要求修改，编辑会清除旧结论。审核不改变沙盒环境、不签发生产凭据，也不证明组织身份或域名所有权 | `server/src/open_commerce_developer_manifest_*.rs`、`server/src/store/open_commerce_developer_app_manifests.rs`、`pc-frontend/src/features/open-commerce/DeveloperAppManifestPanel.tsx`、`pc-frontend/src/features/open-commerce/DeveloperAppManifestReviewPanel.tsx`、`docs/open-commerce-developer-app-manifest-review-v1-acceptance.md` |
| 开发者 App 主页域名控制证明 | 代码已形成、未编译 | 当前资料修订可生成一次性 challenge，并从主页同源固定 HTTPS 地址完成精确白名单内的控制证明；编辑资料立即失效，提交审核前必须验证。它不证明组织身份、域名法律所有权或生产资质 | `server/src/open_commerce_developer_domain_service.rs`、`server/src/store/open_commerce_developer_app_domains.rs`、`pc-frontend/src/features/open-commerce/DeveloperAppManifestPanel.tsx`、`docs/open-commerce-developer-app-domain-verification-v1-acceptance.md` |
| 开发者 App 可撤销准入审查 | 代码已形成、未编译 | 当前资料和域名证明通过后，App 可提交主体声明；平台管理员可批准、退回、固定人工风险层级或暂停。资料变化与 App 停用会失败关闭旧状态。主体信息未经过外部工商核验，批准不签发生产凭据、不开启公共网络或真实交易 | `server/src/open_commerce_developer_admission_*.rs`、`server/src/store/open_commerce_developer_app_admissions.rs`、`pc-frontend/src/features/open-commerce/DeveloperAppAdmission*.tsx`、`docs/open-commerce-developer-app-admission-v1-acceptance.md` |
| 开发者 App 限权生产凭据 | 代码已形成、未编译，默认关闭 | 当前资料、域名证明和准入均有效后，平台管理员可显式签发一次性 `oc_live_` 凭据；明文不入库，范围不得超出已审资料，并受风险层级期限、轮换和紧急撤销约束。调用账本、幂等重放、终态事件和 Webhook 按凭据环境隔离，沙箱不得触达真实商户运行时。资料变化、域名重验、App 停用或准入暂停原子撤销，并停用关联生产订阅。它不代表外部身份核验、真实资金或生产部署 | `server/src/open_commerce_developer_credential_*.rs`、`server/src/store/open_commerce_developer_credentials.rs`、`server/src/open_commerce_invocation_service.rs`、`docs/open-commerce-invocation-credential-provenance-v1-acceptance.md` |
| 商户可控外部调用配额 | 已实现 | 可按能力和指定 App/全部 App 配置固定时间窗上限；新调用原子计数，幂等重放不重复占额，超限记录为零金额失败并返回 429 | `server/src/open_commerce_rate_limit_service.rs`、`server/src/store/open_commerce_rate_limits.rs`、`docs/open-commerce-rate-limits-v1-acceptance.md` |
| 商户级 App 紧急封禁 | 已实现 | 商户可手动封禁已注册 App，原子撤销有效 Grant 并取消待审批申请；解除不恢复旧授权 | `server/src/open_commerce_app_block_service.rs`、`server/src/store/open_commerce_app_blocks.rs`、`docs/open-commerce-app-blocks-v1-acceptance.md` |
| 商户侧 App 调用活动证据 | 已实现 | 从持久化调用派生外部 App 近 24 小时成功、失败、限流、Grant 预算拒绝和中断恢复计数；稳定原因只提醒人工处置，不自动评分或封禁 | `server/src/store/open_commerce_app_activity_health.rs`、`pc-frontend/src/features/open-commerce/OpenCommerceAppBlockManager.tsx`、`docs/open-commerce-app-activity-health-v1-acceptance.md` |
| Grant 限时授权 | 已实现 | 商户直接授权和批准申请均可选择期限；PC 默认 30 天，长期授权需显式选择。批准条件对双方可见，到期后发现和调用失败关闭，历史不改写 | `server/src/open_commerce_authorization_decision.rs`、`pc-frontend/src/features/open-commerce/openCommerceGrantExpiry.ts`、`docs/open-commerce-grant-expiration-v1-acceptance.md` |
| 消费者关系授权凭证 | 已实现、范围受限 | 消费者可对已发布商户建立最长 366 天的匿名关系并随时撤销；PC 在 14 天内提示安全续期，续期撤销旧凭证、轮换匿名标识且对重试幂等。关系凭证本身不保存偏好值、订单或 CRM 数据 | `server/src/open_commerce_relationship_service.rs`、`pc-frontend/src/features/open-commerce/ConsumerRelationshipManager.tsx`、`docs/open-commerce-consumer-relationship-renewal-v1-acceptance.md` |
| 消费者结构化偏好档案与关系披露 | 已实现、范围受限 | 当前用户可保存类别、标签、城市和价格上限等低敏偏好，显式用于发现，并对有效 `preference.remember` 关系选择字段生成匿名快照；商户只见有效关系披露。当前未实现敏感数据保险箱、字段加密、自动营销或跨运营方迁移 | `server/src/open_commerce_consumer_preference_service.rs`、`pc-frontend/src/features/open-commerce/ConsumerPreferenceProfilePanel.tsx`、`docs/open-commerce-consumer-preference-disclosures-v1-acceptance.md` |
| 消费者关联数据删除请求 | 已实现、范围受限 | 消费者可针对本人关系发起删除请求并原子撤销关系；商户只见匿名别名，可接单、拒绝或声明完成。平台不存待删除数据，商户完成不是平台验证的外部删除证明 | `server/src/open_commerce_data_request_service.rs`、`pc-frontend/src/features/open-commerce/ConsumerDataRequestManager.tsx`、`docs/open-commerce-consumer-data-erasure-requests-v1-acceptance.md` |
| 消费者可携带数据包 | 已实现、范围受限 | V3 把本人关系历史、私有续期链、删除请求、低敏偏好、历史披露和账户级终态调用凭证保存为幂等不可变快照，服务端和 PC 同时复核总包及每条凭证的 SHA-256；旧 V1/V2 包保持原摘要兼容。凭证可含本人已收到的商户结果，但不含原始输入，也不是商户完整订单或支付证明；尚未提供跨运营方导入 | `server/src/open_commerce_portability_service.rs`、`pc-frontend/src/features/open-commerce/ConsumerPortabilityExports.tsx`、`docs/open-commerce-consumer-portability-exports-v3-acceptance.md` |
| 消费者账户级调用凭证 | 已实现、范围受限 | 当前用户可列出跨项目终态调用摘要，读取并下载仅属于本人的结果；服务端和 PC 复核规范负载 SHA-256，且不暴露原始输入和内部标识。当前只支持未扣真实资金状态，不是订单、支付或链上凭证 | `server/src/open_commerce_consumer_receipt_service.rs`、`pc-frontend/src/features/open-commerce/ConsumerInvocationReceipts.tsx`、`docs/open-commerce-consumer-invocation-receipts-v1-acceptance.md` |
| 开发者 App 终态调用事件流 | 已实现、范围受限 | 测试 Token 可按 App 绑定游标持续读取成功或失败摘要，并读取本 App 单条结果；序号由数据库终态触发器原子追加，PC 开发者门户可断点续读且不持久化 Token。轮询继续作为恢复与对账入口 | `server/src/open_commerce_developer_event_service.rs`、`server/src/store/open_commerce_developer_events.rs`、`pc-frontend/src/features/open-commerce/DeveloperInvocationEvents.tsx`、`docs/open-commerce-developer-terminal-events-v1-acceptance.md` |
| 开发者 App 签名 Webhook | 代码已形成、未编译，生产默认关闭 | App 所有者可配置白名单内标准 443 HTTPS 回调，并固定沙箱或生产环境以及成功、失败事件范围；新订阅须完成签名 challenge 才能启用。耐久队列、短租约、指数退避、死信、自动停用、公网地址固定、密钥轮换、单条重试和最多 100 条历史补发均保留。沙箱只接收旧调用和沙箱调用；生产只接收生产调用，并要求双开关和当前有效 App、资料、域名、准入及凭据，资格失效会停用订阅并终止未完成投递。PC 已形成按环境聚合订阅、积压、重试、死信和生产阻断码的只读健康摘要。当前尚未完成编译、迁移、DNS、TLS、并发、网络、性能、跨语言签名和 UI 验证，也不是工商身份、跨运营方总线、支付回调或业务动作重放 | `server/src/open_commerce_webhook_*.rs`、`server/src/open_commerce_production_webhook*.rs`、`server/src/store/open_commerce_developer_webhook*.rs`、`server/src/store/open_commerce_production_webhooks.rs`、`docs/open-commerce-production-webhooks-v1-acceptance.md`、`docs/open-commerce-webhook-operational-health-v1-acceptance.md` |
| 商户业务调用证据与 ERP 关联 | 已实现、范围受限 | 项目成员可按商户读取终态 Invocation、结果摘要、可选标准业务回执和当前 ERP 实例关联；新商户运行回执格式错误时失败关闭。当前只提供证据层，不自动写入 ERP/CRM，也不证明支付或履约 | `server/src/open_commerce_merchant_evidence_service.rs`、`pc-frontend/src/features/open-commerce/MerchantBusinessEvidencePanel.tsx`、`docs/decisions/open-commerce-merchant-business-evidence-v1.md` |
| 业务证据到 ERP/CRM 显式衔接回执 | 已实现、范围受限 | 项目编辑者可记录人工确认回执；每个数据接入也可签发 1–366 天限时的一次性机器 Token，默认只写回执。轮换、撤销、到期和接入停用立即失败关闭，机器回执固化凭据版本。`applied` 必须有有效标准业务回执和目标记录摘要。机器鉴权仍不是外部签名、回读或自动入库证明 | `server/src/open_commerce_business_handoff_service.rs`、`server/src/store/open_commerce_adapter_credentials.rs`、`pc-frontend/src/features/open-commerce/OpenCommerceAdapterCredentialManager.tsx`、`docs/decisions/open-commerce-adapter-credential-expiration-v1.md` |
| ERP/CRM 待衔接任务队列 | 已实现、派生视图 | 从终态证据和最新衔接回执实时派生 `pending` 与 `retry_required`；成功或忽略自动移出，失败保留重试。HTTP、MCP 与 PC 共用服务，不自动执行外部系统 | `server/src/open_commerce_business_handoff_service.rs`、`pc-frontend/src/features/open-commerce/MerchantBusinessHandoffQueue.tsx`、`docs/decisions/open-commerce-business-handoff-queue-v1.md` |
| 接入器任务领取与短时租约 | 代码已落位、统一验证待执行 | 只有显式增加 `business_handoff.claim` 的限时凭据可领取单条有效终态结果；同一 Invocation 活动租约数据库防重，续租总期限不超过 1 小时，主动释放立即可重试，被拒绝任务采用 30–900 秒退避和最久未尝试优先，第 6 次拒绝后暂停并要求编辑者明确恢复，完成回执与租约状态原子绑定。尚无真实生产适配器或外部回读 | `server/src/open_commerce_adapter_claim_service.rs`、`server/src/store/open_commerce_adapter_claims.rs`、`server/src/store/open_commerce_business_handoffs.rs`、`pc-frontend/src/features/open-commerce/OpenCommerceAdapterClaimMonitor.tsx`、`docs/open-commerce-adapter-handoff-claims-v1-acceptance.md` |
| Grant 生命周期预算 | 已实现 | 商户可为单个授权设置总调用次数和总计量金额；调用前原子预留，成功确认、失败退回，幂等重放不重复占额；重启与过期孤儿调用会原子失败关闭并释放遗留预留 | `server/src/open_commerce_grant_budget_service.rs`、`server/src/store/open_commerce_grant_budgets.rs`、`server/src/store/open_commerce_invocation_recovery.rs`、`docs/open-commerce-grant-budgets-v1-acceptance.md` |
| 项目 AI 资源控制面 | 已实现、只预演 | 可盘点当前用户的 Codex、本人节点、授权共享 Codex 和平台模型，保存项目策略并预演候选；不会启动真实任务 | `server/src/ai_resource_control/`、`docs/open-commerce/ai-resource-control.md` |
| 开放商业 PC 五工作区 | 已实现 | 项目详情内已有商户节点、消费者沙盒、开发者、AI 资源和影子经济五个独立视图 | `pc-frontend/src/features/open-commerce/`、`scripts/test-open-commerce-pc-workspace.js` |
| 项目文档治理和讨论知识图 | 已实现 | AI 可按范围分析、精确检索、按章节读取、审查模块化和维护讨论来源 | `server/src/node_agent_project_docs_mcp_tools.rs`、`docs/project-document-governance-mcp.md` |

## 部分实现或待验证能力

| 能力 | 状态 | 尚缺内容 |
|---|---|---|
| 美团、抖音、京东、淘宝闪购等经营数据统一接入 | 部分实现 | 接入控制面、状态和同步回执已实现；仍必须逐个平台确认官方授权、实现适配器、验证字段覆盖和长期稳定性，不能把登记数据源描述成已接通全量 API |
| 商户 ERP、海报、短视频、小游戏和营销活动自动生成 | 部分实现 | 通用 ERP 蓝图与 AI 应用开发主干已存在；真实行业业务模块、发布连接器、经营效果回流和规模化验证仍需完善 |
| 商户数据自主控制和跨应用授权 | 部分实现 | V1、跨项目脱敏目录、消费者沙盒、限时 App 授权、消费者匿名关系及低敏偏好字段披露、删除请求与商户声明、可验证导出、调用配额、App 终态结果流、商户业务证据、人工和受限机器 ERP/CRM 衔接回执、活动证据、手动 App 封禁和首个商户自有运行时已打通；敏感数据保险箱、完整订单迁移、跨运营方导入、外部主动推送和删除证明、生产 ERP/CRM 写入适配器、外部回读、签名信任、生产 App 身份互认、自动全网风控与公共互操作治理尚未完成 |
| 闲置电脑、收银机和工作站共享算力 | 部分实现 | 模型推理供给的所有者开关、模型白名单、并发、每日实耗与在途预算原子预留、候选回退、流租约、重启与过期预授权回收、所有者运行告警和 PC 控件已实现；通用异构任务、竞价市场、故障赔付、真实提现与链上结算仍未完成 |
| 低成本分布式模型训练 | 提案 | 普通公网节点更适合异步任务、推理和可切分工作，不能宣称已等效替代高速互联的企业级 GPU 集群 |

## 尚未实现的提案

- Sui SDK、Move Package、钱包及测试网或主网适配器。
- 可转让网络代币、服务 Credit 或链上收入权益。
- 将每月合同收入自动分配给链上持有人。
- 具备生产应用审核、跨运营方身份互认、动态风控和全网滥用治理的公众消费者 AI 商业网络。
- 可由任意开发者部署并互联的完整联邦商户节点网络。
- 由协议自动治理的公开算力与 AI API 交易市场。

## 融合原则

这些能力可以融合，但必须共用同一组基础对象，不能各做一套平行系统：

1. 用户、商户、AI Agent 和计算节点共用统一身份与授权模型。
2. 开发任务、经营任务和算力任务统一进入 Matter 与 Assignment 生命周期。
3. 商业调用、模型调用和节点执行统一产出不可变审计回执。
4. 当前先由链外账本记录计量事实；Sui 只作为可替换的结算适配器，不反向侵入核心业务。
5. 商户数据保留在商户选择的存储中，协议开放的是经过授权的能力，不是公开下载全部数据库。
6. 项目文档必须明确标识已实现、部分实现、已接受和提案，避免 AI 基于愿景生成错误实现计划。

## 宣传口径

可使用：

> 我们已经具备 AI 应用开发、群体协作、节点执行计量和开放商业 V1 主干，正在把这些能力融合为商户自己的 AI 经营节点。

不可使用：

> 我们已经全面接管所有大型平台、拥有等效企业 GPU 集群，或已经完成 Sui 代币与收入分配网络。
