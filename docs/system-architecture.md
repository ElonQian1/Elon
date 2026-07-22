# 系统架构详细设计

> 本文档供 AI 代理按需读取，描述云端APK开发平台的完整系统架构。

---

## 1. 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                        用户手机                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  一龙 APK                                              │  │
│  │  - AI 对话界面（自然语言输入）                          │  │
│  │  - 任务进度展示（编译中/部署中/完成）                   │  │
│  │  - APK 下载/更新入口                                   │  │
│  │  - 后端服务器版本展示                                  │  │
│  └────────────────┬──────────────────────────────────────┘  │
└───────────────────┼─────────────────────────────────────────┘
                    │ HTTPS / WebSocket
                    ▼
┌─────────────────────────────────────────────────────────────┐
│                      服务器                                   │
│                                                             │
│  ┌─────────────────┐    ┌──────────────────────────────┐   │
│  │  Rust API 服务   │    │     AI 对话处理模块           │   │
│  │  - 接收用户请求  │───►│  - 调用 LLM 理解需求          │   │
│  │  - 任务队列管理  │    │  - 生成代码修改方案            │   │
│  │  - 状态推送      │    │  - 规划执行步骤               │   │
│  └────────┬────────┘    └──────────────┬───────────────┘   │
│           │                            │                    │
│           ▼                            ▼                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                代码修改执行层                         │   │
│  │                                                     │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │   │
│  │  │ Rust代码  │  │ Android  │  │   前端代码        │  │   │
│  │  │ 修改模块  │  │ 代码修改  │  │   修改模块        │  │   │
│  │  └──────────┘  └──────────┘  └──────────────────┘  │   │
│  └─────────────────────────┬───────────────────────────┘   │
│                            │                               │
│                            ▼                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 自动化流水线                          │   │
│  │                                                     │   │
│  │  git commit → 本地开发机编译 → 上传产物到服务器     │   │
│  │       → 重启后端 / 分发APK → 生成下载链接            │   │
│  └─────────────────────────┬───────────────────────────┘   │
│                            │                               │
└────────────────────────────┼───────────────────────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │  APK 下载链接   │
                    │  → 推送回用户  │
                    └────────────────┘
```

---

## 2. 数据流详解

### 2.1 用户发起需求

```
用户输入: "帮我在首页加一个红色按钮，点击后显示'你好'"
    │
    ▼
APK 客户端 → POST /api/v1/conversation
    {
      "user_id": "xxx",
      "session_id": "yyy",
      "message": "帮我在首页加一个红色按钮，点击后显示'你好'"
    }
```

### 2.2 AI 理解需求，规划修改

```
AI 分析结果:
    {
      "type": "android_ui_change",
      "files_to_modify": ["android/app/src/main/res/layout/activity_main.xml"],
      "description": "在 activity_main.xml 中添加红色按钮",
      "git_message": "feat: 用户xxx请求 - 首页添加红色按钮"
    }
```

### 2.3 执行修改 → 编译 → 部署

```
Step 1: 修改 Android 布局文件
Step 2: git add . && git commit -m "feat: 用户xxx请求 - 首页添加红色按钮"
Step 3: 在本地开发机触发 Android 编译 (./gradlew assembleRelease)
Step 4: 本地签名 APK (apksigner / Gradle signingConfig)
Step 5: 上传 APK 产物到分发服务器
Step 6: 生成唯一下载链接
Step 7: 通过 WebSocket 推送给用户
```

### 2.3.1 指定手机的无人值守 ADB 更新

`publish-apk.*` 在服务器 APK 原子发布、版本验证和 release finish 全部成功后，会检查发布机本地的 `~/.elon/apk-adb-targets.json`。只有该文件显式列出的手机才会执行 `adb install -r`；不会把 `adb devices` 中偶然出现的其他手机当成部署目标。

```json
{
  "schemaVersion": 1,
  "enabled": true,
  "packageName": "com.elon.app",
  "maxAttempts": 3,
  "retryDelaySeconds": 5,
  "launchAfterInstall": true,
  "targets": [
    {
      "label": "一龙测试手机",
      "serial": "192.168.31.171:5555",
      "hardwareSerial": "手机 ro.serialno"
    }
  ]
}
```

`serial` 是当前 ADB 连接端点，`hardwareSerial` 是 `adb -s <serial> shell getprop ro.serialno` 返回的稳定硬件身份。两者必须同时匹配，防止无线 ADB IP 被其他手机复用时装错设备。安装后脚本会校验 `versionCode`、自动拉起 APP，并输出 `APK_ADB_DEPLOY_STATUS=updated`。指定手机连续重试后仍失败时发布命令返回非零，同时明确保留“服务器 APK 已发布”的事实。可用 `ELON_APK_ADB_TARGETS_FILE` 指定其他本机配置路径，用 `ELON_ADB_PATH` 覆盖 ADB 程序路径。

### 2.4 PC 节点 AI 运行路线

PC 项目会话按三层拆分：运行路线决定 AI / 模型来源和项目执行位置；CLI 会话 / 传输模式决定 node-agent 如何连接 CLI；前端展示 / 恢复模式决定网页端如何展示公开过程、折叠最终回复、恢复或接管任务。

PC 项目会话的 AI 运行路线分为五类：

| 路线 | 请求值 | AI / 模型来源 | 项目文件与命令执行位置 |
|---|---|---|---|
| 本机AI | `route_a` | 项目绑定 PC 上已登录的 Codex / Copilot / Claude / Gemini CLI | 项目绑定 PC 节点 |
| 我的Key | `route_b` | 项目绑定 PC 上配置的 OpenAI-compatible API key | 项目绑定 PC 节点的一龙工具 runtime |
| 平台AI | `route_c` / `route_c1` | 一龙平台提供模型能力 | 项目绑定 PC 节点的一龙工具 runtime |
| 远程AI | `route_c2` | 其他用户 PC 节点的 API runtime | 被授权的远程 PC 节点 |
| 远程Codex | `route_c3` | 其他用户 PC 节点已登录的 Codex / Claude / Copilot 等 CLI | 被授权的远程 PC 节点 |

项目会话默认优先 `route_a`，让本机 Codex CLI 自己读取项目规则、判断是否需要读文件、修改代码、运行命令或构建。PC 前端的“强制 Codex / 直连”开关会把本轮请求强制成 `route_a`，并传入当前本机节点和项目工作区路径。

开发阶段新项目的 `runtime_permission` 默认是 `full_access`。对 Route A Codex 来说，它沿用既有 `danger_full_access` CLI 语义，启动参数会关闭 Codex sandbox/逐次审批；但节点仍要求本机 grant，并按当前登录 owner、node credential、install、project 和规范化 workspace 绑定授权。PC 前端在缺少 grant 时只会为 `/api/cloud-projects` 已证明属于当前节点且项目/目录完全匹配的绑定自动调用授权 API，继续发送 `confirm_full_access=true` 供节点审计，不使用浏览器 `window.confirm`。未登录、未绑定、节点或目录不匹配仍拒绝；显式 `project_write` 设置继续保留。

Codex Pro `auth.json` 云端保险箱默认只属于账号所有者自己的备份/恢复能力：本机节点上传密文，云端只存 AES-GCM 密文，本人节点用用户 token + 节点 secret 恢复到托管临时 `CODEX_HOME`。普通 Route C3 共享仍不允许远程用户下载、恢复或复制 provider 的 `auth.json` 明文，只允许把任务派发到 provider 的 PC 节点。

医疗机器人等高可用/协作场景使用 Codex 保险箱授权共享：provider 机器人必须在平台上显式授权 consumer 机器人，consumer 自己的在线节点必须用节点 secret 证明身份，云端才会下发短 TTL 租约。共享不要求一定是紧急场景，但必须是短租约、可撤销、可审计、可计费的授权行为。节点只把租约写入托管临时 `CODEX_HOME`，不覆盖默认 `~/.codex/auth.json`。租约、provider/consumer、槽位、token 账单和 provider 收益分别记录在 `codex_vault_emergency_grants`、`codex_vault_emergency_leases`、`token_usage_events` 和 `node_transactions`，计费来源统一为 `shared_codex`。表名保留 `emergency` 是历史兼容名，产品和 API 新入口使用 `sharing` 语义。

Route A 本机 CLI 是否使用 PTY 是 CLI 会话 / 传输模式选择，不是新的运行路线。Route A / Route C3 都可以在对应节点里选择下面的传输模式：

| 模式 | 当前是否具备 | 定位 |
|---|---|---|
| `pipe_sidecar` | 已具备，Codex JSON 默认 | sidecar 管进程生命周期、取消、journal、session id 和恢复入口；stdout/stderr 仍保持干净 pipe |
| `direct_json_pipe` | 已具备，回退路径 | 直接启动 `codex exec --json`，读取 stdout JSONL / stderr，并生成结构化过程事件 |
| `pty_sidecar` | 已具备，辅助路 | 用 portable_pty / ConPTY 管真实终端，适合 TUI、人工接管、resize、交互输入和终端型 CLI |

当前 Codex CLI 的后台开发主链路使用 `pipe_sidecar + pipe + JSON`。它比把 Codex JSON 放进 PTY 更适合后台结构化过程展示：sidecar 管生命周期和恢复，stdout/stderr 仍保持干净 pipe，避免终端画面污染机器事件流。

当前边界是：Codex CLI 自己负责项目理解、命令执行、文件修改和最终回答；一龙平台负责排队、并行、取消、重连、journal、恢复和前端过程展示。`pipe_sidecar + pipe + JSON` 已补齐基础生命周期管理、稳定运行句柄、恢复契约和 Codex JSON 输出回放；后续继续增强前端恢复入口和多 CLI 统一管理，不替代 Codex CLI 本身能力。

sidecar `sessions.json` 使用进程间互斥和同目录唯一临时文件原子替换，更新时保留最近有效备份；临时文件只有在本进程成功独占创建后才允许清理，Windows `MoveFileExW` 的短时 sharing/lock、空间压力和资源压力错误会有界退避重试，写后还要重新读取校验。主文件损坏会在锁内从备份回退并原子重建；主文件与备份同时持续损坏时显式失败，不能伪造成空 registry。实时输出跟随只把 sessions 游标视为可重放检查点并限频写入：单次写入失败会保留 task journal、隔离工作树和 sidecar JSONL，继续排空到真实 CLI 终态并重试；终态时仍无法持久化才报告可恢复的明确失败，不在 CLI 仍执行时提前杀死监督任务。父节点同时监视 worker：worker 若在生成终端输出前异常退出，会补写失败终态，让本机任务快速进入可恢复的 failed/resume_required 链路，不会永久停在 running/recovering。

#### 2.4.1 Codex Desktop -> PC 本机监督协议

本机任务 API 可选接受 `supervision`，协议版本为 `elon.desktop_pc_supervision.v1`。契约记录 `task_role`、父/根任务、验收条件和改进策略；node-agent 把契约及桌面验收作为 append-only journal 事件保存，不改变旧任务的数据库结构，也不影响未传 `supervision` 的调用方。

节点从 Codex JSON `item.started/item.completed` 的 `command_execution`、`file_change`、`agent_message` 生成确定性 `supervision.evidence`，包括工具调用/结果、失败工具、命令退出码与失败摘要、文件变更、agent message 和终态。高输出 stdout/stderr 按流聚合，只保存原始行/字节计数、首尾和截断标记，不让一次 `rg` 膨胀成数百 journal 事件。桌面端仍需结合 Git、测试和发布事实做独立判断，然后调用 `/api/local-tasks/:task_id/supervision/review` 写入结论。PC 工作台同时展示 phase、脱敏 current command、last progress、heartbeat、idle duration 和 timeout policy。

`task_role` 形成可追溯任务树：`requirement` 是原需求，`capability_repair` 是阻塞能力修复，`resume_original` 是修复后的原任务续跑，`post_task_improvement` 是任务完成后的非阻塞增强。节点给执行 CLI 注入防递归标记，避免桌面监督与本机执行相互重复派发。完整安全边界、API 示例和日常流程见 `docs/codex-desktop-pc-supervision.md`。

受监督的本机 Codex 任务需要覆盖真实项目构建、发布和统一收尾，不能沿用普通 full-access 任务固定的 1200 秒总时限。pipe sidecar 与 direct-pipe 回退路径统一使用默认 21600 秒总时限、900 秒进展空闲时限和 15 秒 heartbeat；三个值分别由 `ELON_SUPERVISED_CODEX_TIMEOUT_SECS`（1201–86400）、`ELON_SUPERVISED_CODEX_IDLE_TIMEOUT_SECS`（30–7200）、`ELON_SUPERVISED_CODEX_HEARTBEAT_SECS`（1–60）约束配置。输出/命令/文件刷新进展，节点 heartbeat 只证明运行时存活，不掩盖真正空闲。未带监督协议的 Codex 任务仍为 1200/300 秒，其它 CLI 仍为 180 秒；取消、进程树回收、审批、云控截止时间和终态持久化不放宽。

`resume_original` 只允许复用已终止父任务由节点记录的同项目隔离 worktree。节点会重新验证父任务 owner/agent/install、监督协议、项目、授权基础仓库、平台 worktree 路径形状、Git common-dir、登记分支、终态记录的 `git_head` 和当前独占占用；任一信息缺失、伪造、跨项目、父任务仍活跃或 worktree 已被其它 CLI 占用时都拒绝续跑。父任务本身是 `resume_original` 时不会用其新生成的本机 conversation id 覆盖继承身份；只有父子监督契约的 root identity 一致，且记录分支、平台路径、Git 身份、HEAD 与 root lease 仍完整吻合，才允许 resume-of-resume 继续复用同一现场。若活动目录仍在但 `.git` 指向的注册已丢失，节点绝不移动或原位修补该孤儿目录；只读门禁必须证明记录 common-dir/remote/path/branch、分支 HEAD 是记录 HEAD 的后继且已进入 `origin/main`、孤儿业务内容与目标提交无差异，并把同 root 的每个历史 lease 关联到同 owner/node/install/project 的终态 registry 任务。全部成立且无 live prompt/sidecar 后，Resume 准入锁内创建新的平台 conversation worktree、迁移 requirement 根任务 provenance、回收已证明终态的历史 lease，并让新 child 立即持久化继承 Git 身份；任一漂移或非终态占用继续 fail closed。全访问授权仍锚定父任务原基础仓库，不因迁移活动 worktree 而扩大。

用户明确改变需求时不修改根合同，也不把旧 worktree 当作新任务路径重新提交。Desktop 使用 `Supersede` 创建带 `elon.supervision.contract_revision.v1` 收据的 `resume_original` 子代；节点完成相同的父链、项目、Git、租约和独占校验后，记录上一有效需求摘要、新完整需求、新验收条件和变更原因。后续 Resume 最多沿 64 代不可变父链重建最近有效修订；摘要、父链、根身份或收据冲突时拒绝执行。

节点在创建或复用受监督隔离 worktree 时直接以 `elon-supervision:<root_task_id>` 写入唯一权威 Git lease；prepare、恢复和 merge 不先写通用锁，也不存在临时解锁窗口。为兼容历史节点曾写入的监督后代任务 lease，Resume 路由会先逐代验证持久监督契约直到同一 `requirement` 根，并核对 owner、节点、安装、项目、授权基础工作区、Git 身份和独占状态；只有全部一致时，才持有仓库级跨进程准入锁并用同目录原子替换把 Git `locked` 原因迁移为 root lease，通用锁、陌生 lease 或断裂谱系一律拒绝。该准入锁持续到新任务在活跃工作区注册，避免并发 Resume 穿透。可信终态在执行句柄退出后按 root identity 精确解锁；启动与周期维护幂等回收已有终态陈旧锁，运行态、错误 lease 和未知文件均保留。Review 独立记录验收结论并保留兼容解锁入口，但不再是唯一释放入口；解锁后只有 clean 且已合入的 worktree 才能由通用 cleanup 回收。普通非监督 conversation worktree 仍使用原通用锁，并在成功合并后解锁删除。Windows 超时/取消先同步等待 `taskkill /T /F` 回收完整执行器进程树，再结束直接子进程。启动时 `workspace_status` 先持久化基础路径、活动路径、分支、完整 `git_head` 和 Git 身份锚点；进入可靠终态前，节点在同一持久事务内重新验证路径、branch、common-dir、remote、root lease、节点/安装/owner/project 和占用身份，并原子刷新为当前 HEAD。验证失败时保留启动证据并持久化不可 Resume 原因，为注册丢失后的确定性重建提供 fail-closed 身份。

#### 同节点 Android 多会话合并调试

Codex 会话继续在独立 conversation worktree 开发和自行编译，真机部署则进入节点托管的第二层集成槽。HTTP、MCP 和 PC 工作台提交候选时必须明确 `ready`，提供或由兼容入口确定当前 HEAD，并证明来源 worktree 干净、提交属于固定基础 SHA 的后继、来源 task/session 可审计。协调器在节点数据根的 `workspaces/android-debug-integration` 下创建代次专属 detached worktree，按接收顺序 cherry-pick；不复制脏文件、不直接混写多个 worktree，也不猜测解决冲突。

槽身份是“Git common-dir + 项目 + 物理硬件序列号 + 稳定节点指纹”。状态原子记录基础 SHA、贡献提交顺序、来源、期望代次、已安装代次、冲突、共享预览所有者、APK SHA-256，以及最近成功版本策略是否启用。最近成功版本（LKG）是 opt-in：HTTP/MCP/PC 工作台的任务请求只有显式传入 `lkgEnabled=true` 才记录、推进并按已有签名钉扎语义校验 `lastUsable`；字段缺失、旧状态缺字段或普通任务均视为“未启用”，不创建、不推进、不校验、不要求 LKG，也不能阻塞构建、ADB 覆盖安装或任务统一收尾。状态接口必须同时返回 `lkgEnabled`，PC 工作台在关闭时明确显示“未启用”，不得把空 LKG 暗示成门禁失败。合并、构建、APK 身份校验和安装仍由同一物理设备/包互斥锁串行；新候选提高代次，旧构建在暂存和安装前用 fencing token 再校验，因而不能在新构建之后安装。各代构建输出保持隔离，协调器只把通过 `aapt`/`apksigner` 校验的包名、标签、版本、签名和内容哈希原子暂存为 `<sha256>.apk`，不会让多个会话写同一路径。

真机最终包名固定为 `com.elon.app.uituner_<节点指纹>`；`.uituner`、`.uitest`、`.uitest_anim` 及旧带指纹调用都只作为兼容输入。只有 `emulator-*` 且调用者显式设置隔离选项时，才保留模拟器独立测试包。正式 `com.elon.app` / “一龙ai”不变。非法后缀、非 ready/未提交候选、合并冲突、过期代次和设备离线均返回明确状态；LKG 启用时额外执行最近成功 APK 的签名钉扎和保留语义，未启用时只执行当前 APK 自身的包名、标签、版本、签名与哈希校验。两种模式都不创建新真机包，不自动卸载旧包，失败时手机当前版本保持不变。升级前遗留的杂包只进入诊断报告，由用户明确决定是否处理。

本机低优先自进化是独立调度域：父用户任务先终止，节点再预创建并持久化独立 task/conversation/worktree。pause/review 操作采用“先持久 action intent、再执行外部取消或审查、最后原子提交队列状态”，所以重试不会产生半状态；配额类失败进入有界退避。取消/让路事件持久化四元组和可信 `interruption_source`，审计失败即拒绝取消。发布控制面同样使用持久的 `batch_id + immutable SHA` 阶段 ledger，把 server、PC 前端、Windows 节点的 owner、waiter、heartbeat、attempt 与错误统一成可接管且 fail-closed 的事务视图。

服务器频繁发布重启时，Route A 任务不应把“后端进程重启”直接当成用户任务失败。短期发布排水只做很短的停止接新和状态落盘窗口，不能等待 Codex 长任务自然结束；长期目标是任务可恢复：云端保存 `task_id`、`pc_req_id`、`agent_id`、会话/sidecar 信息和最后公开进度，重启后进入 `recovering` 状态，节点重连后通过本机 journal / Codex session 回放或续接。前端文案应表达“服务器正在更新升级，任务已保留，正在恢复/已恢复/恢复失败可重试”，只有节点确认无法恢复时才转为失败。

当前 Route A 任务恢复闭环分为四段：

1. 云端快照：服务端持久化 `task_id`、`pc_req_id`、`agent_id`、任务事件、最后公开步骤和云端 `attach` 状态；服务器重启时把未完成任务标记为 `recovering`，而不是立即写成普通失败。
2. 本机 journal 查询：任务快照接口会通过节点 WebSocket 协议发送 `InspectCliTaskJournal`，让 Win 端按 `pc_req_id` 读取本机 task journal、sidecar session、Codex session/thread、审批状态和可续接合同；这个查询不绕过本机管理 HTTP token，也不依赖浏览器本地端口。
3. 前端恢复展示：PC 网页端加载会话时会把本机 journal 快照合成为 `runtime_status` 公开过程，显示“正在恢复 / 需要继续 / 本机 journal 暂不可达”，并把 `attach`、`resume`、`approval_state`、`last_event_seq` 注入任务状态，继续用于过程折叠和恢复按钮。
4. 失败收口：如果自动恢复在预期时间内没有完成，服务端会先写入 `resume_required` 公开过程，再写最终恢复失败说明。用户看到的是“服务器或 Win 端正在更新升级后未能自动恢复，请点击继续让 AI 检查当前工作区后接着处理”，而不是普通 CLI 无输出超时。

仍需区分“可恢复现场”和“强恢复执行”。当前已经能做到恢复可见、journal 回放、sidecar/审批状态快照和继续入口闭环；但它不是任意外部终端进程的强制热迁移。强恢复必须满足 Win 端在线、节点版本支持 `InspectCliTaskJournal`、本机 journal 未损坏、Codex session/thread id 可用。对于已经脱离云端流式连接的进程，平台优先给出 `continue_from_snapshot`，由新的 Codex 轮次先检查工作区和 journal 后继续；后续增强方向才是更自动地把 Codex 原生 `resume <SESSION_ID>` 接进续接执行。

节点客户端自更新属于一龙控制面链路，默认必须直连一龙服务器，不能继承开发机或子项目设置的系统代理；确需代理的环境用 `NODE_AGENT_UPDATE_USE_SYSTEM_PROXY=1` 或 `NODE_AGENT_UPDATE_PROXY_MODE=system` 显式开启。

Win 端启动和更新的产品闭环如下：

1. PC 网页端只负责提示和触发：节点未运行时展示“启动 Win 端 / 下载 / 节点设置”；节点已运行但版本落后时展示“Win 端可更新 / 下载新版 / 节点设置”。
2. 浏览器通过 `elon-node://open` 拉起本机 Win 端；如果协议未注册或本机未安装，用户走标准下载地址 `/api/node-agent/download/windows-client`。
3. 节点设置页读取 `/api/node-agent/version` 和本机 `/api/client-maintenance`，用服务器版本、包大小、本机安装状态判断“未知 / 最新 / 可更新”。
4. 用户点击“更新并重启 Win 端”或云端广播更新时，节点先检测活跃监督任务。有任务则持久化 `elon.local_supervision_restart.v1` 并 drain 到安全终态，无任务才下载替换；计划重启恢复为 `runtime_online`，意外重启留下 `resume_required` 与无 token 的 Resume 动作。网页端轮询 `/api/status.restart_recovery` 确认排空、重启和恢复状态。
5. 服务器更新频繁时，网页端不能把“服务端有新版本”误报成任务失败；任务页继续展示当前 CLI 公开过程，节点/设置页单独提示客户端版本维护。长期目标是在 Win 端启动时主动比对服务器版本并提示/自动维护，但用户可见入口仍保持在节点设置页。

```
PC 网页端 → Rust server → node-agent → pipe sidecar → codex exec --json
    → stdout JSONL item.started / item.completed 事件流
    → 节点解析 command_execution / file_change / agent_message / usage / final_reply
    → PC 网页端任务过程卡片
```

这条链路默认不走 PTY/ConPTY。PTY 是给人看的终端画面，适合终端接管、TUI、人工输入、resize、取消和调试；`codex exec --json` 是给程序读的结构化事件流，进入 PTY 后可能被终端折行、ANSI 控制序列或提示文本污染。节点默认 `ELON_CODEX_JSON_DIRECT_STDOUT=1` 且 `ELON_CODEX_PIPE_SIDECAR` 默认开启，因此 Codex 进入 `managed_pipe_json_sidecar`；设置 `ELON_CODEX_PIPE_SIDECAR=0` 可回退到直接 pipe，设置 `ELON_CODEX_JSON_DIRECT_STDOUT=0` 才回到旧 PTY sidecar 路径。

---

### 2.5 项目文档治理与 MCP

项目真实目录和 Markdown 始终是内容真源。PC 网页端把知识呈现为四条正交轴：产品功能回答“用户能做什么”，技术架构回答“系统怎样实现”，OneNote 式主题树回答“文档讲什么”，治理总览分别回答“是否读取、是否当前、能否作为事实、是什么类型”。`.elon/document-sections.json` 保存知识首页、主/副主题、四维治理、有类型的文档关系以及功能/技术节点和实现证据；旧的治理分区只作为快捷投影，它不等于实际移动文件。AI 建议单独保存在 `.elon/document-organization-suggestions.json`，按统一权限模式应用。

Windows 节点提供项目绑定、短期令牌保护的标准 Streamable HTTP MCP。`project_docs_analyze` 复用路径权威性分类器并保持零模型 token；图谱和阅读计划工具限制节点数与 token，正文仍只为歧义或缺失入口按需读取。问题工作流工具支持筛选、分派、期限、原因化忽略/延期和趋势；版本工具同时支持普通 Git 项目与托管知识库的文档差异和新提交式恢复。图谱与 PC 页面由 Rust 后端生成同一份机器契约；文档覆盖和实现证据分别计算，不能由前者推断后者。保存和应用均使用 revision 防并发覆盖，默认由整理前/整理后两个仅文档 Git 提交保护。详细契约按需读取 `docs/project-document-governance-mcp.md`。

持续知识治理由工作区外 SQLite 派生索引、持久变更事件和 60 秒维护轮询组成。质量内核检查链接、孤立文档、owner、复查周期和显式实现引用；问题状态与健康快照同样位于工作区外。服务端把架构、质量、维护、问题工作流和联邦节点合并为目录快照中的 `analysis`，并返回可解释评分组成。大型仓库通过 `.elon/knowledge-federation.json` 的路径与 glob 分层；个人笔记通过 `vaultId` 映射到托管 Git 知识库，每次保存自动提交且可恢复，用户界面不暴露 Git 复杂度。

### 2.6 PWA Runtime 像素证据

Windows 节点复用 `yilong_ui_live` Streamable HTTP MCP，通过 `ui_capture_pwa_runtime` 在本机启动受控的无头 Edge、Chrome 或 Chromium。它使用浏览器 CDP 渲染真实 http(s) 页面并生成 PNG，不依赖 Codex Desktop Browser、可见浏览器、桌面点击或 DOM/CSS 摘要。MCP 会话必须绑定项目 `EDIT_ROOT`；同一内核也由 `/api/source-preview/capture-pwa-runtime` 调用，因此 PC 画面模块、Codex CLI 与 MCP 共用 URL、认证、浏览器、工件和脱敏策略，旧 Android Renderer 工具保持不变。

默认只允许 `localhost` 和 loopback URL，导航及所有 http(s) 子请求都受 origin allowlist 拦截。项目若确需同一受信部署源，可在 `.elon/ui-pwa-runtime.json` 中显式登记最多 16 个 `allowedOrigins`，并可设置 `defaultAuthProfile` 与 `authenticatedReadySelector`；该文件不得保存秘密。认证材料只能放在本机项目 `.elon/ui-tuner/pwa-sessions/<profile>.json`，由 MCP 参数中的安全 profile 名引用；显式准备合同是 `{"version":1,"cookies":[{"name":"session","value":"<local-only>","path":"/","httpOnly":true,"secure":true}],"headers":{},"localStorage":{}}`，也只允许 `Authorization` 或 `X-*` header 以及最多 16 个受限 localStorage 键值。localStorage 只在目标 origin 新文档启动前注入到浏览器一次性用户目录，浏览器结束后连同临时 profile 一并回收；目录已被 Git 忽略。URL userinfo、疑似 token/secret/session/signature 的 query、直接 Cookie/Authorization 参数、链接/重解析点越界、超限 viewport/等待/PNG 都会返回机器可读诊断；登录表单、401/403 或认证就绪 selector 未出现不会被误报为成功画面。

节点依次探测 `ELON_PWA_BROWSER_PATH`、标准 Edge/Chrome 安装路径和 `PATH`。每次捕获使用独立临时 profile、随机 CDP 端口和隐藏窗口；成功、超时、协议错误或启动失败都回收浏览器进程树和临时目录。`BROWSER_NOT_FOUND` 时安装 Microsoft Edge/Google Chrome，或把受信浏览器绝对路径写入 `ELON_PWA_BROWSER_PATH` 后重启节点；`URL_ORIGIN_NOT_ALLOWED`、`AUTHENTICATION_REQUIRED`/`AUTHENTICATION_FAILED` 和 `BROWSER_CLEANUP_FAILED` 的 `nextStep` 是 CLI/PC UI 的权威恢复提示。

成功结果保存到项目 `.elon/ui-tuner/pwa-runtime/captures/`，返回 PNG 与 manifest 的绝对路径、SHA-256、实际像素尺寸、媒体类型、采集时间、脱敏 route、浏览器版本、viewport、网络门禁、进程回收和 source/route revision。PWA 源码闭环仍先完成构建、资源、真实 iframe route/source revision 校验，再安全自动请求 PNG；无法自动捕获时保留源码验收成功并显示明确下一步。Codex context pack 只引用工件路径、哈希和尺寸，`screenshotsEmbeddedAsBase64=false`，默认不嵌入图片 Base64。

## 3. 代码仓库结构（目标结构）

```
d:\一龙\
├── .github\
│   └── copilot-instructions.md   ← AI全局指令（永远加载）
├── docs\
│   ├── system-architecture.md    ← 本文件
│   └── ai-agent-workflow.md      ← AI工作流详细步骤
├── server\                        ← Rust 服务端
│   ├── src\
│   │   ├── main.rs
│   │   ├── api\                   ← HTTP API 路由
│   │   ├── conversation\          ← AI 对话处理
│   │   ├── pipeline\              ← 编译部署流水线
│   │   └── models\                ← 数据模型
│   └── Cargo.toml
├── android\                       ← Android APK
│   ├── app\src\main\
│   │   ├── kotlin\                ← Kotlin 源码
│   │   ├── res\                   ← 布局/资源
│   │   └── AndroidManifest.xml
│   └── build.gradle
├── frontend\                      ← Web 前端（如有）
│   ├── src\
│   └── package.json
└── scripts\                       ← 自动化脚本
    ├── publish-server.ps1         ← Windows 本地交叉编译后端并上传 binary
    ├── publish-server.sh          ← Linux/macOS 本地交叉编译后端并上传 binary
    └── publish-apk.ps1            ← 本地构建、签名并上传 APK
```

---

## 4. 多用户隔离方案

每个用户的修改需要隔离，防止冲突：

| 方案 | 说明 | 推荐场景 |
|---|---|---|
| **Git 分支隔离** | 每个用户在独立分支修改，合并到主分支前测试 | 生产推荐 |
| **代码沙箱** | 每个用户有独立的代码副本 | 高并发场景 |
| **任务队列** | 串行处理，同一时刻只有一个修改在执行 | 简单起步阶段 |

> 初期推荐：**任务队列** + **Git 分支**组合

---

## 5. APK 分发方案

当前一龙 APK 分发不是第三方托管方案，而是固定直链 + 版本信息 + 同 WiFi P2P mirror：

```
scripts/publish-apk.ps1
    │
    ├── 上传 /opt/elon/data/app/ElonSpeed-latest.apk
    ├── 上传 /opt/elon/data/app/version.json
    ├── POST /api/app/update/broadcast 通知在线客户端
    │
    ▼
服务器
    ├── GET /app/version.json
    │     ├── 读取磁盘 version.json
    │     ├── 重写 downloadUrl/downloadPageUrl 为公网地址
    │     └── 动态注入在线 seeder mirrors
    ├── GET /app/ElonSpeed-latest.apk
    ├── GET /app/peer/ws?version_code=N
    └── GET /app/relay/peer/{peer_id}/apk
```

相关实现：

| 模块 | 职责 |
|---|---|
| `server/src/app_update.rs` | 读取最新 `version.json` 并广播在线更新事件 |
| `server/src/peer_relay.rs` | 注册同 WiFi seeder、动态注入 `mirrors`、中继 APK 下载 |
| `android/app/src/main/kotlin/com/elon/app/update/AppUpdateManager.kt` | 拉取 `version.json`、尝试 mirrors、失败后回退服务器直链 |
| `android/app/src/main/kotlin/com/elon/app/update/PeerSeederManager.kt` | 已安装 APK 的手机连接 WebSocket，收到 `SEND_APK` 后发送本机安装包 |

P2P 分发维护规则：

- `version.json` 是 APK 分发事实来源；发布后必须校验公网 `/app/version.json`，不能只看本地生成文件。
- 当前 mirror 字段由 `server/src/peer_relay.rs` 动态注入，仅包含 `version_code >= 当前发布 versionCode` 的在线 seeder。
- 当前 Android 端按 `priority` 降序尝试 mirror；如果后续引入 dev-mirror 并希望采用“数字越小越优先”，必须同步修改 Android 排序、服务器注入规则和文档，然后按 APK 发布闭环发布。
- WebSocket 长连接必须使用无读超时或足够长读超时；服务器端遇到 Ping/Pong 等控制帧不能当作传输失败。
- 大 APK 中继要关注背压和内存占用。当前服务器会收集完整 APK 后再返回 HTTP 响应；若 APK 增大或并发增加，应改为流式转发，并在 Android WebSocket 发送端按队列大小节流，避免 OkHttp 写缓冲撑满导致截断。
- mirror 全部失败时必须保留 `downloadUrl` 直链兜底，避免 P2P 节点不在线影响普通更新。

历史备选方案：

```
编译完成的 APK
    │
    ├── 方案A: 自建文件服务器
    │         APK 存到 /var/www/apk/{version}/app.apk
    │         生成链接: https://download.example.com/apk/v1.2.3/app.apk
    │
    ├── 方案B: 对象存储 (OSS/S3)
    │         上传到 OSS，生成预签名 URL（有时效性）
    │
    └── 方案C: pgyer / 蒲公英 等第三方分发
              调用 API 上传，返回下载页面链接
```

---

## 5.1 版本信息通道

一龙区分 APK 版本和后端服务器版本：

| 类型 | 来源 | 接口 | 用途 |
|---|---|---|---|
| APK 版本 | `android/app/build.gradle` + 发布脚本生成的 `version.json` | `/app/version.json` | Android 自更新、下载页、P2P APK mirrors |
| 后端版本 | `server/Cargo.toml` 的 `package.version` + 构建时注入的 git SHA | `/api/server/version` | APK 个人页展示服务器版本，用户可见后端已更新 |

后端运行代码变更必须递增 `server/Cargo.toml` 版本号并走服务端部署脚本；部署脚本负责注入 git SHA，重启后验证 `/health` 和 `/api/server/version`。

---

## 6. 安全考虑

- **代码执行沙箱**：AI 生成的代码变更必须经过人工确认规则或自动安全扫描才能执行
- **APK 签名密钥**：存储在服务器安全存储，不随代码提交，只在 CI 步骤中注入
- **用户鉴权**：所有 API 需要用户 Token，对话内容不混用
- **代码审计**：保留所有 git commit 历史，可溯源每个修改的用户和时间
- **速率限制**：限制每个用户每天触发编译的次数，防止资源滥用
