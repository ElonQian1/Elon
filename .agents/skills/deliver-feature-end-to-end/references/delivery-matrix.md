# 通用交付矩阵

在非简单功能、重构、迁移、生产修复或多代理协作中使用本矩阵。每行对应一个可独立验收的能力，不对应一个文件。

## 独立状态轴

| 字段 | 值 | 含义 |
|---|---|---|
| `implementation_status` | `not_started` | 尚未实现。 |
| `implementation_status` | `in_progress` | 当前批次正在实现。 |
| `implementation_status` | `partial` | 已有部分能力，但合同仍有缺口。 |
| `implementation_status` | `implemented` | 计划内实现和兼容保护已完成。 |
| `implementation_status` | `fallback` | 明确由稳定旧路径或官方路径承接。 |
| `verification_status` | `not_run` | 尚无验证证据。 |
| `verification_status` | `offline_passed` | 静态、单元或本地合同已通过。 |
| `verification_status` | `integration_passed` | 跨模块或端到端集成已通过。 |
| `verification_status` | `environment_passed` | 目标设备、服务器或生产只读验证已通过。 |
| `verification_status` | `failed` | 当前证据证明行为不正确。 |
| `verification_status` | `deferred` | 所需环境暂不可用。 |
| `verification_status` | `user_action_required` | 下一步需要用户监督或授权。 |
| `delivery_status` | `not_required` | 本任务不需要发布或部署。 |
| `delivery_status` | `not_started` | 需要交付但尚未开始。 |
| `delivery_status` | `pushed` | 代码已进入远端权威分支。 |
| `delivery_status` | `published` | 工件已发布但尚未验证目标环境。 |
| `delivery_status` | `deployed` | 目标环境身份和入口已验证。 |
| `delivery_status` | `failed` | 发布或部署失败。 |
| `acceptance_status` | `pending` | 业务验收尚未完成。 |
| `acceptance_status` | `accepted` | 真实工作流满足验收条件。 |
| `acceptance_status` | `rejected` | 真实工作流不满足要求。 |
| `acceptance_status` | `deferred` | 验收环境或授权暂不可用。 |

状态轴不得相互推导。例如 `implemented + offline_passed + pushed + pending` 是合法状态，表示代码已同步但用户路径尚未验收。

## 能力行模板

| 能力 | 模块/Owner | 实现 | 验证 | 交付 | 验收 | 证据 | 剩余缺口 | 下一动作 |
|---|---|---|---|---|---|---|---|---|
| 示例：订单去重 | canonical import | implemented | integration_passed | pushed | pending | contract test 12/12 | 生产只读抽样 | 发布后查重复率 |

## 不同任务的基线重点

| 任务类型 | 实现前必须记录 |
|---|---|
| 新功能 | 用户入口、主路径、失败路径、权限和发布目标。 |
| Bug 修复 | 可复现输入、当前错误结果、预期结果和失败层。 |
| 重构 | 旧公开 API、字段、状态、错误、性能边界和回滚路径。 |
| 数据迁移 | 数据量、约束、幂等规则、备份、校验查询和回退。 |
| 多数据源 | 来源标识、优先级、能力、健康、去重、冲突和 fallback。 |
| UI 改动 | 真实用户任务、响应式状态、可访问性、加载/空/错状态。 |
| 发布运维 | 权威提交、工件身份、目标环境、健康检查和回滚入口。 |

## 风险批次

| 风险 | 例子 | 执行规则 |
|---|---|---|
| `read_only` | 查询版本、列表、日志、数据库计数 | 可批量执行，不修改业务状态。 |
| `reversible` | 测试记录、临时开关、可撤销配置 | 先记录旧状态和恢复命令。 |
| `sensitive` | 登录、私密内容、上传、通知、真实账号 | 用户明确授权，结果脱敏。 |
| `destructive` | 删除、付款、余额、权限、生产迁移 | 逐项确认、备份、幂等和回滚，不并行。 |

## 重构功能等价清单

- 旧入口和调用者已盘点。
- 请求字段、响应字段、状态码和错误语义有合同测试。
- 空值、重复、延迟、部分失败和重试行为已覆盖。
- 旧数据可读，新旧数据写入规则明确。
- UI 可见能力没有因内部统一而消失。
- 新架构具备来源、健康、错误和回退可观测性。
- 切换、回滚和删除旧路径分别有条件，不在同一步完成。

## 脱敏证据回执

```json
{
  "capability": "canonical-match-deduplication",
  "implementation_status": "implemented",
  "verification_status": "integration_passed",
  "delivery_status": "pushed",
  "acceptance_status": "pending",
  "git_sha": "0123456789abcdef",
  "artifact": null,
  "expected": "two source records resolve to one canonical record",
  "observed": "input_count=2, canonical_count=1, duplicates=0",
  "remaining_gap": "production read-only sample",
  "next_action": "verify after deploy"
}
```

不要在回执中保存密码、token、Cookie、完整个人数据、聊天正文或生产业务内容。

## 收尾摘要

按以下顺序汇总：

1. `BUSINESS_STATUS`：用户能力是否达成。
2. `IMPLEMENTATION_STATUS`：代码和兼容合同状态。
3. `VERIFICATION_STATUS`：实际完成的验证层级。
4. `DELIVERY_STATUS`：push、工件、部署身份。
5. `ACCEPTANCE_STATUS`：用户或生产工作流是否验收。
6. `REMAINING_GAPS`：具体缺口、Owner 和下一动作。
7. `WORKSPACE_STATUS`：主工作区、任务 worktree 和来源不明文件。
8. `FINALIZABLE`：只引用项目完成脚本结果，不自行推断。
