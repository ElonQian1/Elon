# 通用 ERP 蓝图 V1 接口与 AI 工具

本文记录 ERP 蓝图治理的 HTTP 与 MCP/Harness 入口。接口只管理蓝图、版本、实例和治理元数据，不读取或上传商户原始经营数据、密钥与私有源码。

## HTTP 接口

所有接口都位于现有项目权限边界内。读取接口要求项目访问权；写入接口要求可编辑角色。

| 方法 | 路径 | 用途 |
|---|---|---|
| GET | `/api/projects/:project_id/erp/overview` | 读取项目关联的蓝图、版本、实例、提案和升级活动 |
| POST | `/api/projects/:project_id/erp/blueprints` | 将当前项目登记为官方蓝图 |
| POST | `/api/projects/:project_id/erp/blueprints/:blueprint_id/versions` | 发布不可变版本清单 |
| POST | `/api/projects/:project_id/erp/blueprints/:blueprint_id/evolve` | 按定义修订号追加模块、能力、主题或扩展点 |
| POST | `/api/projects/:project_id/erp/blueprints/:blueprint_id/instances` | 新建独立商户项目实例，或以 `target_project_id` 纳入现有项目 |
| GET | `/api/projects/:project_id/erp/capabilities` | 检索机器可读能力目录 |
| POST | `/api/projects/:project_id/erp/requirements/resolve` | 把需求解析为复用、组合、私有扩展或通用候选 |
| POST | `/api/projects/:project_id/erp/instances/:instance_id/signals` | 经授权提交脱敏需求信号 |
| POST | `/api/projects/:project_id/erp/instances/:instance_id/configuration` | 由商户按配置修订号更新主题、模块和扩展元数据 |
| POST | `/api/projects/:project_id/erp/instances/:instance_id/bootstrap-matter` | 原子创建或返回商户实例初始化 Matter |
| GET | `/api/projects/:project_id/erp/instances/:instance_id/materialization` | 从 Matter、Assignment 和 Artifact 真源读取初始化状态、证据与下一步 |
| GET | `/api/projects/:project_id/erp/instances/:instance_id/open-commerce-readiness` | 聚合 ERP 物化、商户节点、运行时、对外能力和目录发布状态；多商户时可传 `merchant_id` |
| POST | `/api/projects/:project_id/erp/instances/:instance_id/open-commerce-merchant` | 由商户按配置修订号绑定或解除 ERP 对应的开放商业商户节点 |
| POST | `/api/projects/:project_id/erp/proposals/:proposal_id/decision` | 由蓝图维护者接受或拒绝提案 |
| POST | `/api/projects/:project_id/erp/proposals/:proposal_id/matter` | 为已接受提案创建正式 Matter |
| POST | `/api/projects/:project_id/erp/instances/:instance_id/upgrades` | 准备兼容检查和升级活动 |
| POST | `/api/projects/:project_id/erp/upgrades/:campaign_id/decision` | 由商户确认采用或回滚；采用必须附执行验证证据 |

## MCP/Harness 工具

ERP 工具合并到现有开放商业 MCP 工具列表中，供项目内 AI 在开发商户系统时按需调用。

| 工具 | 副作用 | 用途 |
|---|---|---|
| `erp_get_overview` | 无 | 读取当前项目的 ERP 治理概况 |
| `erp_get_materialization_status` | 无 | 读取实例初始化合同、任务进度、失败项和证据缺口，不启动任务 |
| `erp_get_open_commerce_readiness` | 无 | 区分消费者 AI 可调用、开放目录可发现与 ERP 项目已验收，不返回运行地址或密钥引用 |
| `erp_search_capabilities` | 无 | 开发前检索已有能力，避免重复造轮子 |
| `erp_resolve_requirement` | 无 | 生成需求分类和实现建议，不修改公共内核 |
| `erp_submit_feature_signal` | 写治理元数据 | 仅在商户明确授权后提交脱敏信号 |
| `erp_update_instance_configuration` | 高风险写实例元数据 | 在项目写权限和工具确认边界内登记主题、模块、插件与私有扩展；MCP 标记为 destructive，不上传源码或经营数据 |
| `erp_prepare_upgrade_check` | 写升级计划 | 只计算并保存兼容结果，不执行代码或部署 |

AI 工具故意不提供蓝图演进、提案接受、Matter 创建、公共版本发布、升级采用或回滚操作。这些操作必须由有权限的人通过 PC 工作台完成。实例配置工具受当前项目写权限和 Harness 工具确认约束，`merchant_confirmed` 只是业务确认字段，不应被当作独立的身份凭证。

能力检索和需求解析始终使用当前商户固定版本的能力目录。维护项目可查看最新已发布目录以及尚未发布的能力标识，但尚未发布能力不会出现在旧商户实例的可复用结果中。

创建实例时默认使用 `project_name` 新建项目；传入 `target_project_id` 时忽略空的 `project_name`，并要求操作者对目标项目具有 `owner`、`admin` 或 `editor` 角色。响应中的 `onboarding_mode` 以及物化合同中的 `target_onboarding_mode` 用于区分 `new_project` 与 `existing_project`。该接口只登记治理关系，不导入本机目录、不复制 Git 仓库，也不启动开发任务。

版本清单可以通过 `runtime` 固定 `yilong.erp.kernel.v1` 的 Node 包名称和严格语义版本。物化合同只把这项不可变绑定传递给后续 Matter/Assignment，不能自行下载依赖、修改目标项目或宣称部署成功。当前官方 `1.2.0` 示例绑定 `@yilong/merchant-erp-kernel@0.1.0`；生产数据库适配、商户项目迁移和包分发仍需独立任务与验收。

开放商业就绪度是从各领域真源即时生成的只读投影，不新增第二套执行状态机。ERP 实例可以把同项目中的一个商户节点登记为稳定商业身份，该字段属于现有实例配置并复用 `configuration_revision`：绑定变化后旧物化证据自然不再匹配。已绑定实例不能被查询参数临时覆盖；未绑定且只有一个商户节点时可安全预览，多节点时必须先传 `merchant_id` 预览，再由有编辑权的人确认绑定。`consumer_invocation_ready` 要求商户启用、运行时已验证且至少存在一项非 `owner_only` 的有效 `merchant_runtime` 能力；其中 `public` 能力可按现有公开规则调用，`authorized` 能力仍必须由具体消费者 App 取得有效 grant，本投影不会代替授权。`consumer_discovery_ready` 还要求商户已发布到开放目录；`erp_onboarding_ready` 只由物化证据是否达到 `accepted_verified` 决定。这三个结果不得互相替代。

## 机器合同

- `contracts/erp/erp-blueprint-v1.schema.json`
- `contracts/erp/erp-instance-v1.schema.json`
- `contracts/erp/erp-feature-signal-v1.schema.json`
- `contracts/erp/erp-release-manifest-v1.schema.json`
- `contracts/erp/erp-upgrade-campaign-v1.schema.json`
- `contracts/erp/erp-materialization-contract-v1.schema.json`
- `contracts/erp/erp-materialization-evidence-v1.schema.json`
- `contracts/erp/erp-open-commerce-readiness-v1.schema.json`

参考清单位于 `examples/erp-blueprints/`。其中咖啡店与最小零售实例使用同一内核版本，但拥有不同主题、插件和私有扩展边界。
