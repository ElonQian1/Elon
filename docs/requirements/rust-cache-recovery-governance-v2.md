---
title: "Rust 缓存安全恢复治理 V2"
owner: developer-platform
reviewed_at: 2026-08-15
review_interval_days: 60
role: requirement
lifecycle: active
authority: authoritative
default_retrieval: false
version_status: current
---

# Rust 缓存安全恢复治理 V2

## 背景

V1 已让统一任务收尾定向回收当前 `D:\wt` 任务拥有的 workspace 构建分区，并能在
任务根消失后识别严格命名的遗留分区。真实机器审计仍暴露三个恢复缺口：

1. 历史任务可能由旧收尾或独立 `cleanup-task-worktrees.ps1` 删除，留下大量 marker
   有效、workspace 已不存在但不满足严格 `D:\wt` 孤儿形态的哈希分区。
2. Inventory 只根据 `.rust-cache.lockdir` 是否存在判断占用，已经退出的 PID 留下的
   失效锁会永久阻止 GC。
3. `-ForceAged -Apply` 会按年龄选择不同作用域，缺少只允许 workspace 分区进入本次
   恢复计划的操作边界，不能用于共享缓存保留场景。

## 目标

1. 为显式维护窗口提供“缺失 workspace 恢复”模式，只选择 marker 有效、作用域为
   `workspace`、16 位哈希、workspace 路径已不存在且超过宽限期的受管分区。
2. 提供 workspace-only 过滤器，使共享分区、quarantine、无效 marker 和未知作用域
   无论年龄或磁盘水位都不能进入该次删除计划。
3. 区分活动锁与失效锁。活动锁继续失败关闭；失效锁只有在删除前重新取得同一分区
   锁时才被回收，不能直接手工删除锁目录。
4. 独立 worktree 清理入口在移除 clean、已合并 worktree 前，调用同一套定向 Rust
   缓存回收，并在活动分区锁存在时保留 worktree。
5. 源码、机器安装版和真实 GC 报告一致后，再执行真实遗留 workspace 分区回收。

## 非目标

- 不终止长期运行的 Cargo、rustc、服务进程或其他用户任务。
- 不删除 `shared-*` 分区、`sccache`、Cargo registry、quarantine、发布 target 或未登记
  legacy cache。
- 不删除 worktree 中的源码、未提交文件、脏分支或未合并分支。
- 不把目录不存在当作公开自动 GC 的默认规则；恢复模式必须由维护者显式启用，且
  默认仍为 dry-run。
- 不处理 C 盘任意临时目录；C 盘只按来源、锁、年龄和可重建性单独审计。

## 安全合同

缺失 workspace 分区只有同时满足以下条件才可进入恢复计划：

1. 位于 `rust-cache-v2/build/<epoch>/<project>/<domain>/<partition>` 受管边界内。
2. 分区名严格为 16 位小写十六进制，`.last-used.json` 可解析且 partition 名匹配。
3. `cache_scope=workspace`，`workspace_root` 为绝对路径且当前不存在。
4. marker 最后使用时间超过 `orphan_task_grace_hours`；近期分区仍保留。
5. 不存在活动分区锁。锁 owner PID 不存在或与记录的进程代际不匹配时标记为 stale，
   但实际删除仍必须通过 `Enter-RustCacheLock` 原子接管。
6. `-WorkspaceOnly` 启用时，任何非 workspace 分区均以稳定原因出现在报告中并保留。
7. apply 前后都写 JSON 报告，包含作用域、锁状态、选择原因、预计及实际磁盘水位。

## 验收标准

1. 回归证明 active、stale、invalid 和 absent 四种锁状态可区分，stale 锁不会永久阻止
   GC，活动锁不会被接管。
2. 缺失 workspace 恢复 dry-run 只选择有效、超龄、路径不存在的 workspace 哈希分区；
   当前 workspace、近期分区、shared、quarantine、invalid marker 全部保留。
3. `-WorkspaceOnly` 与 `-ForceAged`、低磁盘选择组合时仍不删除共享分区。
4. apply 通过分区锁和原子 trash 删除候选项，并在锁竞争出现时改为保留。
5. `cleanup-task-worktrees.ps1` 在删除目标 worktree 前回收其 workspace 分区；活动缓存锁
   会阻止 worktree 删除，其他 worktree 和共享分区不受影响。
6. 缓存平台、任务收尾、Prompt 审计、源码体积和文档模块化门禁通过。
7. 本机安装版与已推送源码一致；真实预演列出候选路径与总量，经审计后 apply，最终
   报告证明共享缓存、当前任务 worktree、脏/未合并 worktree 和长期 Cargo 进程均保留。

## 实现范围

- `scripts/rust-cache.ps1`
- `scripts/rust-cache/RustCache.Runtime.psm1`
- `scripts/rust-cache/RustCache.TaskLifecycle.psm1`
- `scripts/rust-cache/RustCache.Inventory.psm1`
- `scripts/cleanup-task-worktrees.ps1`
- `scripts/test-rust-cache-platform.ps1`
- `scripts/test-ai-task-finish-workflow.ps1`
- `docs/rust-cache-platform.md`
- `docs/rust-cache-on-demand-adoption.md`
