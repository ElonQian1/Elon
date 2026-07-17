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

#### 2.4.1 Codex Desktop -> PC 本机监督协议

本机任务 API 可选接受 `supervision`，协议版本为 `elon.desktop_pc_supervision.v1`。契约记录 `task_role`、父/根任务、验收条件和改进策略；node-agent 把契约及桌面验收作为 append-only journal 事件保存，不改变旧任务的数据库结构，也不影响未传 `supervision` 的调用方。

节点返回的 `supervision.evidence` 是确定性摘要，包括事件数、工具调用/结果、失败工具、文件变更事件、文件名和是否看到终态事件。桌面端仍需结合 Git、测试和发布事实做独立判断，然后调用 `/api/local-tasks/:task_id/supervision/review` 写入 `accepted`、`needs_follow_up`、`blocked_capability` 或 `rejected`。PC 工作台的任务详情展示契约、证据和最新结论。

`task_role` 形成可追溯任务树：`requirement` 是原需求，`capability_repair` 是阻塞能力修复，`resume_original` 是修复后的原任务续跑，`post_task_improvement` 是任务完成后的非阻塞增强。节点给执行 CLI 注入防递归标记，避免桌面监督与本机执行相互重复派发。完整安全边界、API 示例和日常流程见 `docs/codex-desktop-pc-supervision.md`。

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
4. 用户点击“更新并重启 Win 端”时，网页端只调用本机 `/api/client-maintenance/update`；真正下载、替换、重启由 Win 端维护层执行，网页端轮询 `/api/status` 确认节点重新上线。
5. 服务器更新频繁时，网页端不能把“服务端有新版本”误报成任务失败；任务页继续展示当前 CLI 公开过程，节点/设置页单独提示客户端版本维护。长期目标是在 Win 端启动时主动比对服务器版本并提示/自动维护，但用户可见入口仍保持在节点设置页。

```
PC 网页端 → Rust server → node-agent → pipe sidecar → codex exec --json
    → stdout JSONL 事件流
    → 服务端解析 tool_call / tool_result / usage / final_reply
    → PC 网页端任务过程卡片
```

这条链路默认不走 PTY/ConPTY。PTY 是给人看的终端画面，适合终端接管、TUI、人工输入、resize、取消和调试；`codex exec --json` 是给程序读的结构化事件流，进入 PTY 后可能被终端折行、ANSI 控制序列或提示文本污染。节点默认 `ELON_CODEX_JSON_DIRECT_STDOUT=1` 且 `ELON_CODEX_PIPE_SIDECAR` 默认开启，因此 Codex 进入 `managed_pipe_json_sidecar`；设置 `ELON_CODEX_PIPE_SIDECAR=0` 可回退到直接 pipe，设置 `ELON_CODEX_JSON_DIRECT_STDOUT=0` 才回到旧 PTY sidecar 路径。

---

### 2.5 项目文档治理与 MCP

项目真实目录和 Markdown 始终是内容真源。PC 网页端把知识呈现为四条正交轴：产品功能回答“用户能做什么”，技术架构回答“系统怎样实现”，OneNote 式主题树回答“文档讲什么”，治理总览分别回答“是否读取、是否当前、能否作为事实、是什么类型”。`.elon/document-sections.json` 保存知识首页、主/副主题、四维治理、有类型的文档关系以及功能/技术节点和实现证据；旧的治理分区只作为快捷投影，它不等于实际移动文件。AI 建议单独保存在 `.elon/document-organization-suggestions.json`，按统一权限模式应用。

Windows 节点提供项目绑定、短期令牌保护的标准 Streamable HTTP MCP。`project_docs_analyze` 复用路径权威性分类器并保持零模型 token；图谱和阅读计划工具限制节点数与 token，正文仍只为歧义或缺失入口按需读取。问题工作流工具支持筛选、分派、期限、原因化忽略/延期和趋势；版本工具同时支持普通 Git 项目与托管知识库的文档差异和新提交式恢复。图谱与 PC 页面由 Rust 后端生成同一份机器契约；文档覆盖和实现证据分别计算，不能由前者推断后者。保存和应用均使用 revision 防并发覆盖，默认由整理前/整理后两个仅文档 Git 提交保护。详细契约按需读取 `docs/project-document-governance-mcp.md`。

持续知识治理由工作区外 SQLite 派生索引、持久变更事件和 60 秒维护轮询组成。质量内核检查链接、孤立文档、owner、复查周期和显式实现引用；问题状态与健康快照同样位于工作区外。服务端把架构、质量、维护、问题工作流和联邦节点合并为目录快照中的 `analysis`，并返回可解释评分组成。大型仓库通过 `.elon/knowledge-federation.json` 的路径与 glob 分层；个人笔记通过 `vaultId` 映射到托管 Git 知识库，每次保存自动提交且可恢复，用户界面不暴露 Git 复杂度。

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
