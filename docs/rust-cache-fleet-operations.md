---
title: "Rust 缓存平台跨 PC 运维与子项目接入"
owner: platform
reviewed_at: 2026-08-16
review_interval_days: 60
role: runbook
lifecycle: active
authority: authoritative
default_retrieval: false
version_status: current
implementation_refs:
  - file:scripts/rust-cache.ps1
  - file:scripts/rust-cache/RustCache.Fleet.psm1
  - file:scripts/rust-cache/RustCache.FleetQueue.psm1
  - file:scripts/rust-cache/RustCache.Install.psm1
  - file:scripts/rust-cache/RustCache.Launcher.psm1
  - file:scripts/rust-cache/RustCache.GcApproval.psm1
  - file:server/src/node_api/rust_cache_fleet/mod.rs
  - file:server/src/node_api/rust_cache_fleet/contract.rs
  - file:server/src/node_api/rust_cache_fleet/gc.rs
  - file:server/src/node_api/rust_cache_fleet/gc_contract.rs
  - file:server/src/node_agent_rust_cache_fleet.rs
  - file:server/src/node_agent_rust_cache_fleet/gc.rs
  - file:server/src/node_agent_rust_cache_fleet/model.rs
  - file:server/src/node_agent_rust_cache_fleet/storage.rs
  - file:server/src/store/rust_cache_fleet_reports.rs
  - file:server/src/store/rust_cache_gc_requests.rs
  - file:pc-frontend/src/features/node/NodeCacheHealthCard.tsx
  - file:pc-frontend/src/features/node/NodeCacheGcApproval.tsx
  - file:pc-frontend/src/features/node/NodeCacheFleetOverview.tsx
  - file:pc-frontend/src/features/node/nodeCacheFleet.ts
  - file:.agents/skills/manage-shared-build-cache/SKILL.md
  - file:scripts/test-rust-cache-portability.ps1
---

# Rust 缓存平台跨 PC 运维与子项目接入

本文定义一龙项目及其子项目在多台 Windows PC 上共用同一套缓存治理能力的业务流。缓存文件不跨电脑直接共享；跨电脑共享的是版本化工具、项目身份、兼容域策略、健康报告格式和安全操作流程。

## 分层架构

| 层级 | 保存内容 | 分发方式 | 权限边界 |
|---|---|---|---|
| 权威源码层 | `rust-cache.ps1`、模块、测试、Skill | Git 仓库 | 只有审查后的提交可作为安装源 |
| PC 安装层 | 当前平台快照、策略、锁、报告、SCCache | 每台 PC 本地安装 | 安装锁保证单机升级串行 |
| 稳定启动层 | `%LOCALAPPDATA%\Elon\bin\rust-cache.ps1` | 安装器生成 | 只转发参数，不启动第二个可见 Shell |
| 项目合同层 | `rust-cache.project.json` | 跟随子项目 Git | 只保存稳定项目 ID 和兼容域，不保存机器路径 |
| 构建执行层 | SCCache、workspace/shared build-dir、target | 目标 PC 本地 | Cargo 调用持有分区锁，发布产物仍由项目拥有 |
| Fleet 观测层 | 脱敏报告与不可变 outbox 信封 | 节点按需生成或排队 | 报告只读，不能成为删除授权 |
| GC 审批层 | 请求状态、脱敏计划摘要、精确摘要审批和回执 | 服务端与节点轮询 | 不接收路径或命令，删除只在目标 PC 执行 |

SCCache 负责兼容编译对象的跨项目复用。命名 Cargo build-dir 只允许在同一 `project_id`、rustc 代际和兼容域内跨 worktree 复用。不同 PC 不应通过网络盘共同写一个 Cargo target 或 build-dir。

## 一台新 PC 的安装流程

在当前可信的一龙仓库检出中执行：

```powershell
& .\scripts\rust-cache.ps1 install -ProjectRoot . -Apply -InstallCodexSkill
& .\scripts\rust-cache.ps1 doctor -ProjectRoot .
```

安装器会分别写入规范化源码指纹与安装文件原始字节指纹，并安装固定用户启动器和 Codex Skill。源码指纹会统一 UTF-8 BOM 与 LF/CRLF，使同一 Git 内容在不同 Windows 配置和 worktree 中保持同一版本身份；安装指纹不做规范化，安装后的任何字节变化仍会被 `doctor` 识别。`doctor` 报告 `platform-version`、`platform-integrity` 或 Skill 完整性异常时，必须从当前可信提交重新安装，不能手工复制单个模块。

调用入口必须在当前 PowerShell 会话内使用 `&`。缓存工具、用户启动器和 Skill 均不得通过 `Start-Process powershell.exe` 或 `Start-Process pwsh.exe` 打开可见窗口。节点后台服务若必须创建独立进程，应由节点宿主使用隐藏窗口和受控日志，而不是由项目包装器自行弹窗。

## 一个子项目的接入流程

没有平台源码的子项目使用固定用户启动器：

```powershell
$cache = "$env:LOCALAPPDATA\Elon\bin\rust-cache.ps1"
& $cache init-project -ProjectRoot D:\work\shop-app -ProjectId shop-app `
  -AllowedDomain dev-windows-msvc,agent-validation
# 审查 JSON 预览后再执行：
& $cache init-project -ProjectRoot D:\work\shop-app -ProjectId shop-app `
  -AllowedDomain dev-windows-msvc,agent-validation -Apply
& $cache doctor -ProjectRoot D:\work\shop-app
```

将生成的 `rust-cache.project.json` 提交到子项目。项目可以保留自己的 `cargo-dev.ps1`、验证和发布包装器，但它们只能选择稳定 domain/partition 并把参数转给用户启动器，不能复制锁、GC、注册表或路径算法。

首次只启用 workspace 隔离和 SCCache。确认两个 worktree 的依赖、feature、target 和并发模式兼容后，才为受控验证入口声明命名共享分区。不要把任务 ID、会话 ID、Git SHA、功能名或 PID 作为 domain/partition。

## Fleet 健康报告

每台节点使用平台分配的稳定 `node_id` 生成报告：

```powershell
& "$env:LOCALAPPDATA\Elon\bin\rust-cache.ps1" fleet-report `
  -ProjectRoot D:\work\shop-app -NodeId <platform-node-id> -IncludeSizes
```

默认报告写入该 PC 缓存根的 `reports\fleet`。也可用绝对 `-OutputPath` 写入节点待上传目录。报告包含：

- 平台源码指纹和健康状态；
- 项目 ID、允许域和共享分区数量；
- 分区、锁、quarantine、legacy 和活动 Cargo/rustc 数量；
- 磁盘总量、剩余量及是否建议审查 GC；
- 按 scope/domain 聚合的数量和可选字节数。

报告不包含项目根、缓存根、用户名、电脑名、启动器路径或分区绝对路径。`node_id` 必须由一龙节点身份层显式传入，缓存工具不把电脑名当身份。当前仓库已实现本地报告导出、节点自动上传、服务端有界存储、PC 节点详情健康卡片、本人多节点只读总览，以及与报告分离的精确摘要 GC 审批代码。生产 TLS、发布后的节点升级和真实多 PC 验收完成前，不能宣称公网远程回收已经上线。

网络不稳定或需要节点服务稍后上传时，使用 outbox 信封：

```powershell
& "$env:LOCALAPPDATA\Elon\bin\rust-cache.ps1" fleet-stage `
  -ProjectRoot D:\work\shop-app -NodeId <platform-node-id> -IncludeSizes
```

默认信封写入缓存根的 `reports\fleet\outbox`，包含紧凑脱敏报告、报告字节长度、SHA-256、稳定节点 ID，以及“接收端必须校验节点归属、不得据此执行破坏操作”的机器可读约束。信封生成后保持不可变；上传尝试、失败原因和服务端 ACK 应写独立回执，避免重试修改原始证据。

服务端现已提供：

- `POST /api/node/cache-reports/{node_id}`：PC 节点使用既有节点凭证自动提交完整信封；
- `POST /api/me/nodes/{node_id}/cache-reports`：由已登录节点所有者提交完整信封；
- `GET /api/me/nodes/{node_id}/cache-reports/latest`：由节点所有者读取最新已接受报告；
- ACK schema `elon.rust_cache.fleet_ack.v1`：返回信封 ID、报告哈希、接收时间和幂等状态；
- 每节点最多保存 100 条历史，同一 `node_id + report_sha256` 重试不会重复写入。

接收端严格校验节点凭证，以及路由、信封和内嵌报告中的节点 ID，重新计算 UTF-8 JSON 字节长度与 SHA-256，拒绝未知字段、绝对路径、本机身份、破坏性执行记录或破坏性授权。响应始终声明 `destructive_actions_authorized: false`。

PC 节点获得有效凭证后会周期性扫描既有 outbox，每轮最多处理 4 个信封。上传目标只允许 HTTPS 或本机回环 HTTP；只有服务端 ACK 的节点 ID、信封 ID 和报告哈希全部匹配时，节点才先写入不可变 `receipts\<envelope-id>.ack.json`，再把原信封原子移动至 `accepted`。网络或服务端失败只写 `attempts` 状态并保留原信封，后续继续重试。报告 ACK 永远不会触发 GC。节点另行轮询受限 GC 请求，只能选择已安装工具的 `gc-plan` 或 `gc-apply-approved` 两个固定动作，不能接收任意命令或路径。

公网节点必须配置唯一安全真源 `NODE_ENDPOINT_HTTPS_ORIGIN`。当前仍只使用 `http://43.139.149.158:8080` 的旧节点会在本地 `attempts` 写入 `secure-upload-origin-required`，并原样保留 outbox；不得为了上传遥测而把节点长期凭据降级发送到明文 HTTP。TLS 端点未配置时，中央缓存总览显示“暂无报告”是正确状态，不代表远程 GC 可用。

PC 工作台的“我的节点”详情会读取所有者接口，区分尚未上报、读取失败、健康、报告陈旧、建议检查和报告异常。健康摘要再次校验响应 schema、节点 ID 和 `destructive_actions_authorized=false`。独立的“安全回收”区只能请求目标电脑生成新预演、显示脱敏摘要、批准准确的计划 ID 与摘要，或在执行前撤销；它不显示和不接受路径。

“缓存总览”以最多 4 个并发请求读取当前账号名下的全部节点，单台节点的 404 或读取失败不会阻断其他节点。总览复用单节点响应校验和 24 小时陈旧规则，按读取失败、异常、需关注、未上报、健康的顺序展示，并可进入对应节点详情。它不新增聚合数据库、跨所有者查询或写操作；节点数增长到需要服务端分页聚合时，应新增版本化只读端点，而不能放宽现有节点归属校验。

## GC 业务流

跨 PC 回收使用独立状态机，不能把最近一次健康报告直接升级为删除计划：

```text
requested（所有者请求，不含路径）
  -> plan_ready（目标 PC 保存完整不可变计划，只上传脱敏摘要和 SHA-256）
  -> approved（所有者批准准确的 plan_id + plan_digest）
  -> executing（目标 PC 重新扫描动作、大小、活动写入者并取得本机锁）
  -> completed / partial / failed（上传脱敏回执）
```

同一节点同时只允许一个活动请求。计划最多有效 24 小时；本地工具单次最多运行 6 小时，执行状态超过 7 小时没有回执会失败关闭为 `execution-timeout`，释放节点重新生成计划。迟到回执不能覆盖该终态。路由节点、所有者、节点凭证、请求 ID、计划 ID 和摘要必须全部匹配。动作集合、目录大小或活动 Cargo/rustc 数量变化时整批拒绝，审批不能自动迁移到新计划；取锁后新出现的锁只会保留对应分区并生成 `partial` 回执。服务端不能通过远程文件共享删除目录，也不保存绝对路径。

服务端每节点只保留最近 100 条终态 GC 请求。节点仅在服务端确认后，把本地不可变计划、GC 回执、已接收 Fleet 信封和 ACK 各保留最近 100 份；未上传 outbox 和当前失败尝试不因历史上限被删除。

远程审批只覆盖受管分区的机器级普通策略 GC 和显式老化策略。依赖具体项目清单或工作区路径的 `-SharedAliasesOnly`、`-WorkspaceOnly`、`-RecoverMissingWorkspaces`，以及 legacy purge、缓存迁移、Cargo 父配置修改和任意脚本执行，仍必须在目标 PC 本机完成。

常用命令：

```powershell
# 本机状态，计算真实大小
& $cache status -ProjectRoot D:\work\shop-app -IncludeSizes

# 默认只预演
& $cache gc -ProjectRoot D:\work\shop-app -WorkspaceOnly

# 审查报告后，在同一目标 PC 应用
& $cache gc -ProjectRoot D:\work\shop-app -WorkspaceOnly -Apply
```

`-ForceAged` 只用于明确回收已过 TTL 的分区，不能用于绕过低磁盘告警。外部旧缓存必须先 `register-legacy -Retired`，再通过 `purge-legacy` 预演和执行。活动、带锁、未知、外部、脏或未合并 worktree 的数据始终保留。

## 自动触发边界

自动化只适合以下低风险动作：

- 长构建前运行 `doctor` 或轻量 preflight GC；
- 任务 worktree 成功收尾时精确回收该任务拥有的 workspace 分区；
- 低磁盘时自动生成 GC 预演报告；
- 节点空闲时生成并排队脱敏 fleet report；已有节点 uploader 会安全消费 outbox。

节点服务消费 `fleet-stage` outbox，并独立轮询受限 GC 状态机；两条通道不共享授权含义。现有 HTTP 接口以节点凭证或已认证所有者和数据库中的节点归属为授权根，并校验路由 `node_id`、信封或计划内 `node_id`、请求身份及 SHA-256 一致；载荷自报身份不能单独构成授权。上传器复用节点宿主的内存凭证，不把长期用户会话或节点密钥写入普通项目目录。

通用 GC `-Apply`、legacy purge、Cargo 父配置激活和平台迁移不能因一次 Git 提交而在所有 PC 自动执行。它们必须在目标 PC 上有明确审批、重新扫描和回执。

## 验收标准

推广到新项目或新 PC 前必须确认：

1. `doctor` 能识别项目清单、规范化源码身份、平台与 Skill 的原始字节完整性、启动器、SCCache、磁盘和活动写入者。
2. 用户启动器不包含 `Start-Process`、`powershell.exe` 或 `pwsh.exe` 嵌套启动。
3. `fleet-report` 通过同一 schema 输出，JSON 中不含本机绝对路径。
4. `fleet-stage` 生成不可变信封，篡改内嵌报告后校验失败，且信封不携带删除授权。
5. 服务端接收接口拒绝节点身份不一致、哈希篡改、未知隐私字段和破坏性授权，重复信封返回幂等 ACK。
6. 节点上传器拒绝远程明文 HTTP，ACK 不匹配时保留 outbox；匹配时先写本地回执再归档信封。
7. 两个 worktree 只有在相同项目与兼容域内才命中同一命名分区，并由同一锁串行化。
8. 未登记项目进入 quarantine，不污染正式项目分区。
9. GC dry-run 不删除；apply 重新取锁并只处理受管路径。
10. 项目回滚只需取消命名共享分区，不需要手工清空缓存根。
11. 远程请求不接受路径或任意命令；同一节点不能并行创建两个活动请求。
12. 审批必须绑定准确计划摘要；计划、活动写入者或候选项漂移时目标 PC 拒绝执行并保留数据。
13. 后台节点启动 PowerShell 时使用隐藏窗口；公网轮询没有可信 TLS 时失败关闭。

底层路径、域、锁和 GC 细节见 `docs/rust-cache-platform.md`；渐进共享策略见 `docs/rust-cache-on-demand-adoption.md`。
