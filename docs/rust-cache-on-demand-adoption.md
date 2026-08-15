---
title: "跨项目 Rust 缓存按需共享与渐进接入"
owner: platform
reviewed_at: 2026-08-10
review_interval_days: 60
role: runbook
lifecycle: active
authority: authoritative
default_retrieval: false
version_status: current
implementation_refs:
  - file:scripts/rust-cache.ps1
  - file:scripts/rust-cache/RustCache.Scope.psm1
  - file:scripts/rust-cache/RustCache.Runtime.psm1
  - file:scripts/rust-cache/RustCache.Inventory.psm1
  - file:scripts/test-rust-cache-platform.ps1
---

# 跨项目 Rust 缓存按需共享与渐进接入

最后更新：2026-08-10
版本：v1（当前）

本文只在新增 Rust 项目、创建大量 worktree、发现重复编译目录、调整缓存作用域或执行缓存迁移时按需读取。它是所有本机项目的共享接入合同；各项目只记录自己的启用状态和入口，不复制本文。

## 目录

1. [现有方式的问题](#现有方式的问题)
2. [目标分层](#目标分层)
3. [按需共享合同](#按需共享合同)
4. [项目适配器合同](#项目适配器合同)
5. [渐进启用顺序](#渐进启用顺序)
6. [验证、回滚与清理](#验证回滚与清理)
7. [非 Rust 缓存的后续路线](#非-rust-缓存的后续路线)

## 现有方式的问题

2026-08-10 对 `D:\rust\shared\rust-cache-v2` 和多个项目 worktree 的检查暴露了以下问题。这里记录的是迁移动机，不表示这些缺陷必须永久保留。

| 问题 | 根因 | 结果 | 本方案状态 |
|---|---|---|---|
| 同项目每个 worktree 都出现大型 Cargo 中间目录 | 默认分区使用绝对 workspace 路径的 16 位哈希 | 路径不同即复制依赖编译产物；共享根看起来统一，build-dir 实际仍按 worktree 分裂 | 保留为安全默认，增加显式命名共享分区 |
| 共享分区能力难以由其他项目使用 | `SharedBuildPartition` 原先只在运行时模块和少量内部脚本中存在，机器级入口没有公开参数 | 项目适配器只能继续创建 workspace 分区，或自行复制平台逻辑 | 机器入口统一公开 `-SharedBuildPartition` |
| 共享写入容易绕过锁 | 只设置 `CARGO_BUILD_BUILD_DIR` 不能覆盖 Cargo 进程完整生命周期 | 两个 worktree 可能同时写同一 build-dir，导致争用或损坏 | 共享分区禁止与 `-NoLock` 同用，只允许锁覆盖完整 Cargo 调用的入口 |
| 旧安装版 GC 会被任意 Cargo/rustc 阻塞 | 已安装的 Inventory 仍使用机器级全局进程守卫 | 一个长期运行的开发服务会阻止其他已锁托管分区回收 | 平台源码已改为分区锁；安装升级应单独排期，不在业务项目提交中静默替换 |
| 短期 worktree 删除后 workspace 分区仍保留 | Git worktree 生命周期与 `rust-cache-v2\build` 生命周期原先彼此独立 | 长会话和多代理任务会留下大量无法再命中的绝对路径哈希分区 | 统一收尾定向回收当前任务分区，GC 在 24 小时宽限后识别严格命名的失效任务根 |
| 最终产物与可重建缓存容易混为一谈 | 历史上把通用 `target` 同时当缓存池和发布产物目录 | 清理风险大，发布脚本之间互相污染 | build-dir 归平台治理；target-dir 继续由 workspace 或发布脚本明确拥有 |
| Node 依赖在每个 worktree 重复 | npm 的 `node_modules` 是按 checkout 实体化的安装树 | 多前端项目和 worktree 重复占盘 | 不由 Rust 缓存脚本处理，后续采用共享内容仓库的包管理方案 |

任何“共享根目录”都不自动等于“缓存已经共享”。判断共享是否成立，必须同时检查缓存键、分区路径、写锁、生命周期和最终产物边界。

## 目标分层

| 层 | 共享范围 | 是否按需 | 所有者 |
|---|---|---|---|
| Cargo registry/git | 当前用户的 Rust 项目 | 否 | Cargo |
| sccache | 兼容的 rustc 输入，可跨项目与 worktree | 否 | 机器缓存平台 |
| workspace build-dir | 单个 workspace | 默认 | 机器缓存平台 |
| named shared build-dir | 同一 rustc 代际、项目、domain 和稳定命名分区 | 是 | 机器缓存平台与项目适配器共同保证 |
| quarantine | 未注册项目或裸 Cargo | 自动 | 机器缓存平台 |
| target-dir | workspace 最终产物，或发布脚本的专用绝对目录 | 显式 | 项目/发布流程 |
| legacy cache | 平台外旧目录 | 只登记 | 原目录所有者 |

命名共享 build-dir **不会跨不同 `project_id` 复用**。真正跨项目的编译对象复用由 sccache 提供；命名共享分区解决的是同一项目多个 worktree 的重复 Cargo 中间产物。

## 按需共享合同

只有同时满足以下条件，项目才能启用命名共享分区：

1. 项目根存在受审查的 `rust-cache.project.json`。
2. `domain` 表示稳定兼容边界，不使用任务名、会话 ID、Git SHA 或 worktree 路径。
3. `SharedBuildPartition` 表示稳定用途，例如 `dev-windows`、`validation-light-0` 或 `windows-release`；名称必须已经是小写 slug，只使用字母、数字、点、下划线或连字符。
4. Cargo 的整个进程生命周期都由 `Invoke-RustCacheCargo` 或 `rust-cache.ps1 run` 包裹。
5. 不传 `-NoLock`；平台会对命名共享分区串行写入。
6. 最终 `target-dir` 仍保持 workspace 本地，或使用发布流程专属的绝对路径。
7. 首次启用先选一个低风险入口，观察命中率、等待时间和磁盘增长，再扩大范围。
8. 会被多个入口调用的稳定分区必须登记到 `shared_partition_domains`，由平台把分区
   名称绑定到唯一 allowlist domain；不能只靠包装器约定 domain。

推荐用统一入口完成项目登记，而不是手写 JSON：先运行 `rust-cache.ps1 init-project` 预览，
确认项目 ID、允许域和命名共享分区后再追加 `-Apply`。接入前后均运行 `doctor`；
若它报告安装指纹漂移，先从当前权威仓库重新 `install`，不要让各项目复制缓存实现。

默认入口保持 workspace 隔离：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 run `
  -ProjectRoot . -Domain dev-windows-msvc `
  check --manifest-path server\Cargo.toml --locked
```

显式启用同项目、同 domain 的命名共享分区：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 run `
  -ProjectRoot . -Domain dev-windows-msvc `
  -SharedBuildPartition dev-windows `
  check --manifest-path server\Cargo.toml --locked
```

运行时必须输出以下可审计字段：

```text
RUST_CACHE_SCOPE=shared
RUST_CACHE_PARTITION=shared-dev-windows
RUST_CACHE_SHARED_DOMAIN_CANONICALIZED=False
```

未传参数时应输出 `RUST_CACHE_SCOPE=workspace`，以便确认默认隔离没有被隐式改变。未注册项目传共享分区会失败，不会进入托管共享池。

## 项目适配器合同

项目适配器保持薄层，只负责：

- 解析项目根和机器缓存根；
- 选择本项目已审查的 domain；
- 把可选 `SharedBuildPartition` 原样传给机器入口；
- 保持 Cargo 参数逐项透传，不重写 feature、target 或 profile；
- 在本项目文档记录哪些入口已启用、哪些仍关闭。

项目适配器不得复制 scope 解析、注册表、GC、sccache 或锁实现。仅调用 `Set-RustCacheBuildEnvironment` 后再由调用方裸跑 Cargo 的旧入口，暂时不能启用命名共享分区，因为该函数无法持有覆盖后续 Cargo 生命周期的锁；这类入口应先重构成由平台直接执行 Cargo。

## 渐进启用顺序

| 阶段 | 动作 | 当前状态 |
|---|---|---|
| 0. 兼容基线 | 所有未改造入口继续使用 workspace hash；sccache 保持跨项目共享 | 已启用 |
| 1. 平台能力 | 公开命名共享参数、拒绝无锁共享、输出 scope/partition、补回归测试 | 已实现，待安装版按维护窗口升级 |
| 2. 项目薄适配 | 项目的直接 Cargo 包装器转发可选共享分区，默认关闭 | BB64A 首批接入 |
| 3. 选择性启用 | 先启用受控验证或开发入口；发布入口逐个确认 target 与并发模型 | 待逐项目执行 |
| 4. 收敛旧分区 | 任务收尾处理自身 workspace 分区；GC 识别失效任务分区和已被 canonical domain 取代的共享别名，审查后再 `-Apply` | 平台能力已实现，机器安装版需同步验证 |
| 5. Node 依赖 | 评估 pnpm 共享内容仓库和按项目虚拟安装树，不直接共享可写 `node_modules` | 未开始 |

平台安装与项目启用是两件事：先提交并验证平台源码，再安排没有 Cargo/rustc 写入者的维护窗口更新机器安装版，最后由项目显式选择共享分区。不得因为平台支持该参数就批量修改所有项目默认值。

## 验证、回滚与清理

项目启用前后至少验证：

1. 两个不同 worktree 使用相同 `project_id + rustc epoch + domain + SharedBuildPartition` 时得到同一 build-dir。
2. 同一分区的并发调用会等待锁，而不是同时写入。
3. 不同 domain、不同 rustc 代际和不同项目仍落入不同路径。
4. `CARGO_TARGET_DIR` 没有被改成共享 build-dir。
5. `gc` 默认 dry-run；有锁分区和 quarantine 风险区保持不动。
6. 受管任务收尾只移除 marker 指向当前任务根的 workspace 分区，不移除同一项目的命名共享分区。
7. 已消失的严格任务根在宽限期后显示为 `orphaned-task-worktree`；近期、任意普通缺失路径、无效 marker 和未知作用域不被该规则选中。
8. 同一登记分区从不同 allowlist domain 发起时解析到同一 build-dir；旧 domain 副本
   只在 canonical shared marker 有效后显示为 `retired-shared-alias`。
9. `gc -SharedAliasesOnly` 只选择 canonical 分区已经就绪的历史共享别名，不同时选择
   workspace、quarantine、普通超龄分区或缺失工作区恢复候选。

回滚只需移除项目入口的 `-SharedBuildPartition`，下一次调用会恢复 workspace 分区。旧共享分区仍是可重建缓存，由平台 TTL/LRU 治理；不要用 `cargo clean` 指向共享根，也不要手动递归删除平台根。

确认命名共享分区稳定后，如需按普通 TTL 处理旧 workspace，必须显式限制作用域：

```powershell
# 先查看计划
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 gc -ForceAged -WorkspaceOnly

# 审查报告后才应用
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 gc -ForceAged -WorkspaceOnly -Apply
```

如果旧流程已经移除了 workspace，优先使用更精确的
`gc -RecoverMissingWorkspaces -WorkspaceOnly`；它不等待普通 14 天 TTL，但仍要求 marker
有效、路径不存在、超过孤儿宽限期且没有活动锁。独立 worktree 清理入口也会在移除
clean、已合并 worktree 前定向回收其 workspace 分区，不再制造新的遗留哈希目录。

如果安装版仍使用全局 Cargo/rustc 守卫，GC 被活动进程拒绝是安全降级，不应通过强杀无关开发服务来绕过；先升级并验证平台安装版。

## 非 Rust 缓存的后续路线

Node 依赖不复用 Rust 分区模型。后续跨项目方案应使用内容寻址的共享 store（优先评估 pnpm），每个 workspace 仍保留自己的锁文件和链接树。迁移前先验证现有 npm 脚本、native addon、Tauri/Android 构建与 CI 兼容性；不得把多个项目直接指向同一个可写 `node_modules`。

Gradle、Android SDK、Cargo registry/git 和浏览器下载缓存已经具备不同程度的机器级共享能力，应继续保留。清理时只删除能够证明为同内容重复、且可由共享层重建的独立副本。
