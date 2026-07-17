# Codex 桌面监督与 PC 本机执行

最后更新：2026-07-18

## 目标

当用户在 Codex 桌面端讨论项目改动时，默认由一龙 PC 本机节点启动 Codex CLI 完成真实修改，桌面端负责需求拆解、过程监督、独立验收和能力改进决策。这个机制同时长期实测 PC 端，但不把一次任务变成无限自改，也不把执行者的自述当成验收。

它包含两个受控闭环：

1. **任务闭环**：派发需求 -> PC 执行 -> 回传证据 -> 桌面验收 -> 修正或完成。
2. **能力闭环**：发现 PC 平台能力缺口 -> 若阻断原任务则先修复 -> 验收修复 -> 续跑原任务；非阻塞改进排在原任务完成后。

这里的“进化”指代码、Skill、调度、证据、恢复和测试能力经过版本化迭代，不是模型权重自行训练。

## 角色与边界

| 角色 | 负责 | 不负责 |
|---|---|---|
| 用户 | 确认目标、授权范围和高影响选择 | 逐条指挥内部技术步骤 |
| Codex Desktop | 拆验收条件、派发、观察、独立检查、写 verdict、决定修复/续跑 | 在 PC 任务运行时抢写同一工作区；扩大用户授权 |
| 一龙 PC 节点 | 鉴权、启动本机 Codex CLI、保存 journal、审批、取消、恢复和证据 | 替桌面端做最终独立验收 |
| PC Codex CLI | 按项目 `AGENTS.md` 修改、测试、提交、发布、统一收尾 | 再次派发给 PC 节点；仅用文字声称成功 |

桌面端若发现节点不可达，应回报探测证据并请求用户决定，不能静默切回桌面直接改代码。涉及删除、生产外部写入或其他审批敏感动作时，仍遵守原有授权边界。

## 协议

协议标识：`elon.desktop_pc_supervision.v1`。

创建本机任务仍使用 `POST /api/local-tasks`，只新增可选字段：

```json
{
  "project_id": "elon-self",
  "workspace_path": "D:\\projects\\elon",
  "prompt": "完成用户原始需求",
  "runtime_permission": "full_access",
  "supervision": {
    "supervisor": "codex_desktop",
    "task_role": "requirement",
    "acceptance_criteria": ["定向测试通过", "提交、发布和统一收尾通过"],
    "improvement_policy": "after_task_or_unblock"
  }
}
```

### 任务角色

| `task_role` | 含义 | 何时使用 |
|---|---|---|
| `requirement` | 用户原始需求或普通跟进 | 默认 |
| `capability_repair` | 修复阻断原任务的 PC 平台能力 | 只有能力缺口确实阻断时 |
| `resume_original` | 能力修复验收后恢复原任务 | 指向父任务和根任务 |
| `post_task_improvement` | 任务完成后的平台增强 | 不得延迟用户结果 |

### 改进策略

- `after_task_or_unblock`：非阻塞改进任务后做；阻塞能力可先修复。
- `after_task_only`：所有改进都排到当前任务之后。
- `observe_only`：只记录监督结论，不派生改进。

`parent_task_id` 和 `root_task_id` 把修复、续跑与原需求连成可审计任务树。旧调用方不传 `supervision` 时行为不变。

## 证据与验收

`GET /api/local-tasks/:task_id` 返回：

```json
{
  "supervision": {
    "protocol": "elon.desktop_pc_supervision.v1",
    "enabled": true,
    "contract": {},
    "review": {},
    "evidence": {
      "event_count": 20,
      "tool_calls": 4,
      "tool_results": 4,
      "failed_tools": 0,
      "file_change_events": 2,
      "changed_files": ["server/src/example.rs"],
      "terminal_event_seen": true
    }
  }
}
```

证据摘要用于快速定位，不能代替完整验收。桌面端至少检查：

- 原始验收条件是否逐项满足；
- journal 中实际工具调用、失败与终态；
- Git diff、提交范围和工作区状态；
- 项目要求的定向测试、构建、发布与线上/本机探针；
- 统一收尾是否给出 `FINALIZABLE=true`；
- 是否存在未披露风险、绕过或权限扩张。

验收写入 `POST /api/local-tasks/:task_id/supervision/review`：

```json
{
  "verdict": "accepted",
  "summary": "diff、测试、发布与统一收尾均已独立复核",
  "improvements": ["后续可缩短节点版本探测时间"],
  "reviewed_by": "codex_desktop"
}
```

可用结论为 `observing`、`accepted`、`needs_follow_up`、`blocked_capability`、`rejected`。契约和 review 都写入 append-only task journal；同一任务显示最新 review，历史仍保留。

## 桌面端操作入口

仓库级 Skill 位于 `.agents/skills/codex-pc-supervisor/`。辅助脚本兼容 Windows PowerShell 5.1，自动探测 `127.0.0.1:7799-7819`，只从受信 loopback Origin 获取 admin token，输出中不包含 token。

```powershell
$helper = '.agents\skills\codex-pc-supervisor\scripts\invoke-supervised-task.ps1'
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Probe
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Submit -ProjectId elon-self -WorkspacePath 'D:\projects\elon' -Prompt '需求' -AcceptanceCriteria '测试通过','发布通过'
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Wait -TaskId 'local-...' -WaitSeconds 55
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Review -TaskId 'local-...' -Verdict accepted -Summary '独立验收通过'
```

其他动作：

- `Inspect`：读取任务、journal、审批和监督状态。
- `Improve`：从父任务继承项目/工作区并创建能力改进；加 `-BlockingImprovement` 时角色为 `capability_repair`。
- `Resume`：从父任务恢复原始 prompt，角色为 `resume_original`。
- `SelfTest`：不连接节点，仅校验脚本的契约构造。

等待动作最多 55 秒，桌面端应分段等待并保持过程更新。`Wait` 会在窗口内重试节点短暂重启，并只读取小事件窗口；完整日志由 `Inspect` 获取，监督证据摘要仍由节点扫描完整 journal。设置 `ELON_NODE_ADMIN_URL` 可覆盖默认探测地址，但地址仍必须受节点 Origin 白名单信任。Windows 客户端 watchdog 使用同安装路径下的单实例选举，更新或修复并发启动时只保留一个守护者，避免互相重启运行时。

## 阻塞恢复顺序

```text
原任务失败
  -> 桌面确认是普通实现错误？ -- 是 --> needs_follow_up -> 窄修正任务
  -> 确认是平台能力缺口？ ---- 是 --> blocked_capability
                                      -> capability_repair
                                      -> 独立验收修复
                                      -> resume_original
                                      -> 再验收原需求
```

能力修复不得借机重写无关模块。修复失败时保留原任务、修复任务、journal 和 Git 现场，由用户决定扩大范围、回滚或停止。

## 安全与兼容

- 本机管理 API 继续要求动态 admin token 和受信 Origin；Skill 不打印 token。
- 执行 prompt 带 `<elon-pc-executor>`，执行者看到后禁止重新派发，避免递归。
- 契约使用 journal 事件持久化，没有为旧任务新增强制数据库列。
- PC 工作台只在 `supervision.enabled=true` 时显示监督卡，普通本机任务不受影响。
- 版本发布、灰度、回滚和节点兼容仍以 `docs/node-agent-upgrade-compatibility.md` 为准。
- 任何协议升级必须新增版本号或保持向后兼容，并补 Rust、PowerShell 与 PC 前端测试。

## 成功标准

一次监督任务只有同时满足下列条件才算完成：PC 执行任务到终态；桌面端拿到可复核证据；项目验证与发布规则通过；桌面端写入 `accepted`；没有未处理的阻塞能力缺口。长期质量看任务成功率、首包/总耗时、失败工具率、恢复成功率、验收驳回率和回归率，而不是“自动修改次数”。
