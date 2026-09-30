---
version_status: current
reviewed_at: 2026-09-30
---

# 网络缓存与冷归档

本手册补充 [Rust 缓存平台](rust-cache-platform.md) 与 [节点数据根](pc-node-data-root.md)。
网络缓存是本机独占的可重建数据；冷归档是具有内容校验清单的恢复副本。两者不能混为一套删除策略。

## 分层与边界

| 数据 | 推荐位置 | 回收条件 |
|---|---|---|
| 用户凭据、Codex 数据库、节点身份、当前程序 | 本地 | 不属于缓存回收 |
| 项目源码、未提交工作、活动 worktree | 本地 | 仅所属 Git/任务生命周期处理 |
| Rust 中间产物与 sccache | 本地受管根，或经过实测的本机独占网络根 | 平台 GC 重新扫描并取得分区锁 |
| 已同步的旧发布事件、失败终止的发布副本 | 校验后的冷归档 | 发布状态、年龄与运行保护再次通过 |
| 当前版本、待发送事件、未知状态、回滚依赖 | 保留原处 | 不按目录年龄推断可以删除 |

同一目录不能由多台 PC 同时作为可写 Cargo/sccache 根。网络根绑定本机身份；其他机器或
无法证明归属的旧根必须拒绝接管。现有未标记共享副本也不能直接当作可认领空根。

## 接入共享缓存

1. 先运行 `doctor`，确认没有活动 Cargo/rustc 写入者；保留当前根和用户 Cargo 配置备份。
2. 在共享盘选择新的本机专属目录，例如 `\\cache-server\build-cache\pc-<id>\rust-cache-v2`。
3. 由当前平台初始化归属标记；不要手写标记、覆盖另一机器的标记或跳过检查。
4. 分批复制需要复用的缓存，核验内容后再安装。不要复制 `.cargo` 用户凭据，
   也不要覆盖网络根归属标记。旧 registry 内的工作区路径只能作为历史定位数据。
5. 使用正式 `rust-cache.ps1 install` 生成当前平台、原生 wrapper 与 sccache 配置。
   安装前检查已有 Cargo include；不覆盖无关用户配置。
6. 在实际使用的用户会话运行 `doctor`、`status -IncludeSizes`、GC 预演和小型真实编译。
   从 sccache 统计确认实际缓存位置。源码、临时文件与最终测试程序仍可保留本地。
7. 验证通过后才激活用户配置和 launcher。既有进程可能仍持有旧环境变量，
   当前任务应显式传 `-CacheRoot`；新会话再次检查解析结果。

优先使用 UNC，不依赖只在某个登录会话可见的映射盘符。共享盘断线时，当前实现不会静默
回退 C/D 并重新生成大缓存；必须报告故障，再由操作者选择可用且有足够空余的本地根。
锁重试有上限，底层 SMB I/O 仍受操作系统网络超时影响。长构建必须继续使用项目日志执行器。

## 校验归档

`scripts/node-storage-archive.ps1` 提供 `Invoke-ElonVerifiedTreeArchive` 与恢复入口。
它仅负责复制、完整性与路径边界，删除权限仍由拥有该数据的管理工具决定。

```powershell
. .\scripts\node-storage-archive.ps1
Invoke-ElonVerifiedTreeArchive -Path <exact-candidate-directory> `
  -AllowedRoot <owning-managed-root> -ArchiveRoot <archive-share> -Mode ArchiveOnly
```

归档按机器和操作 ID 隔离，保存 `payload`、逐文件 SHA-256 `manifest.json` 与
`verified.json` 回执。源与目标不得重叠，拒绝重解析点与嵌入的 Git 仓库。
复制失败、目标校验失败、源目录变化或状态失效时保留源，不把半份副本标记成成功。

`ArchiveOnly` 不回收原件。归档成功后若需释放本地空间：

- Rust 分区重新运行官方 GC 预演，核对精确候选与已验证副本；实际 GC 仍受分区锁保护。
- 外部旧 target 先 `register-legacy -Retired`，再 `purge-legacy` 预演；不得用归档函数替代登记。
- 发布历史使用维护入口的发布状态筛选和归档回收模式；不得把整棵发布状态根作为候选。

发布历史先生成只读计划，检查候选、保留项和预计字节，再用输出的精确摘要执行：

```powershell
& .\scripts\inspect-node-disk-usage.ps1 -ReleaseHistoryOnly -ArchiveMode Preview `
  -ArchiveRoot <archive-share> -ArchivePlanPath <new-local-plan.json>
& .\scripts\inspect-node-disk-usage.ps1 -ReleaseHistoryOnly -ArchiveMode ArchiveAndReclaim `
  -ArchiveRoot <same-archive-share> -ArchivePlanPath <same-local-plan.json> `
  -ExpectedPlanSha256 <printed-sha256>
```

计划绑定本机、原路径、共享目标、发布状态和文件元数据，24 小时后失效；执行前重新验证。
默认 `-ReleaseLocation Legacy` 读取旧 `%LOCALAPPDATA%\Elon`。显式 `Managed` 仅使用
通过节点所有权标记验证的数据根 `release-state`，预演与执行必须选择同一位置；不接受任意根路径。
`-CandidatePath` 与 `-CandidateKind` 只能在预演时缩小范围。`ArchiveOnly` 可验证副本而保留原件。
该入口保护当前安装、最新保留版本、发布写入者、pending 状态和全部回滚目录。
目录修改时间仍参与计划和年龄审查；实际复制前后比较目录成员及文件内容与修改时间，
避免 Windows 延迟刷新目录时间造成误报。网络归档不能充当活动数据库的在线备份。

恢复到不存在的目标，禁止覆盖现有目录：

```powershell
Restore-ElonVerifiedTreeArchive -ArchiveDirectory <one-verified-archive> `
  -Path <new-restore-directory> -AllowedRoot <approved-restore-parent>
```

恢复会重验清单和 payload 哈希。成功归档不代表当前安装、回滚或 pending 发布可以删除。

## 避免再次积累

- Cargo 用户 include、项目 wrapper、launcher 和用户环境必须指向同一个预期根。
- 每个任务只使用已有命名分区，不按功能或会话增加永久共享分区。
- 发布完成后由维护入口盘点终态历史，保留当前与必要回滚；不把未知状态改成终态来通过清理。
- 外部工作区、渲染器 slot 和旧 worktree 应先修复其所属生命周期，不按 `build`/`target` 名字整树删除。
- 发布与节点数据根迁移会留下旧根；旧根需要单独盘点，设置新根不会自动清掉旧副本。
- 按磁盘空余与本地缓存配额治理；共享容量充足不等于本地无需预留构建临时空间。

本手册不创建定时任务，也不授权自动删除未知数据。每次执行保留扫描、归档、回收与磁盘余量回执。
