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
| 节点计算调用和账本证据 | 已实现并持续收口 | 已有节点调用、用量、补偿与 Assignment 结算证据，可作为未来影子结算输入 | `server/src/store/node_ledger.rs`、`server/src/group_ai/actions/assignment_actions.rs` |
| 链外影子经济回执 | 已实现、默认关闭 | 真实节点成本可形成幂等用量凭证，Matter 人工验收后生成双分录；商业调用只记录未扣费用量 | `server/src/task_settlement/`、`docs/task-shadow-settlement-v1-acceptance.md` |
| Sui 凭证投影 | 已实现、无网络副作用 | 已对账影子凭证可生成对象化数据和候选 PTB 步骤，固定标记 `not_submitted` | `server/src/task_settlement/sui_projection.rs` |
| 开放商业网络 V1 | 已实现 | 已有商户节点、商业能力、授权、调用、计量和审计的 HTTP/MCP 主路径 | `server/src/open_commerce_service.rs`、`server/src/open_commerce_mcp.rs`、`docs/open-commerce-network-v1-acceptance.md` |
| 受控商户自有运行时 | 已实现、生产配置依赖环境 | 平台可通过审核绑定和 HMAC 签名调用商户 ERP；`cofficethinking` 参考节点已实现商品、报价、显式确认下单、订单查询、幂等回执和库存事务 | `docs/open-commerce/merchant-runtime.md`、`docs/open-commerce-merchant-runtime-v1-acceptance.md` |
| 通用 ERP 蓝图与独立实例 | 已实现 | 可登记官方蓝图及不可变版本，从蓝图创建独立商户项目，并以公共模块、行业插件、主题和私有扩展表达差异；初始化 Matter 携带物化合同，状态从既有任务与产物证据派生 | `server/src/erp_blueprint/`、`docs/erp/README.md`、`contracts/erp/` |
| AI 通用功能提案治理 | 已实现 | AI 先匹配能力目录；只有经商户授权的脱敏信号才按独立实例去重聚合，维护者接受后才创建正式 Matter | `server/src/erp_blueprint/proposal.rs`、`server/src/erp_blueprint/matter_bridge.rs`、`docs/decisions/erp-feature-proposal-governance-v1.md` |
| ERP 兼容检查、采用与回滚 | 已实现、只管理治理状态 | 不可变发布清单驱动兼容检查；实例版本与活动状态原子更新，私有扩展保持不变，但 V1 不自动执行 Git、迁移或部署 | `server/src/erp_blueprint/compatibility.rs`、`server/src/store/erp_upgrades.rs`、`docs/decisions/erp-release-upgrade-v1.md` |
| 商户数据接入控制面 | 已实现 | 可登记厂商无关的数据来源、授权范围和数据域，以幂等同步回执记录健康度，并向开发代理提供脱敏上下文 | `server/src/open_commerce_integration_model.rs`、`docs/open-commerce-integration-control-plane-acceptance.md` |
| 开放商业连接器 SDK | 已实现 | 提供厂商无关 Manifest、健康检查、分页同步、幂等回执和兼容性门禁；不包含任何具体大厂生产适配器 | `sdk/open-commerce-connector/`、`docs/open-commerce/connector-sdk.md` |
| 商户主动发布与跨项目脱敏目录 | 已实现 | 商户默认私有；编辑者显式发布后，HTTP、MCP 和消费者沙盒只返回脱敏商户与能力契约，撤回后阻断外部调用 | `server/src/open_commerce_directory_service.rs`、`docs/decisions/open-commerce-directory-publication-v1.md` |
| 消费者发现与第三方应用沙盒 | 已实现、范围受限 | 可注册测试 App、一次性显示测试 Token、跨项目发现主动发布商户、申请并审批授权后调用；App 身份绑定所有者 | `server/src/open_commerce_client_api.rs`、`docs/open-commerce/consumer-developer-sandbox.md` |
| 项目 AI 资源控制面 | 已实现、只预演 | 可盘点当前用户的 Codex、本人节点、授权共享 Codex 和平台模型，保存项目策略并预演候选；不会启动真实任务 | `server/src/ai_resource_control/`、`docs/open-commerce/ai-resource-control.md` |
| 开放商业 PC 五工作区 | 已实现 | 项目详情内已有商户节点、消费者沙盒、开发者、AI 资源和影子经济五个独立视图 | `pc-frontend/src/features/open-commerce/`、`scripts/test-open-commerce-pc-workspace.js` |
| 项目文档治理和讨论知识图 | 已实现 | AI 可按范围分析、精确检索、按章节读取、审查模块化和维护讨论来源 | `server/src/node_agent_project_docs_mcp_tools.rs`、`docs/project-document-governance-mcp.md` |

## 部分实现或待验证能力

| 能力 | 状态 | 尚缺内容 |
|---|---|---|
| 美团、抖音、京东、淘宝闪购等经营数据统一接入 | 部分实现 | 接入控制面、状态和同步回执已实现；仍必须逐个平台确认官方授权、实现适配器、验证字段覆盖和长期稳定性，不能把登记数据源描述成已接通全量 API |
| 商户 ERP、海报、短视频、小游戏和营销活动自动生成 | 部分实现 | 通用 ERP 蓝图与 AI 应用开发主干已存在；真实行业业务模块、发布连接器、经营效果回流和规模化验证仍需完善 |
| 商户数据自主控制和跨应用授权 | 部分实现 | V1、跨项目脱敏目录、消费者沙盒和首个商户自有运行时已打通授权调用；可携带关系、生产 App 身份互认、限流与公共互操作治理尚未完成 |
| 闲置电脑、收银机和工作站共享算力 | 部分实现 | 已有节点执行、计量和资源控制面；控制面目前只盘点与预演，开放供需市场、异构调度、故障补偿和真实结算仍未完成 |
| 低成本分布式模型训练 | 提案 | 普通公网节点更适合异步任务、推理和可切分工作，不能宣称已等效替代高速互联的企业级 GPU 集群 |

## 尚未实现的提案

- Sui SDK、Move Package、钱包及测试网或主网适配器。
- 可转让网络代币、服务 Credit 或链上收入权益。
- 将每月合同收入自动分配给链上持有人。
- 具备生产应用审核、跨运营方身份互认、限流和滥用治理的公众消费者 AI 商业网络。
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
