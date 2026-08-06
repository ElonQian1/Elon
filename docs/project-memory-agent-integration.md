# 项目记忆与桌面 AI 代理接入

最后更新：2026-08-06

本文只回答 Codex Desktop/CLI、直接安装的 Codex 和其他 MCP 代理如何复用项目文档记忆。完整文档治理接口见 `docs/project-document-governance-mcp.md`。

## 目标与非目标

项目记忆不是第二套代码搜索。代理仍用自己的 `rg`、Git、符号索引和文件读取能力核对当前实现；本功能只提前给出少量高置信导航摘要和证据路径，减少第一次宽搜与重复读取。

固定信源顺序是：

1. 当前源码与测试；
2. 约束性项目规则和 current ADR；
3. Git 中已审核、证据仍有效的导航记忆；
4. 本机待审核候选。

低优先级来源与高优先级来源冲突时，低优先级来源失效，不做自动合并。摘要不保存源码正文、聊天、prompt、transcript、命令或工具输出。

## 可移植记忆

跨 PC 真源是 `.elon/document-sections.json.context_memories`，随 Git 同步。每条记忆包括：

- 最多 800 字符的导航摘要和最多 8 个 topic；
- 1–8 个工作区相对证据路径、定位符、SHA-256，以及可用时的 Git 对象身份；
- owner；
- repository 或 paths scope，并可进一步限制 federation `scope_id`、Git branch、release/channel 与 clean/dirty worktree；
- 审核回执、审核日期、审核者、复核周期和可选到期日。

新 PC 不需要复制旧 PC 的 SQLite 候选。SQLite 只是本机审核收件箱；只有 suggestions/revision/authorization/apply/Git 链路通过后的共享记忆才可移植。

旧共享记忆缺少生命周期字段时继续可读，不自动改写；Memory CI 会提示补齐。到期、漂移或与另一条共享事实存在潜在冲突的记忆不进入 `verified_project_memory`。

共享 manifest 最多保存 256 条记忆。轻量规划先按任务路径、联邦分片、当前分支、release 和工作区状态零 I/O 排除不适用项，再只对相关性最高的 24 条做证据验证，最终最多注入 3 条。调用方没有提供某个限定值时，显式依赖该限定值的记忆失败关闭，不会作为“可能相关”混入上下文。

## Memory CI

完整 governance MCP 提供 `project_docs_check_native_context_memory`；PC 使用同源接口：

```http
POST /api/project-docs/native-context/health
Content-Type: application/json

{
  "project_root": "D:\\repo",
  "offset": 0,
  "limit": 50,
  "failure_policy": "advisory",
  "include_capabilities": false
}
```

失败策略：

- `advisory`：任何问题只返回 `warn`，建议退出码仍为 0；
- `fail_on_drift`：证据漂移或记忆过期返回建议退出码 1；
- `strict`：复核逾期、治理缺口和共享事实潜在冲突也返回建议退出码 1。

响应包含全量计数、当前页、next offset/cursor、逐项状态和 repair plan。repair plan 永远 `automatic=false`：重定位只给候选路径，冲突只要求按当前源码和约束文档人工裁决，更新仍走审核链路。PC 仅在“一条漂移路径恰好对应一个 Git 对象验证过的新路径”时显示显式确认按钮；确认后也只生成 pending 候选，仍需审核与 apply。

仓库提供可执行包装器，由它读取服务端建议退出码并结束 CI 进程：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\project-memory-ci.ps1 `
  -ProjectRoot D:\repo -FailurePolicy strict
```

能力矩阵默认省略，只有需要代理/Hook/app-server/Memories 接入合同时传 `include_capabilities=true`，避免分页 CI 重复消耗响应 token。

## 代理接入矩阵

| 入口 | 当前能力 | 不自动做的事 |
|---|---|---|
| 节点托管的普通 Codex 宽任务 | 自动注入只读 context、按需 feature、只写 receipt 和会话级 Hook 配置 | 不注入完整治理 schema，不绕过 Hook 信任 |
| 直接运行的 Codex Desktop/CLI | 可手动 bootstrap 描述符，或安装仓库内 `plugins/yilong-project-memory` 接入 context/feature/receipt | 仓库只提供插件包，不替用户安装、不自动信任 Hook |
| 其他支持 HTTP MCP 的代理 | 可使用 vendor-neutral governance 描述符；context/feature/receipt 可由适配器手动接入 | 无可靠工具过滤时不自动注入普通任务；Hook 需供应商适配 |
| Codex plugin bundle | 已从仓库 marketplace 安装到真实 Codex cache；app-server 发现三个 server，各只发布一个工具，并真实调用 feature list；删除、重装和新任务重载均已验证 | Hook 仍需用户在 `/hooks` 单独信任；安装成功不等于 Hook 已加载，也不等于模型已完成公平 A/B |

context、feature、receipt、governance profile 与短期 session/token 固定；修改 URL 查询参数不能提升权限。普通任务只使用三个各含一个工具的极小 profile，因此不会每次积压完整治理工具 schema 或项目全文；feature 的详细 action schema 只有显式 `describe` 才返回。`project_context_plan` 可传 `task_paths`、`scope_id` 和 `release`；服务端另从当前 Git 工作区绑定 branch 与 clean/dirty 状态，这些范围也进入缓存键和 plan receipt。若项目存在 `.elon/project-features.json`，同一只读工具会先排序、只校验前 12 个候选，再返回最多 3 个 query/task path 相关且需求 hash 有效的活动功能，不复制需求正文；详细状态机见 `docs/project-feature-registry.md`。

## Codex 桌面端安装与就绪门禁

仓库在 `.agents/plugins/marketplace.json` 发布 `yilong-project-memory` 本地插件条目，源路径固定为 `./plugins/yilong-project-memory`。Codex 桌面端重启后可读取 repo marketplace；在 Plugins 中选择 `Yilong Project`、安装并启用插件，再新建任务让新的 MCP 工具进入会话。不要把“marketplace 可发现”误报成“插件已安装”，也不要把“插件已启用”误报成“Hook 已信任”。

只读检查入口：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-project-memory-codex-readiness.ps1
```

默认只要仓库内 marketplace、manifest、三个 MCP profile、三个有界 Hook 事件和 Node.js 18+ 合同正确就退出 0，并另外报告本机安装与运行状态。需要把未安装或未启用视为失败时传 `-RequireInstalled`；还要验证一龙节点 loopback `/api/health` 时传 `-RequireRuntime`。输出明确分开：

- `static_ready`：仓库插件源可以交付；
- `installed_ready`：当前项目已信任、Codex cache 中存在与仓库逐文件一致的安装副本、插件配置已启用；
- `runtime_ready`：在 installed 基础上还能发现本机节点；
- `hook_trust_verified=false`：脚本永远不读取或伪造 Codex Hook 信任；必须在新任务的 `/hooks` 中人工确认；
- `end_to_end_verified=false`：就绪检查不冒充真实任务验收。

Codex 安装的是 cache 副本，不会直接执行 marketplace 源目录。更新插件内容后必须递增插件版本、先删除旧插件、再从 marketplace 安装并新建任务；只修改仓库 marketplace、插件外文档或仓库外测试脚本时不需要为了“刷新”而随意递增插件版本。不要只改 SemVer 的 build metadata 后直接重复 `add`：build metadata 不改变版本优先级，旧 cache 可能继续被选择。就绪门禁会逐文件比较当前仓库源和 cache，发现陈旧副本时失败关闭。

## Hook 生命周期

当前会话只配置 `PostToolUse`、`Stop`、`SessionEnd`。`SessionStart`、`PreCompact`、`PostCompact`、`SubagentStart`、`SubagentStop` 在执行程序中仅作为有界 no-op 适配缝，真实运行验收前不配置。

节点会话与仓库插件包都只配置这三个事件。插件 Hook 的账本位于 `PLUGIN_DATA`（没有时退回系统临时目录），只保留最多 48 个规范相对路径及 read/write 类型；SessionEnd 删除，异常残留 24 小时后清理。直接生命周期集成测试已经覆盖 PostToolUse→Stop→SessionEnd，但 Hook 配置不等于真实 Codex 已执行：Codex 首次或定义变化后仍需用户在 `/hooks` 审核，项目不写信任记录。一次性测试进程可以显式使用 Codex 的 trust bypass 参数验证其他链路，但不得持久化或把它作为产品默认值。

## 真实 token 与时间观测

静态 capability manifest 定义 `elon.project_context_runtime_observation.v1`，只接受 Codex app-server 的以下通知类型：

- `hook/started`、`hook/completed`；
- `thread/tokenUsage/updated`；
- `turn/started`、`turn/completed`；
- `item/started`、`item/completed`，只保留 item 类型和工具名以聚合原生文件读取次数。

`scripts/project-memory-app-server-observer.mjs` 已提供 stdin 事件接入器：它在进入 loopback API 前先缩减为方法名、token 累计计数或 item 类型/工具名；服务端再次白名单校验，并只在工作区外 SQLite 保存不可逆 session 指纹、baseline/enabled 窗口、事件计数、input/cached-input/output token 计数、返回元数据字节、选择记忆数和耗时。原始事件对象处理后立即丢弃；prompt、聊天、transcript、assistant message、tool input/output、源码正文和命令文本不落库。24 小时未完成的窗口标为 abandoned；非活动窗口只保留最近 2000 条，避免观测索引无限增长。

当前已取得结构性预算数据，但尚未取得可对外宣称的同任务模型 A/B。102 条功能注册表全文为 146930 字节（约 36733 token），feature MCP 的有界列表响应为 1431 字节（约 358 token），减少 99.03%；针对同一条功能做理想的原生定向解析为 1483 字节（约 371 token），说明主要收益来自避免第一次全量扫描和稳定提供语义状态/漂移判断，而不是替代一次已经完美命中的原生读取。三个工具 schema 合计 4747 字节（约 1187 token）；`codex debug prompt-input` 启用/禁用插件均为 31634 字节，未观察到插件把项目正文或 schema 常驻复制进模型 prompt。以上 token 均按 UTF-8 字节本地估算，不是供应商账单或完整任务 token。

只有同一个 `benchmark_key` 同时取得 `baseline_without_project_memory` 与 `with_project_memory` 两个完整窗口后，接口才返回输入 token、耗时和原生文件读取次数的差值；否则只报告无测量或部分测量，不能宣称真实任务节省。真实 `codex exec` 模型已经发现并正确选择单一 feature 工具，但非交互任务中的 MCP 审批被取消，另一次绕过审批的网络任务未在时限内完成，因此这些 turn token 不构成公平 A/B，也不写入效果结论。

为了避免人为复用 `benchmark_key` 把不同代码、模型、Codex 版本或任务误配在一起，先生成不含任务正文的 A/B manifest：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\new-project-memory-benchmark-plan.ps1 `
  -CaseId project-orientation `
  -ModelId gpt-5.6 `
  -TaskFile .ai-tmp\project-memory-task.txt `
  -CodexBuild 26.727.6591.0
```

脚本只在 tracked worktree clean 时生成 manifest。`benchmark_key` 由 case ID、模型、任务文件 SHA-256、Git HEAD 和 Codex build 共同派生；manifest 保存任务哈希，不保存任务正文。baseline 和 enabled 两次观察都必须向 observer 传同一个 `--benchmark-manifest`、`--task-file`、`--model-id` 与 `--codex-build`。observer 会重新计算所有哈希、核对当前 Git HEAD/clean 状态，并只在通过后输出 `benchmark_protocol_verified=true`；可先用 `--validate-manifest-only` 离线校验，不连接节点。

直接手写 `benchmark_key` 的旧入口继续兼容合成和诊断，但不能作为严格 A/B 结论。对外报告节省时必须同时保留成对窗口、observer 的协议验证标记以及限定 case，不把单个任务的负 delta 外推为普遍比例。

## 审核质量反馈

拒绝候选时必须从 duplicate、task_local、unsupported、conflict、stale、not_reusable 中选择原因；原因与人工决定时间只保存在本机候选索引。候选列表同时聚合最近最多 200 条的 producer 状态计数，帮助发现某种回执来源是否经常生成重复或任务局部内容。该统计只描述本机审核结果，不自动封禁 producer，也不能改变信源优先级。

## Codex 官方 Memories 边界

Codex 官方 Memories 是供应商自己的本机生成状态，与项目 Git 记忆分离。本项目：

- 不读取、导入、写入、删除或备份 Codex 私有 Memories；
- 不假设存在稳定的通用第三方 memory import/export 接口；
- 团队规则继续放在 `AGENTS.md` 和受控项目文档；
- 只把代理显式提交、证据绑定且经过人工审核的短导航事实写入 Git 共享记忆。

因此“换 PC 后免除全部理解成本”不是承诺。可实现的是：新代理先获得小而可验证的导航层，再按任务只打开少量当前文件；这既保留供应商原生搜索优势，也避免把旧私有状态复制成冲突信源。

## 2026-08-05 隔离运行验收

本轮在独立 worktree 和独立 `127.0.0.1:7817` 节点完成真实编译与传输冒烟，没有打开 PC 项目文档页面，也没有停止现有节点：

- `elon-pc-node` 调试二进制构建通过，`project_document` Rust 回归与 context/receipt profile 固定隔离测试通过；仓库图谱中唯一一条跨产品功能图与技术架构图的重复关系已删除，9 组版本化检索案例重新全部通过。
- context/receipt stdio 代理从进程启动、loopback bootstrap 到 initialize/tools/list 分别耗时 171ms 和 166ms；每个 profile 只返回一个工具。尝试把 context 会话 URL 改成 receipt 返回 HTTP 401。
- context 首次宽任务计划耗时 3595ms，MCP tool result 为 4652 字节；同一短期会话、同一 revision 的重复调用耗时 1196ms、2839 字节，返回 `status=not_modified`、相同 `plan_id` 和自动会话复用回执。响应合同固定 `source_bodies_returned=0`；这里的 token 数仍是本地 UTF-8 结构估算，不是供应商账单或完整任务 token。
- `project-memory-ci.ps1` 通过 loopback 接口取得 `pass` 与退出码 0，返回 0 条当前记忆且不返回正文；这证明空库和节点发现链路可用，不证明已有共享记忆质量。
- Hook 执行程序的 PostToolUse→Stop→SessionEnd 独立冒烟通过：只回传两个相对证据路径，未泄漏测试 session 文本，SessionEnd 后账本为 0。它没有经过真实 Codex `/hooks` 信任流程，因此不等于 Codex 已加载 Hook。
- app-server 观察器用白名单合成事件验证了 baseline/enabled 配对、token/文件读取计数聚合和 `raw_event_payloads_stored=false`。合成窗口只验证接入器与比较合同，不能用其中的负差值宣称真实节省。
- repo marketplace、空 Codex home 与精确 cache 安装 fixture 已通过离线合同检查；哈希绑定 A/B manifest 已在临时 clean Git 仓库通过 plan→observer 双向复算。它们证明“可发现条件”和“配对条件”可机器检查，不证明当前桌面会话已经安装插件。

因此目前可以确认“代理无需主动打开页面即可使用最小 MCP、CI 与 Hook 执行程序”；仍不能确认真实 Codex 安装后的端到端任务节省比例。

## 2026-08-06 真实 Codex、浏览器与可移植性验收

本轮继续在隔离节点和真实 Codex app-server 上完成端到端验收：

- 真实 Codex 从 repo marketplace 安装插件后，三个 MCP server 全部进入 ready，每个只暴露一个工具；直接调用 `project_feature_workflow/list` 成功。典型冷启动约 4.3–8.6 秒，工具调用约 0.54–0.87 秒，响应 1431–1444 字节。
- 真实安装发现并修复了三个可移植性缺口：MCP 进程显式以插件根为 `cwd`、只转发 `ELON_NODE_ADMIN_URL`/`ELON_PROJECT_ROOT`、插件默认 prompt 保持在 Codex manifest 限制内。测试脚本现在会同时门禁这些合同。
- 同一临时 Git 提交的普通 clone 与附加 worktree 都能启动真实 app-server 并调用功能工具；修改 clone 中需求文件后，漂移检查返回 `requirement_drifted`、`requirement_current=false` 和 `automatic=false` 修复步骤。它证明同机可移植 Git 身份与失败关闭，不冒充不同操作系统或远端团队网络验收。
- 功能注册表 Rust 测试覆盖 proposed→released、需求漂移/重绑、依赖阻塞、revision 冲突、租约过期重认领、历史分页、证据刷新和并发写者只能一个成功。Rust 检查、节点构建及 PC build/lint/文档合约测试通过。
- PC 项目文档页面在真实浏览器中覆盖 101 条功能、两页分页、搜索/空态、AI 指令、打开需求、离线降级/恢复和刷新保持；控制台错误为 0。MCP 使用全程不要求主动打开该页面。
- Hook 执行程序的真实进程生命周期集成通过；真实 Codex 的 `/hooks` 用户信任仍保留为人工安全边界，不由项目伪造。

## 仍需真实验收的项目

以下项目仍需要生产环境或人工安全确认，不能由本轮自动化替代：

- 真实 Codex `/hooks` 中由用户审核并信任，然后观察 Codex 自身触发 PostToolUse/Stop/SessionEnd；compact/subagent 事件仍未配置，只保留 no-op 适配缝；
- 官方 app-server 事件流的真实任务 A/B，使用同一 benchmark 测得 input token、原生文件读取和总耗时，而不是合成事件；
- 不同 PC/操作系统、真实远端 push/pull、换行策略和多人长期并发下的迁移验证；
- 生产规模注册表、长期运行节点和故障恢复下的容量、延迟与资源曲线。
