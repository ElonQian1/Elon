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
  - file:scripts/rust-cache/RustCache.Install.psm1
  - file:scripts/rust-cache/RustCache.Launcher.psm1
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
| Fleet 观测层 | 脱敏 `fleet-report` JSON | 节点按需上报 | 中央只读汇总，不远程递归删除目录 |

SCCache 负责兼容编译对象的跨项目复用。命名 Cargo build-dir 只允许在同一 `project_id`、rustc 代际和兼容域内跨 worktree 复用。不同 PC 不应通过网络盘共同写一个 Cargo target 或 build-dir。

## 一台新 PC 的安装流程

在当前可信的一龙仓库检出中执行：

```powershell
& .\scripts\rust-cache.ps1 install -ProjectRoot . -Apply -InstallCodexSkill
& .\scripts\rust-cache.ps1 doctor -ProjectRoot .
```

安装器会写入源码与安装指纹、固定用户启动器和 Codex Skill。`doctor` 报告 `platform-version` 或 `platform-integrity` 失败时，必须从当前可信提交重新安装，不能手工复制单个模块。

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

报告不包含项目根、缓存根、用户名、电脑名、启动器路径或分区绝对路径。`node_id` 必须由一龙节点身份层显式传入，缓存工具不把电脑名当身份。当前仓库已实现本地报告导出；节点上传接口、中央看板和远程审批队列仍是后续集成，不能宣称已经上线。

## GC 业务流

跨 PC 回收必须按以下状态机执行：

```text
fleet-report/status 发现风险
  -> 目标 PC 运行 gc dry-run
  -> 保存计划、大小、活动写入者和锁证据
  -> 人工或受权 AI 审核具体 action
  -> 目标 PC 重新扫描并用 gc -Apply 取分区锁
  -> 原子移入受管 trash 后删除
  -> 生成 GC 回执并重新上报 fleet-report
```

中央控制面可以请求目标节点预演或执行已批准计划，但不能把旧报告当作删除授权，也不能通过远程文件共享直接删除目录。执行时必须重新扫描，因为报告生成后可能出现新的 Cargo 写入者或锁。

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
- 节点空闲时生成并上传脱敏 fleet report。

通用 GC `-Apply`、legacy purge、Cargo 父配置激活和平台迁移不能因一次 Git 提交而在所有 PC 自动执行。它们必须在目标 PC 上有明确审批、重新扫描和回执。

## 验收标准

推广到新项目或新 PC 前必须确认：

1. `doctor` 能识别项目清单、平台指纹、启动器、SCCache、磁盘和活动写入者。
2. 用户启动器不包含 `Start-Process`、`powershell.exe` 或 `pwsh.exe` 嵌套启动。
3. `fleet-report` 通过同一 schema 输出，JSON 中不含本机绝对路径。
4. 两个 worktree 只有在相同项目与兼容域内才命中同一命名分区，并由同一锁串行化。
5. 未登记项目进入 quarantine，不污染正式项目分区。
6. GC dry-run 不删除；apply 重新取锁并只处理受管路径。
7. 项目回滚只需取消命名共享分区，不需要手工清空缓存根。

底层路径、域、锁和 GC 细节见 `docs/rust-cache-platform.md`；渐进共享策略见 `docs/rust-cache-on-demand-adoption.md`。
