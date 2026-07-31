# 通用 ERP 蓝图 V1 接口与 AI 工具

本文记录 ERP 蓝图治理的 HTTP 与 MCP/Harness 入口。接口只管理蓝图、版本、实例和治理元数据，不读取或上传商户原始经营数据、密钥与私有源码。

## HTTP 接口

所有接口都位于现有项目权限边界内。读取接口要求项目访问权；写入接口要求可编辑角色。

| 方法 | 路径 | 用途 |
|---|---|---|
| GET | `/api/projects/:project_id/erp/overview` | 读取项目关联的蓝图、版本、实例、提案和升级活动 |
| POST | `/api/projects/:project_id/erp/blueprints` | 将当前项目登记为官方蓝图 |
| POST | `/api/projects/:project_id/erp/blueprints/:blueprint_id/versions` | 发布不可变版本清单 |
| POST | `/api/projects/:project_id/erp/blueprints/:blueprint_id/instances` | 创建独立商户项目实例 |
| GET | `/api/projects/:project_id/erp/capabilities` | 检索机器可读能力目录 |
| POST | `/api/projects/:project_id/erp/requirements/resolve` | 把需求解析为复用、组合、私有扩展或通用候选 |
| POST | `/api/projects/:project_id/erp/instances/:instance_id/signals` | 经授权提交脱敏需求信号 |
| POST | `/api/projects/:project_id/erp/proposals/:proposal_id/decision` | 由蓝图维护者接受或拒绝提案 |
| POST | `/api/projects/:project_id/erp/proposals/:proposal_id/matter` | 为已接受提案创建正式 Matter |
| POST | `/api/projects/:project_id/erp/instances/:instance_id/upgrades` | 准备兼容检查和升级活动 |
| POST | `/api/projects/:project_id/erp/upgrades/:campaign_id/decision` | 人工确认采用或回滚 |

## MCP/Harness 工具

ERP 工具合并到现有开放商业 MCP 工具列表中，供项目内 AI 在开发商户系统时按需调用。

| 工具 | 副作用 | 用途 |
|---|---|---|
| `erp_get_overview` | 无 | 读取当前项目的 ERP 治理概况 |
| `erp_search_capabilities` | 无 | 开发前检索已有能力，避免重复造轮子 |
| `erp_resolve_requirement` | 无 | 生成需求分类和实现建议，不修改公共内核 |
| `erp_submit_feature_signal` | 写治理元数据 | 仅在商户明确授权后提交脱敏信号 |
| `erp_prepare_upgrade_check` | 写升级计划 | 只计算并保存兼容结果，不执行代码或部署 |

AI 工具故意不提供提案接受、Matter 创建、公共版本发布、升级采用或回滚操作。这些操作必须由有权限的人通过 PC 工作台或受控 HTTP 接口完成。

## 机器合同

- `contracts/erp/erp-blueprint-v1.schema.json`
- `contracts/erp/erp-instance-v1.schema.json`
- `contracts/erp/erp-feature-signal-v1.schema.json`
- `contracts/erp/erp-release-manifest-v1.schema.json`

参考清单位于 `examples/erp-blueprints/`。其中咖啡店与最小零售实例使用同一内核版本，但拥有不同主题、插件和私有扩展边界。
