---
title: "AI 任务受管 Rust 缓存生命周期 V1"
owner: developer-platform
reviewed_at: 2026-08-15
review_interval_days: 60
role: requirement
lifecycle: active
authority: authoritative
default_retrieval: false
version_status: current
---

# AI 任务受管 Rust 缓存生命周期 V1

## 背景

Windows Rust 缓存平台已经把构建目录统一放入 `rust-cache-v2`，但历史默认仍按
workspace 路径哈希隔离 Cargo 中间产物。密集 AI 任务会创建大量短期 worktree；
worktree 删除后，对应分区仍等待普通 14 天 TTL 或磁盘低水位回收，造成可重建内容
长期占用磁盘。共享根目录并不等于所有 Cargo build-dir 已经真正共享。

## 目标

1. 完成的受管 Codex worktree 在统一收尾删除前，回收只属于该 worktree 的非共享
   Rust build-dir。
2. GC 能识别已经消失、符合受管 `wt/<任务ID>` 形态且超过宽限期的遗留分区，不再
   必须等待普通 TTL 或再次把磁盘压到低水位。
3. 保留命名共享分区、sccache、quarantine、活动锁、未知目录和非受管项目缓存。
4. 所有删除继续通过平台的路径边界、分区锁和原子 trash 流程执行并留下报告。

## 非目标

- 不把多个项目指向同一个可写 `target-dir`。
- 不删除 `sccache`、Cargo registry、发布 target 或未登记 legacy cache。
- 不根据目录大小、文件名相似或 worktree 不在 Git 列表中直接删除未知路径。
- 不处理平台会话、用户长期工作区或外接盘项目的生命周期。

## 安全合同

分区只有同时满足以下条件才可按任务生命周期回收：

1. 位于 `rust-cache-v2/build/<epoch>/<project>/<domain>/<partition>` 受管边界内。
2. 分区名是 16 位 workspace 哈希，并含可解析的 `.last-used.json`。
3. marker 的 `cache_scope` 为 `workspace`；兼容旧 marker 时允许字段缺失，但绝不允许
   `shared`、`quarantine` 或其他未知作用域。
4. marker 的 `workspace_root` 位于精确任务根之内；遗留扫描只接受
   `<盘符>:\wt\<PID>-<8位十六进制>\...` 形态。
5. 定向收尾绑定当前不可变 TaskContract；遗留扫描要求任务根已经不存在并超过策略
   `orphan_task_grace_hours`。
6. 分区无活动锁；删除前再次取得该分区锁并执行受管路径断言。

## 验收标准

1. 定向测试证明统一收尾只删除当前任务的 workspace 分区，其他任务和共享分区保留。
2. 缺失 `D:\wt` 任务根超过宽限期时，普通 GC 即使磁盘未低水位也给出
   `orphaned-task-worktree` 原因；dry-run 不删除，`-Apply` 才删除。
3. 活动锁、近期遗留、任意目录形态、无效 marker 和 `cache_scope=shared` 均失败关闭。
4. 旧 policy 缺少新字段时以24小时安全默认值读取，安装不会覆盖用户既有策略。
5. 缓存平台、统一收尾、预检、Prompt 审计、源码体积和文档模块化门禁通过。
6. 本机安装版与已推送源码哈希一致后，才允许对真实遗留分区执行受控 GC。

## 实现范围

- `scripts/rust-cache/RustCache.TaskLifecycle.psm1`
- `scripts/rust-cache/RustCache.Inventory.psm1`
- `scripts/rust-cache/RustCache.Policy.psm1`
- `scripts/finish-ai-task.ps1`
- `scripts/test-rust-cache-platform.ps1`
- `scripts/test-ai-task-finish-workflow.ps1`
- `docs/rust-cache-platform.md`
- `docs/rust-cache-on-demand-adoption.md`
