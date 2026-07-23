# Codex 桌面监督与 PC 本机执行

最后更新：2026-07-22

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

### 本地优先、恢复与后台改进不变量

- 本机任务先由节点本地管理面鉴权和调度；远程 `elon.desktop_pc_supervision.v1` 任务必须同时证明节点身份、协议能力、有效 live lease 和连接连续性，任一证据缺失都 fail-closed。远程失败不能降级成未受监督的本地写任务。
- `post_task_improvement` 只接受当前 v1 合同、已终止父任务以及完全相同的 owner、node、project、root 和 workspace 身份。它进入持久化低优先队列，使用独立 task、conversation 和平台隔离 worktree，不占用原用户任务的会话或 worktree；原用户任务先到终态并释放执行资源。
- 前台任务、全局发布 owner、节点更新或构建资源压力出现时，后台改进先把 durable action intent 和取消审计落盘，再取消执行器并转为暂停；任一步持久化失败都拒绝改变队列状态。门禁消失后沿同一 root 的独立 conversation/worktree 自动产生下一代继续；配额、429、credit/rate-limit 错误进入有上限退避重试。成功后进入 `review_required`，只有带 reviewer、review source 和时间的 Desktop approve/reject 后才结束审查。任务进入可信终态后即精确释放该 root 的 Git 执行 lease；Review 仍独立持久化验收结论，不再承担唯一解锁职责。
- 同一 root 的多代 `resume_original` 或后台改进只能继承已验证的基础仓库、平台路径、分支、Git 身份和 `elon-supervision:<root_task_id>` lease。旧项目别名先规范化为节点当前绑定再做授权比较；别名不能把跨项目、跨 root、脏而不可信或并发占用的现场变成合法继承。
- 计划更新的安全恢复不需要用户点 `Resume`：两个 recovery v1 回执和本地 restart checkpoint 按单调状态合并，`resumed`/终态不能再被旧 `restart_recovery` 覆盖成 `resume_required`。新进程必须继续同一 journal 游标并正常终止；只有无法证明任务身份、现场独占或恢复事务完整性时才暴露 `resume_required`。
- 取消审计统一保留 `requested_by`、`source`、`reason`、`requested_at_ms`，系统让路再记录可信 `interruption_source`：`supervisor_intervention`、`node_restart` 或 `updater_apply`。审计先写 sidecar/journal 才允许实际取消，不能从 `exit=-1` 反推来源；读取旧记录时新字段可空，不能因此拒绝旧任务。
- 发布使用跨组件全局 lease：状态页显示 owner、FIFO waiters 和 coalesced waiters；相同 release kind + SHA 合并等待，不同 SHA 排队。server、PC 前端、Windows 节点共享 `batch_id + immutable SHA`，批次显式声明 `server / pc_frontend / windows_node` 三个预期阶段，缺少任一阶段都不会成功。逐阶段 heartbeat、attempt、接管、成功/失败写入原子持久 ledger；未知、过期、损坏状态一律 fail-closed。release SHA 在 claim 时固定且不可变，等待期间不得因 `main` 前进反复改 SHA 而饿死。
- PC 操作审查使用 `pc_operator:<owner>` 和 `local_pc_ui`，不得冒充 `codex_desktop`；Desktop 辅助脚本必须显式发送 `codex_desktop_helper`。只有真实更新任务才要求 recovery receipt，普通监督任务不因没有更新回执而拒绝审查。

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
      "command_exit_codes": [{"command": "cargo test", "exit_code": 0}],
      "failure_summaries": [],
      "agent_messages": 1,
      "terminal_event_seen": true
    }
  },
  "runtime": {
    "phase": "verification",
    "current_command": "cargo test --bin elon-pc-node",
    "last_progress": 1784361600000,
    "heartbeat": 1784361605000,
    "idle_duration": 5,
    "timeout_policy": {
      "mode": "progress_aware",
      "total_timeout_secs": 21600,
      "idle_timeout_secs": 900,
      "heartbeat_secs": 15,
      "progress_aware": true
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
  "improvements": ["后续可缩短节点版本探测时间"]
}
```

公共 review DTO 为兼容旧 helper 可接收但会忽略 `reviewed_by` 和 `review_source`；新 helper 不发送这两个字段。服务端始终依据已认证路由覆盖调用方提示。本机 UI 路由固定注入
`pc_operator:<owner> / local_pc_api`；Desktop helper 使用独立的 `desktop-review`
路由，由节点注入 `codex_desktop / codex_desktop_helper`，调用方不能通过 body 冒充。

可用结论为 `observing`、`accepted`、`needs_follow_up`、`blocked_capability`、`rejected`。契约和 review 都写入 append-only task journal；同一任务显示最新 review，历史仍保留。

## 桌面端操作入口

仓库级 Skill 位于 `.agents/skills/codex-pc-supervisor/`。辅助脚本兼容 Windows PowerShell 5.1 和 pwsh 7。节点发现依次尝试显式 URL、当前进程最近成功的 URL、7799 快路径，只有失败才扫描 7800–7819；每次都从受信 Origin 重新取得 admin token，输出不包含 token。`Probe`、`Projects`、`Inspect`、`Wait` 不读取或写入工作区外的持久 URL 缓存，重复连接优先使用同进程地址或显式 `ELON_NODE_ADMIN_URL`，避免只读监督触发 Command approval；其它动作保存的最近 URL 仍只包含 loopback 地址，不含 token。

```powershell
$helper = '.agents\skills\codex-pc-supervisor\scripts\invoke-supervised-task.ps1'
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Probe
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Submit -ProjectId elon-self -WorkspacePath 'D:\projects\elon' -Prompt '需求' -AcceptanceCriteria '测试通过','发布通过'
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Wait -TaskId 'local-...' -WaitSeconds 55
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Inspect -TaskId 'local-...' -Since 40 -Limit 25 -Compact
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Review -TaskId 'local-...' -Verdict accepted -Summary '独立验收通过'
powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Supersede -TaskId 'local-...' -Prompt '完整的新需求' -AcceptanceCriteriaJson '["新验收条件"]' -AmendmentReason '用户明确改变目标'
```

其他动作：

- `Inspect`：读取任务、journal、审批、运行时和监督状态；`-Since`、`-Limit` 做增量窗口，`-Compact` 只返回有界摘要和证据。
- `Improve`：从父任务继承项目/工作区并创建能力改进；加 `-BlockingImprovement` 时角色为 `capability_repair`。
- `Resume`：从已终止、带当前监督协议且保留平台隔离 worktree 的父任务恢复原始 prompt，角色为 `resume_original`。父任务角色只允许 `requirement` 或 `resume_original`；能力修复验收后应 Resume 原 `requirement`（或既有 `resume_original` 代），绝不能 Resume `capability_repair`。helper 先做只读门禁；节点不信任 helper，并在解析 root workspace、lease 准入/迁移、Git identity migration 和创建子任务之前执行同一白名单。随后节点再以本机任务记录的基础仓库、隔离路径、分支、`git_head` 和 `elon-supervision:<root_task_id>` lease 做权威校验；不能用参数指定任意 worktree。历史监督后代 lease 只有在节点把持久任务契约逐代验证到同一 `requirement` 根、节点/安装/owner/项目/基础工作区身份一致且无活跃 prompt/sidecar 时，才在跨进程 Resume 准入锁内原子迁移为 root lease；通用锁与未知 lease 不迁移。父任务也是 `resume_original` 时，节点沿已验证的父监督契约继承原工作树身份，不会按续跑任务的新 conversation id 另推路径；父子 root identity 或任一 Git 身份不一致仍拒绝。身份可信且无活跃 prompt/sidecar 占用时允许原位复用有效 Git 脏现场，staged、unstaged、untracked 均不覆盖、不自动提交、不丢弃。目录仍在但 Git worktree 注册丢失时，原目录保持原位且只读；只有分支后继/主线归属、目录内容、完整 provenance 和所有同根占用均可证明时，节点才创建新的 conversation worktree、迁移根 provenance、建立唯一 root lease，任一不一致仍拒绝。
- `Supersede`：仅在用户明确改变需求或验收条件时使用。必须携带完整的新 prompt、新验收条件和 `AmendmentReason`；原 `requirement` 合同保持不可变，新一代仍走 `resume_original` 的工作区/租约安全门禁，并在 `resume_context` 中追加 `elon.supervision.contract_revision.v1` 收据。后续普通 Resume 沿不可变父链重建最近有效需求，不会退回旧根需求；缺少收据的条件漂移继续 fail-closed。
- 受监督任务的 `workspace_status` 启动值只是身份锚点；节点在 `finished_at_ms` 和可靠业务终态同一 SQLite 事务内，重新核对平台路径、分支、Git common-dir、origin remote、root lease、owner/node/install/project 与并发占用后，把 `git_head` 原子刷新为终态快照。HEAD 未变化时 staged、unstaged、untracked 均保留；HEAD 前进时新提交立即成为 Resume 基线。重复终态回放得到同一结果。任何身份或现场读取失败都保留原启动证据，并写入 `resume_blocked_reason`，因此旧 HEAD 不会冒充可信终态；普通非监督任务不进入该刷新路径。
- `SelfTest`：不连接节点，校验 PS5.1/PS7 UTF-8、旧数组以及 JSON/UTF-8 文件多条件构造。

等待动作最多 55 秒，桌面端应分段等待并保持过程更新。`Wait` 在一次调用内从 `Since` 游标前进，不重复轮询旧窗口，并返回 `next_cursor` 供下一次调用续读；缺省窗口为 25，`Inspect` 缺省为 200。监督证据摘要仍由节点扫描完整 journal。验收条件经过外层 `powershell -File` 时，优先使用 `-AcceptanceCriteriaJson '["条件一","条件二"]'` 或 `-AcceptanceCriteriaFile criteria.json`，避免数组被宿主展平误绑定；旧 `-AcceptanceCriteria` 保持兼容。设置 `ELON_NODE_ADMIN_URL` 可覆盖默认探测地址，但地址仍必须受节点 Origin 白名单信任。

带 `elon.desktop_pc_supervision.v1` 的本机 Codex 任务使用“可配置总时限 + 进展感知空闲时限”：总时限默认 21600 秒、范围 1201–86400 秒，空闲时限默认 900 秒、范围 30–7200 秒，心跳默认 15 秒、范围 1–60 秒。分别用 `ELON_SUPERVISED_CODEX_TIMEOUT_SECS`、`ELON_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS`、`ELON_SUPERVISED_CODEX_HEARTBEAT_SECS` 配置。Codex 输出、命令和文件事件刷新进展；纯推理时节点仍写明确 heartbeat，但节点自己的 heartbeat 不冒充模型进展，因此真正无进展仍会空闲超时。普通任务仍使用原固定时限，取消和进程树回收不放宽。

Codex JSON 的 `item.started/item.completed` 会以有界 `codex_item` 写入 journal：`command_execution` 保留脱敏命令、退出码和输出首尾摘要，`file_change` 保留文件路径，`agent_message` 只保留有界文本。非结构化 stdout/stderr 整体聚合为每流一个 `cli_output_summary`，记录原始行数/字节数、首尾、保留行数和 `truncated`，避免一次高输出 `rg` 生成数百 journal 事件。PC 任务详情显示 phase、脱敏 current command、最近进展、heartbeat、idle duration 和 timeout policy。

任务详情同时返回 `elon.supervision_performance_timing.v1`。它从 task journal 的持久事件拆分 `queue`、`cli_startup`、首次结构化输出、首次有效进展、`active_execution`、`recovery`、重复进程派发、外部/未归因等待和 `supervisor_review`；helper 另报告单次 `submit_ms`、`inspect_ms`、`review_ms`。`active_execution` 只累计可配对的 `codex_item started/completed` 区间，纯 heartbeat 等待只推进持久观测上界并归入外部/未归因等待，不能冒充 CLI 有效执行。done、failed、canceled、resume_required 的 runtime 观测以 journal 持久终态更新时间封顶，终态不再返回 timeout deadline、remaining 或 timeout reason。历史 journal 没有新字段时同样使用既有持久 `updated_at_ms`，读取不会把终态后的墙钟时间计入 execution、idle 或 phase。

节点更新默认先检查活跃监督任务：有任务时写 `elon.local_supervision_restart.v1` 检查点并排空，安全终态后自动更新；无任务才立即应用。新进程把 recovery v1 回执、restart checkpoint 和任务 journal 合并为单调恢复事务：安全任务自动回到 `running`，继续增长游标并到正常终态，不需要手动 `Resume`。只有身份、租约、工作区或恢复回执无法闭合时，任务和 worktree 才保留为 `resume_required`，`/api/status` 与维护 API 返回不含 token 的 `resume_actions`；该状态不能覆盖已经确认的 `resumed` 或任务终态。

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

### Desktop review 凭据引导与恢复

生产环境默认使用自动安装的 v3 Desktop review broker：NodeAgent 每次启动生成只存在于进程内存的 RSA-3072 私钥，通过拒绝远程连接的 Windows named pipe 提供签名代理。代理从 pipe 句柄取得不可伪造的真实 client PID，沿进程链拒绝 `elon-cli-worker` / NodeAgent executor，并只接受位于受保护 WindowsApps `OpenAI.Codex_*` 包中的 Codex Desktop 祖先进程。即使 Desktop 与 PC executor 使用同一 Windows SID，executor 直接连接也稳定返回 `desktop_review_executor_ancestry_denied`，不会得到私钥或可复用签名能力；外部本机管理员仍属于操作系统信任根，不把同管理员进程隔离夸大为 Windows 安全边界。

broker 签发的 ticket 仍以长度前缀字段无歧义绑定 owner、task、HTTP method、规范 endpoint path、实际 UTF-8 body 的 SHA-256、expiry、nonce 和 key id。NodeAgent 先对原始 body 验签并消费持久、有界的 nonce 账本，之后才反序列化；重启自动轮换内存密钥，不需要 StateRoot、环境变量或人工复制凭据。`/api/status.desktop_review_broker` 只公开协议、可用性、pipe 名和隔离策略，不公开密钥或 ticket。helper 与节点共同声明 `desktop_review_broker_v1` 时优先走 broker；broker 声明可用但无法连接时稳定返回 `desktop_review_broker_unavailable`，不会回退到 PC operator review。

显式 Desktop 独立 Windows 身份仍可使用原 v3 证书兼容模式：通过 `-DesktopReviewStateRoot/-DesktopReviewInstallRoot` 或同名环境变量提供路径；缺少 broker 能力且未配置路径时返回 `desktop_review_paths_not_configured`，signer 缺失/ACL 或私钥不可访问分别返回 `desktop_review_signer_missing` / `desktop_review_signer_unavailable`。不得用同用户 DPAPI、User/Machine 环境变量或 `node-agent.env` 保存共享秘密。公钥模式启用后禁止回退 v1；v2 仅在 `ELON_DESKTOP_REVIEW_ALLOW_V2=1` 的明确短期迁移窗口接受，默认关闭。nonce 账本损坏、不可写或达到上限时返回 `desktop_review_nonce_ledger_unavailable` 并 fail-closed，重放返回 `desktop_review_ticket_replayed`。

仅在需要跨进程持久证书兼容模式时，运维才同时显式传入 `-StateRoot <Desktop 身份专属状态目录>` 与 `-InstallRoot <NodeAgent 安装目录>`。两条路径会冻结进 Desktop review 状态，后续 Validate、Commit、Rollback 和 ticket 签名路径不一致时一律 fail-closed。先以 `desktop-review-credential.ps1 -Action Prepare` 在 Desktop 身份的 `Cert:\CurrentUser\My` 生成暂存密钥并收紧状态目录和 CNG 私钥 ACL，再 `-Action Validate` 真实复核证书、owner、ACL 与身份，经 Desktop 独立审查后执行 `-Action Commit`；随后先重启 NodeAgent 读取 InstallRoot 中的公钥，再重启 Desktop/helper。两种模式都要求节点与 helper 声明 `desktop_review_ticket_v3`；仅声明 v2/v1 的旧节点保持业务任务与 journal，但 helper 不发送 review。

轮换使用 `-Rotate`，commit 时 NodeAgent 同时接受新旧公钥形成双版本窗口；确认所有 Desktop helper 已切换后下一次维护移除旧公钥。`-Action Diagnose` 只报告存在性、兼容与重启状态，不输出密钥、ticket 或签名。失败时旧 active 状态保持不变；`-Action Rollback` 原子恢复上一个公钥集合，之后由运维人员按 NodeAgent → Desktop 顺序受控重启。正式安装前必须确认 Desktop SID 与 executor SID 不同，并单独验证 executor 调用 `new-desktop-review-ticket.ps1` 被密钥 ACL 拒绝。

- 本机管理 API 继续要求动态 admin token 和受信 Origin；Skill 不打印 token。
- 执行 prompt 带 `<elon-pc-executor>`，执行者看到后禁止重新派发，避免递归。
- 契约使用 journal 事件持久化，没有为旧任务新增强制数据库列。
- `resume_original` 必须指向已终止的同节点、同项目受监督父任务，并复用节点记录的隔离现场；resume-of-resume 还必须证明父任务是当前协议的 `resume_original`、父子 `root_task_id` 一致、记录分支与平台路径相互对应。正常现场校验 Git worktree 注册和匹配 root lease；仅为兼容历史节点，完整可信谱系中的旧后代 lease 可在无临时解锁窗口的原子门禁内一次性迁移，任何通用/陌生/身份不明 lease 仍拒绝。注册丢失但原目录仍在时只允许权威 `requirement` 根做受控迁移：记录 HEAD 到现分支 HEAD 的后继关系、`origin/main` 归属、原目录零差异、owner/install/project/root/common-dir/remote/path/branch 和全部同根 registry/lease 关联必须同时成立；平台创建新 worktree 并迁移 provenance，不原位重建、不移动或覆盖孤儿目录。可信独占且 Git 注册仍有效的脏现场仍可原位恢复并保留三类修改；非法继承、身份漂移和非终态占用都会以冲突拒绝。
- PC 工作台只在 `supervision.enabled=true` 时显示监督卡，普通本机任务不受影响。
- 版本发布、灰度、回滚和节点兼容仍以 `docs/node-agent-upgrade-compatibility.md` 为准。
- 任何协议升级必须新增版本号或保持向后兼容，并补 Rust、PowerShell 与 PC 前端测试。

## 成功标准

一次监督任务只有同时满足下列条件才算完成：PC 执行任务到终态；桌面端拿到可复核证据；项目验证与发布规则通过；桌面端写入 `accepted`；没有未处理的阻塞能力缺口。长期质量看任务成功率、首包/总耗时、失败工具率、恢复成功率、验收驳回率和回归率，而不是“自动修改次数”。

### 验证证据与 smoke 范围冻结

监督任务的 acceptance criteria 在派发时冻结。Rust/Cargo 验证必须通过 `scripts/validate-rust.ps1`，journal/review 记录指纹、证据路径、owner/queue/resource class 和 reused 状态；流式输出截断时读取首次持久证据，不得仅因此重跑。pre-push 复用同一精确成功指纹。

真实 smoke 若发现相邻缺陷，review 使用结构化 `needs_follow_up` 或 `blocked_capability`，记录 `scope_relation`、`directly_blocks_frozen_acceptance`、证据和建议动作。只有直接阻断冻结标准时才纳入当前修复/发布循环。平台能力修复仍以相同 `root_task_id/parent_task_id` 关联原业务 UI，并在修复后 `Resume` 原业务会话；不得拆成与业务完全分离的休眠任务。
