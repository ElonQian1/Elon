# 架构质量治理路线

本文档记录当前仓库从“能快速迭代”走向“可长期商用维护”的执行路线。目标不是一次性大重构，而是先建立质量门禁，再按风险逐步拆分。

## 当前判断

- 功能覆盖面广，Rust server、PC frontend、Android、Win node、发布脚本和大量测试已经形成完整产品雏形。
- 主要架构债集中在大文件、前端包体、测试环境稳定性和发布运营门禁。
- 商用级标准的关键差距不是“有没有功能”，而是质量线是否可重复、回归是否会被自动拦住、变更是否能小步安全合入。

## 已建立的质量线

- GitHub Actions 执行 Rust server 与 PC frontend 验证。
- CI workflow 已有静态门禁，关键质量步骤被删除或改名会被 `check-ci-quality-gates.ps1` 拦截。
- 本地统一预检入口已建立，`check-local-quality.ps1` 支持 Static/Server/Frontend/All 分层执行，减少人工漏跑门禁。
- source-size guard 防止继续扩大巨型文件。
- Rust warning budget 已收紧为 0，新增 warning 会被 CI 阻断。
- Rust dependency audit 已在 CI 固定安装 `cargo-audit@0.22.2`，并缓存 RustSec advisory-db；漏洞数量进入 Strict 阻断，工具缺失或无法产出 JSON 不再静默 skipped。
- PC frontend 已接入 ESLint 9 flat config，新增 lint 问题会被 CI 阻断。
- Windows sidecar 测试改为串行和 readiness 驱动，降低 ConPTY 并发抖动。

## 分阶段执行

### 第一阶段：稳定质量门禁

- CI 覆盖 `check-source-size.ps1`、`check-release-runbook.ps1`、`check-ci-quality-gates.ps1`、`check-realtime-runbook.ps1`、`check-realtime-ownership.ps1`、`check-realtime-diagnostics-snapshot.ps1`、`check-dependency-audit.ps1`、`check-rust-warning-budget.ps1`、`cargo test`、`npm run lint`、`npm run build`、`npm run check:bundle-budget`、`npm run test:message-flow`、`npm run test:workspace-access` 和 `npm run test:admin-realtime`。
- 所有新功能必须先满足“不要增加 warning、不要扩大红区文件、测试可重复”。
- 对 flaky 测试优先修测试环境和等待条件，不通过放宽断言来掩盖问题。

### 第二阶段：保持零 warning

- 普通构建保持 `check-rust-warning-budget.ps1 -MaxWarnings 0`。
- 测试 target 也保持无 Rust warning，避免 CI 输出重新变成噪音。
- 后续若保留兼容 API 或跨 bin 共享入口，必须用局部 `allow` 表明意图，不能全局关闭 lint。

### 第三阶段：拆分高风险大文件

优先拆分频繁改动、职责过宽、测试影响大的文件：

- `server/src/node_agent_main.rs`
- `server/src/ai_cli/mod.rs`
- `server/src/assets/web_page.html`
- PC frontend 入口和大型页面组件

拆分规则：

- 先抽纯函数和类型，再抽流程编排。
- 保持公共 API 小而稳定。
- 每次只迁移一个职责，迁移后跑对应测试。

进展：

- `server/src/ai_cli/mod.rs` 已开始拆分 PC agent 边界：入口函数迁入 `pc_agent_entrypoints.rs`，生命周期清理 guard 迁入 `pc_lifecycle_guards.rs`，外部调用路径保持不变。
- `run_via_pc_agent` 的轻量聊天 channel-closed 收尾逻辑已迁入 `pc_lightweight_completion.rs`，主循环尾部只保留编排调用。
- `CliDone` 的用量结算与项目执行终态记录已迁入 `pc_cli_done.rs`，保留主循环负责回复分流与 APK 同步。
- `CliDone` 成功回复、APK 同步、最终 Done 消息与 compute run 结算已迁入 `pc_cli_done_success.rs`，`ai_cli/mod.rs` 继续收缩到接近红线。
- `CliChunk` 的轻量聊天增量、Codex passthrough 事件和普通流式回复已迁入 `pc_cli_chunk.rs`，主循环只负责转交事件上下文。
- 轻量 PC chat 的超时收尾已迁入 `pc_lightweight_completion.rs`，与 channel-closed 收尾共用同一职责边界。
- 普通项目模式的 CLI 终态等待超时收尾已迁入 `pc_cli_timeout.rs`，主循环不再直接处理关闭 session、失败记录和计费释放。
- PC CLI 通道中断且未收到 `CliDone` 的最终失败收尾已迁入 `pc_cli_timeout.rs`，与终态等待超时共用失败边界。
- `CliDone` 失败分支的错误文案、compute run 失败结算和计费释放已迁入 `pc_cli_failure.rs`，主循环只保留成功/失败分发。
- PC agent 进度心跳启动逻辑已迁入 `pc_progress_heartbeat.rs`，主流程不再直接创建心跳任务和拼接进度文案。
- 完成拆分后的 Rust server 全量回归，`cargo test --manifest-path server\Cargo.toml` 通过 930 个测试。
- PC CLI 请求准备阶段已迁入 `pc_cli_request_preparation.rs`，集中生成 passthrough/lightweight 标记、原生会话 UUID、prompt、extra args 和有效 Codex effort。
- PC CLI dispatch 接收后的启动上下文已迁入 `pc_cli_startup.rs`，集中处理 cancel guard、billing reservation、display model、compute run、执行开始记录和 started event。
- PC CLI event loop 的事件接收逻辑已迁入 `pc_cli_event_recv.rs`，统一处理 first event、轻量聊天超时、项目终态等待超时和 channel closed 分支。
- `CliDone` 前置结算与成功判定已迁入 `pc_cli_done_flow.rs`，集中处理 passthrough flush、用量结算、执行终态记录、可读输出分析和成功/失败分发判定。
- 完成 `pc_cli_event_recv.rs` 与 `pc_cli_done_flow.rs` 拆分后的 Rust server 全量回归，`cargo test --manifest-path server\Cargo.toml --quiet` 通过 393 + 930 个测试。
- PC CLI 已启动后的事件循环已迁入 `pc_cli_event_loop.rs`，`ai_cli/mod.rs` 收缩为入口、请求准备、dispatch 和 startup 编排层。
- 完成 `pc_cli_event_loop.rs` 拆分后的 Rust server 全量回归，`cargo test --manifest-path server\Cargo.toml --quiet` 通过 393 + 930 个测试。
- `workspace_mode.rs` 的 PC relay 优先委托策略已迁入 `workspace_pc_relay.rs`，主流程只保留“PC 是否已处理完成”的编排判断。
- 项目 workspace 请求的任务分类与 quick casual fast-path 已迁入 `workspace_task_mode.rs`，并补充 3 个分类/fast-path 单元测试。
- 本地 CLI 执行与 Codex native session 恢复/失效重试流程已迁入 `workspace_local_cli.rs`，`workspace_mode.rs` 收缩为 workspace 准备、计费、结果落库和最终响应编排层。
- workspace 本地 CLI 的 native session 选择、tiny chat 跳过、未 bootstrap 聊天会话过滤和恢复提示已迁入 `workspace_native_session.rs`。
- `workspace_local_cli.rs` 增加本地 CLI attempt helper，统一 `run_cli_command_traced` 的 trace 参数拼装，减少重试分支重复。
- workspace 本地 CLI 的 resume-error fresh retry 与 stale-session fresh retry 已迁入 `workspace_local_cli_retry.rs`，`workspace_local_cli.rs` 收缩为 initial attempt 与 retry flow 编排层。
- workspace 本地 CLI 输出后的网络健康兜底、native session 落库、token usage 记录、billing settled、APK 查找和最终 Done 响应已迁入 `workspace_completion.rs`，`workspace_mode.rs` 进一步收缩为入口编排层。
- `workspace_local_cli_retry.rs` 内部的 retry 记录、native session 退役和后台修复调度已集中到 helper，fresh retry timeout option 构造也统一为单点规则。
- 完成 workspace/ai_cli 多轮拆分后的 Rust server 全量回归，`cargo test --manifest-path server\Cargo.toml --quiet` 通过 393 + 933 个测试。
- 修复 `project_space_task_watchdog` 单测中 `Instant::now() - Duration` 在 Windows 上可能下溢的问题，改为零超时断言 pending tool 对 heartbeat-only idle timeout 的阻断语义。
- PC APK relay 输出解析、base64 文本 chunk 兼容、metadata 清洗、安全 APK 文件名和 artifact 名生成已迁入 `ai_cli_apk_relay.rs`，`ai_cli_apk_sync.rs` 收缩为同步调度、artifact 写入和 release 注册流程。
- PC APK 同步 PowerShell 脚本模板已从 Rust raw string 迁入 `pc_apk_sync_template.ps1`，`ai_cli_apk_build_script_impl.rs` 收缩为模板 include、转义函数和脚本装配测试。
- PC 会话页的会话消息缓存、任务消息缓存和加载失效序号已迁入 `useConversationMessageCache.ts`，`ConversationPage.tsx` 开始从巨型页面组件向 hook 化边界收缩。
- PC 会话页的本机节点状态轮询、邀请链接预览/加入和自己的在线状态同步已分别迁入 `useLocalNodeStatus.ts`、`useProjectInviteLink.ts`、`useOwnPresence.ts`，页面继续收缩为组合编排层。
- PC 会话页的本机节点自动绑定副作用和运行路线偏好持久化已迁入 `useLocalNodeProjectBinding.ts` 与 `useProjectRuntimePreferences.ts`，本机执行策略从页面 UI 状态中分离出来。
- PC 会话页的成员面板派生状态、消息流展示派生和成员会话列表加载/新会话切入已迁入 `useConversationMemberPanelState.ts`、`useConversationFeedState.ts`、`useMemberConversationListState.ts`，页面主体继续向组合层收缩。
- PC 会话页的成员操作动作和会话消息打开/缓存刷新逻辑已迁入 `useConversationMemberActions.ts` 与 `useConversationMessageLoader.ts`，页面进一步减少直接 API 操作和缓存细节。
- PC 会话页左侧项目/频道/成员会话导航已迁入 `ConversationChannelSidebar.tsx`，`ConversationPage.tsx` 进一步收缩为中间聊天区与抽屉组合层。
- PC 会话页中间聊天区顶栏、状态提示、landing/feed 和 composer 已迁入 `ConversationChatColumn.tsx`，`ConversationPage.tsx` 收缩到接近纯组合壳，为后续 route/component code splitting 做准备。
- PC frontend 已完成页面级 route lazy loading：`App.tsx` 保留 Shell 同步加载，登录、AI、会话、项目、节点、调试等业务页按需加载；生产构建主入口 JS 从约 924 kB 降到 72.94 kB，会话页独立为 336.89 kB async chunk。
- PC frontend 增加 bundle budget 门禁：`check-pc-frontend-bundle-budget.js` 在构建后检查入口、vendor、store、会话页、最大异步 JS/CSS 和总 JS/CSS 预算，并已接入 PC frontend CI。
- PC frontend bundle budget 已接入手工发布路径：`publish-server.ps1` 上传 `/pc` dist 前会检查预算，`publish-node-agent.ps1` 打包内置 PC 工作台前也会检查预算，避免绕过 CI 发布超预算前端。
- 发布后 smoke 已升级为阻断式检查：`publish-server.ps1` 重启后必须通过 `/health`、`/api/server/version` 且版本号和 git SHA 匹配本次发布；`publish-node-agent.ps1` 广播更新前必须通过 node-agent manifest 与下载端点 smoke。
- 新增 `docs/release-quality-gates.md`，固定 Rust server、PC frontend 和 PC node agent 三条发布路径的本地检查、发布脚本门禁、完成验收和失败处理规则。
- 新增 release runbook 静态门禁：`check-release-runbook.ps1` 校验 runbook 中的脚本引用和 `check-task-complete.ps1 -Kind` 枚举，并已接入 CI。
- 新增 CI workflow 静态门禁：`check-ci-quality-gates.ps1` 校验 GitHub Actions 必须保留 source-size、release runbook、Realtime、dependency audit、warning budget、cargo test、前端 build/budget/smoke 等关键步骤，避免质量线被无意删掉。
- 新增本地统一质量预检：`check-local-quality.ps1 -Scope Static` 串联 source-size、release runbook、CI workflow、Realtime runbook/ownership/diagnostics snapshot 等轻量门禁；`Server`、`Frontend`、`All` 范围可继续扩展到 Rust/PC frontend 重量检查。
- Android APK 发布路径已纳入 release runbook 和静态门禁，`AndroidFeature` 完成验收与 `publish-apk.ps1` 发布脚本引用会被 CI 校验。
- 依赖安全审计进入报告模式：新增 `check-dependency-audit.ps1` 与 `docs/dependency-security-audit.md`，CI 输出 npm audit 与 Rust 依赖审计概况，当前不因既有风险阻断合入。
- Rust 依赖审计已升级为 CI 固定安装 `cargo-audit@0.22.2` 后运行，新增 `install-cargo-audit.ps1`、`check-dependency-audit.ps1 -RequireRustAudit` 与 stale advisory-db fallback，并将 npm/RustSec vulnerabilities 切到 Strict 阻断，避免检查在 CI 中静默 skipped 或把拉库失败误报为 0 漏洞。
- `quick-xml` 已升级至 `0.41.0`，清理 `RUSTSEC-2026-0194` 和 `RUSTSEC-2026-0195`；`rsa@0.9.10` 的无补丁 `RUSTSEC-2023-0071` 仅以精确 advisory/package/version/到期日例外受控放行，例外失效、过期或出现其他 RustSec 漏洞均会阻断 CI。
- Rust web 框架链路升级到 `axum@0.8` / `tower@0.5` / `tower-http@0.6`，WebSocket 文本发送迁移到 `Message::text(...)`，并通过跟踪 `server/Cargo.lock` 将 `spin` 刷新到 `0.9.9`，RustSec warning 基线降为 0。
- 实时通道收敛已建立第一层公共传输基座：新增 `ws_transport.rs`，集中 Axum WebSocket 文本帧构造、JSON 文本帧序列化兜底、严格 JSON 文本帧构造和 `type` 字段解析；`/ws/app`、旧 `/ws/notify`、项目任务 WebSocket、语音 WebSocket 与 HomeCLI agent 发送路径已接入公共层，为后续统一鉴权、错误码、限流、指标和连接生命周期治理打底。
- Axum WebSocket 生命周期控制已开始统一：`ws_transport::receive_data_or_control` 集中处理文本帧、二进制帧、Ping 自动回 Pong、Pong 忽略、Close/读错/断流关闭；旧 `/ws/notify`、`/ws/app`、项目任务 WebSocket 和三条语音 WebSocket 主循环已接入，减少各入口重复维护 ping/pong/close 细节。
- PC relay 客户端侧 WebSocket 已单独建立 `ws_client_transport.rs`，集中 `tokio-tungstenite` 文本帧构造和 JSON 帧发送；`pc_relay_client_impl.rs`、`pc_relay_cli_prompt.rs`、项目工作区 inspect 和 git worktree audit 响应已接入客户端传输层，避免与服务端 Axum 帧类型混用。
- HomeCLI agent 会话生命周期已补上结构化断开原因：reader shutdown、正常关闭、读超时、读错误、writer 关闭分别记录 `close_reason`，并映射到 pending CLI 请求的用户可见失败文案；writer 发送失败会主动触发 session shutdown，避免节点假在线或写半边断开时继续等待。
- 手机 P2P `peer_relay` 生命周期已补齐第一层治理：reader 不再吞掉 WebSocket 读错误，空闲和传输中的 Ping 会明确回 Pong，注册确认和 `SEND_APK` 指令发送失败会清理 registry 并返回结构化原因；APK 传输失败现在区分种子断开、读失败、写失败和 reader 结束，便于后续接入指标和告警。
- 实时通道指标门面已建立：新增 `realtime_metrics.rs`，统一记录 `(channel, close_reason)` 断开计数并输出 `target="realtime_metrics"` 的结构化日志；HomeCLI agent 和手机 P2P `peer_relay` 已接入同一口径，并新增 `/api/admin/realtime/close-metrics` 只读管理接口，为后续 Prometheus/OpenTelemetry 或管理后台可视化留出稳定边界。
- 普通 Axum WebSocket 入口已接入统一断开指标：`ws_transport::WsIncoming::Closed` 现在携带 `WsCloseReason`（peer closed、read error、reader ended、pong/write failed、client control close），`/ws/notify`、`/ws/app`、项目任务 WS、语音转写、实时语音聊天和虚拟麦克风已按各自 channel 记录到 `realtime_metrics`，实时通道断开原因开始形成全项目统一视图。
- 管理后台已新增 Realtime Health 面板：`assets/admin.html` 增加独立 Realtime 标签页，读取 `/api/admin/realtime/close-metrics` 后展示总断开数、channel 汇总和 close reason 明细；管理者可直接判断断开主要来自客户端主动关闭、网络断流、读错误还是服务端写失败。
- Realtime Health 已升级为持久化窗口统计：新增 SQLite 表 `realtime_close_events` 与迁移 v96，每次 WebSocket close 事件会同时写入内存计数和 30 天保留的事件表；管理 API 返回 `last_1h`、`last_24h`、`all_time`、`process` 四个窗口，后台页面可切换查看，支持跨进程重启后的趋势判断。
- Realtime Health 已接入阈值告警：最近 1 小时读错误、写失败、读超时分别按 `realtime:*` fingerprint 写入现有告警表，并在 Realtime 面板独立展示；billing 告警列表过滤 `billing:*`，避免计费风险与实时通道风险互相污染。
- Realtime 告警阈值已纳入管理后台配置：`/api/admin/billing/config` 暴露并允许保存 `realtime_close_read_error_alert_threshold_1h`、`realtime_close_write_failure_alert_threshold_1h`、`realtime_close_timeout_alert_threshold_1h`，后台配置卡和配置弹窗可查看/修改最近 1 小时读错、写错、超时阈值。
- Realtime 告警阈值策略已补回归测试：覆盖默认阈值不误报、降低阈值后生成告警、升高阈值后自动 resolved，确保后台配置真正影响告警计算而不只是 UI 展示。
- Realtime 管理 API payload 已补契约测试：`/api/admin/realtime/close-metrics` 的 `metrics`、`alerts`、`windows.all_time`、`windows.last_1h`、`windows.last_24h`、`windows.process` 结构和窗口隔离语义被单测锁住，降低后台可视化字段被误删或混淆的风险。
- 新增 `docs/realtime-operations-runbook.md`，固定 Realtime Health 日常巡检、close reason 语义、阈值配置、读错/写错/超时/P2P relay 处置流程和发布前回归命令，让实时链路告警具备可执行的运营手册。
- 新增 `check-realtime-runbook.ps1` 并接入 CI，静态校验 Realtime runbook 必须保留管理接口、窗口字段、告警阈值、channel、close reason、边界模块和回归命令，避免运维手册随代码演进漂移。
- 管理后台 Realtime 面板已补前端 smoke：`test-admin-realtime-health.js` 使用最小 fake DOM 执行 `admin.html` 内联脚本，验证 `/api/admin/realtime/close-metrics` 请求、窗口选择、summary/detail/alerts 渲染和 alert detail 转义，并通过 `npm run test:admin-realtime` 接入 CI。
- 新增 `docs/realtime-channel-ownership.md` 和 `check-realtime-ownership.ps1`，固定每个 Realtime channel 的业务边界、入口模块、close reason 来源、指标写入点和变更规则，并在 CI 中校验 owner 表与源码标签不漂移。
- Realtime 诊断字典已机器化：新增 `/api/admin/realtime/diagnostics`，由 `realtime_diagnostics_catalog()` 统一导出 channel、close reason、alert bucket、入口模块、同步目标和变更规则；ownership/runbook guard 已校验该 API 与文档不漂移。
- 管理后台 Realtime 面板已消费诊断字典：加载 `/api/admin/realtime/close-metrics` 时同步读取 `/api/admin/realtime/diagnostics`，在 close reason detail 展示 category、alert bucket 和 first check，并在告警卡片展示 bucket 级首查建议；前端 smoke 已覆盖该链路。
- Realtime 后端告警 detail 已消费诊断字典：`refresh_realtime_close_alerts()` 按 `alert_bucket` 从 `realtime_diagnostics_catalog()` 派生 `first_check` 并写入告警 detail，读错/写失败/超时三类告警均有回归测试锁住。
- Realtime 告警计数分类已消费诊断字典：读错、写失败、超时计数不再维护手写 reason 列表，而是按 `alert_bucket` 从 `realtime_diagnostics_catalog()` 派生；ownership guard 禁止旧分类函数回流，回归测试覆盖跨来源 reason 聚合。
- Realtime 诊断字典已补静态 JSON 快照：`realtime_diagnostics_catalog_matches_snapshot` 对比 `server/src/realtime_diagnostics_catalog.snapshot.json`，锁住 diagnostics API 的字段名、数组结构、alert bucket 和 first check；ownership guard 会校验快照存在且覆盖核心字段。
- Realtime diagnostics snapshot 已纳入独立 CI 门禁：`check-realtime-diagnostics-snapshot.ps1` 会校验 JSON 快照结构、源码 `include_str!` 绑定和目标 snapshot 单测，让 `/api/admin/realtime/diagnostics` 契约漂移在发布前被明确拦截。
- PC frontend 已升级到 `vite@8.1.4` 与 `@vitejs/plugin-react@6.0.3`，清零 npm audit 基线；Vite/Rolldown 的 `manualChunks` 配置已迁移为函数形式，Node 运行要求固定为 `^20.19.0 || >=22.12.0`。
- PC frontend 已从旧 `.eslintrc.cjs` 迁移到 ESLint 9 `eslint.config.js`，lint 脚本改为显式扫描 `src/**/*.{ts,tsx}`，并已接入 CI；TypeScript 未定义符号检查继续由 `tsc` 负责，React Hooks 暂保持原有经典规则强度。

### 第四阶段：前端性能与发布门禁

- 对 PC frontend 做 route/component 级 code splitting。
- 为 Vite chunk size 设置明确预算，并在 CI 中阻断首包或大页面 chunk 回退。
- 发布脚本增加 bundle budget、阻断式 smoke test 和版本健康检查。
- 发布路径形成轻量 runbook，避免不同人或不同 AI 代理绕过同一套发布质量线。

### 第五阶段：企业级运营标准

- 依赖安全审计纳入 CI 或定期报告。
- 关键业务路径补端到端 smoke。
- 发布、回滚、数据迁移、配置变更形成 runbook。
- 对核心模块建立 owner/边界文档，减少跨层耦合。

## 长期原则

- 小步重构，先建门禁再降债。
- 业务代码和格式化、warning 清理分开提交。
- 对用户可见文案、状态机、数据排序这类产品语义，用测试锁住。
- 大文件不继续长大，新功能优先进入独立模块。
