# Windows 多项目 Rust 编译缓存平台

最后更新：2026-07-17

## 目标

本平台解决同一台 Windows 开发机上多个 Rust 项目、多个 worktree 和多个发布目标同时使用缓存时的三个问题：

1. 跨项目可以复用的内容没有真正进入 sccache；
2. 所有项目共用一个 `CARGO_TARGET_DIR`，导致 incremental、features、rustflags 和工具链代际在同一目录无限累积；
3. 旧缓存失去写入者后没有登记、退役和安全清理流程。

`bb64a` 和 `elon cli` 是首批参考接入项目，不是平台边界。任何项目都可通过根目录的 `rust-cache.project.json` 注册。

## 分层合同

| 层 | 共享范围 | 生命周期 |
|---|---|---|
| Cargo registry/git | 当前用户所有 Rust 项目 | Cargo 自身管理 |
| sccache | 所有兼容 rustc 调用 | 内容寻址、LRU，独立容量设置 |
| Cargo build-dir | 工具链 + 项目 + domain + workspace hash | 平台磁盘水位、LRU/TTL 治理 |
| Cargo target-dir | 当前 workspace，发布脚本可显式覆盖 | 只承载最终产物，不作为全局缓存池 |
| 历史 target | 外部 legacy | 只读登记；不会被平台自动删除 |

机器级根默认解析顺序：

1. `-CacheRoot`；
2. `ELON_RUST_CACHE_ROOT`；
3. `RUST_SHARED_BUILD_ROOT\rust-cache-v2`；
4. 存在 `D:\rust\shared` 时使用 `D:\rust\shared\rust-cache-v2`；
5. `%LOCALAPPDATA%\Elon\rust-cache-v2`。

目录结构：

```text
rust-cache-v2\
├─ build\<rustc-epoch>\<project-id>\<domain>\<workspace-hash>\
├─ quarantine\<Cargo 分片 workspace-path-hash>\
├─ sccache\
├─ platform\
├─ config\
│  ├─ policy.json
│  └─ cargo-cache.toml
├─ state\registry.json
└─ reports\gc-*.json
```

## 项目注册

项目根目录添加：

```json
{
  "schema_version": 1,
  "project_id": "my-project",
  "default_domain": "dev-windows-msvc"
}
```

注册项目进入兼容域目录。未注册项目仍可编译，但只进入 `quarantine`，防止静默污染正式缓存池。

domain 应描述构建用途和兼容性，例如：

- `dev-windows-msvc`
- `windows-release-portable`
- `server-musl-release-portable`
- `android-aarch64-release`

不要把 feature 列表或 Git SHA 拼进 domain。Cargo 与 sccache 已负责精确指纹；domain 只负责隔离明显不兼容的构建类别。

## 使用入口

状态与真实大小：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 status -IncludeSizes
```

通过平台执行 Cargo：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 run `
  -ProjectRoot . -Domain dev-windows-msvc `
  check --manifest-path server\Cargo.toml
```

一龙仓库日常验证继续使用稳定入口，它已委托给平台：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 check --manifest-path server\Cargo.toml
```

安装到机器缓存根，但不修改 Cargo 父级配置：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 install
```

确认预览后激活父级 Cargo 配置：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 install -Apply `
  -CargoConfigPath D:\rust\.cargo\config.toml
```

激活操作会：

- 备份原 `config.toml`；
- 移除父级全局 `target-dir` 和 `rustflags`；
- 引入生成的 `cargo-cache.toml`；
- 给未走项目入口的裸 Cargo 提供 workspace-hash quarantine；
- 全局启用 sccache（已安装时）。

## 磁盘治理

平台不按项目平均分配固定容量。策略使用磁盘水位：

- `warning_free_percent`：低于该水位才启动冷分区回收；
- `recovery_free_percent`：回收到该水位即停止；
- `critical_free_percent`：低于该水位且无法安全回收时，拒绝开始新的平台构建；
- `partition_ttl_days`：低水位下的普通冷分区年龄；
- `old_epoch_ttl_days`：旧 rustc 代际的优先回收年龄。

GC 默认 dry-run：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 gc
```

实际执行只会删除 `rust-cache-v2` 根内、无活动锁的托管分区：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 gc -Apply
```

任何 Cargo/rustc 进程活动时，实际 GC 会拒绝执行。每次计划或执行都写入 `reports\gc-*.json`。

## release 与 sccache

平台检测到 `--release` 或 `--profile release` 时设置 `CARGO_INCREMENTAL=0`。这是为了避免 release incremental 无限积累，也让更多库 crate 可以进入 sccache。

开发 profile 可以继续使用 incremental；sccache 会复用可缓存的非 incremental 编译。`SCCACHE_BASEDIRS` 同时剥离当前 workspace 根和项目根，而不是它们的共同父目录；这样不同 checkout/worktree 的绝对根不会进入缓存键，相同编译输入才能跨目录命中。

## 旧缓存迁移

先登记，不直接删除：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 register-legacy `
  -LegacyPath D:\rust\common\rust_backend\target `
  -Label old-common-target -Retired
```

legacy 记录永远显示为 `external-report-only`，不会被自动 GC 删除。登记为 `retired` 后，可以通过受策略约束的精确路径命令清理。命令默认只预演；`-Apply` 会再次确认登记状态、目录形态、重解析点和 Cargo/rustc 写入进程，并写入 JSON 审计报告：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 purge-legacy `
  -LegacyPath D:\rust\common\rust_backend\target

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 purge-legacy `
  -LegacyPath D:\rust\common\rust_backend\target -Apply
```

## 验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-rust-cache-platform.ps1
```

测试覆盖注册/隔离路由、release incremental、环境恢复、陈旧锁、GC dry-run、安全路径边界和 Cargo 父配置迁移。
