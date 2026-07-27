# Windows 多项目 Rust 编译缓存平台

最后更新：2026-07-17

## 目标

本平台解决同一台 Windows 开发机上多个 Rust 项目、多个 worktree 和多个发布目标同时使用缓存时的三个问题：

1. 跨项目可以复用的内容没有真正进入 sccache；
2. 所有项目共用一个 `CARGO_TARGET_DIR`，导致 incremental、features、rustflags 和工具链代际在同一目录无限累积；
3. 旧缓存失去写入者后没有登记、退役和安全清理流程。

`bb64a` 和 `elon cli` 是首批参考接入项目，不是平台边界。任何项目都可通过根目录的 `rust-cache.project.json` 注册。

NodeAgent 发布为 Windows、显式 Linux 和 Desktop 壳分别使用稳定命名分区，并由同一分区锁串行化。这样即使 `sccache` 暂时不可用，依赖编译产物也能跨 worktree 复用；普通开发构建仍保持 worktree 隔离。

## 分层合同

| 层 | 共享范围 | 生命周期 |
|---|---|---|
| Cargo registry/git | 当前用户所有 Rust 项目 | Cargo 自身管理 |
| sccache | 所有兼容 rustc 调用 | 内容寻址、LRU，独立容量设置 |
| Cargo build-dir | 默认：工具链 + 项目 + domain + workspace hash；串行发布可显式使用命名共享分区 | 平台磁盘水位、LRU/TTL 治理 |
| Cargo target-dir | 当前 workspace，发布脚本可显式覆盖 | 只承载最终产物，不作为全局缓存池 |
| 历史 target | 外部 legacy | 只读登记；不会被平台自动删除 |

机器级根默认解析顺序：

1. `-CacheRoot`；
2. `ELON_RUST_CACHE_ROOT`；
3. `ELON_NODE_DATA_ROOT\cache\rust-cache-v2`；
4. `RUST_SHARED_BUILD_ROOT\rust-cache-v2`；
5. `%APPDATA%\elon-node-agent\node.json` 中持久化的 `node_data_root\cache\rust-cache-v2`，但仅接受绝对路径、已存在目录且含 `.elon-node-data-root.json` 所有权标记的根；配置损坏、相对、无标记或目录不存在时安全回退；
6. 存在 `D:\rust\shared` 时使用 `D:\rust\shared\rust-cache-v2`；
7. `%LOCALAPPDATA%\Elon\rust-cache-v2`。

解析只选择写入位置，不会搬迁或删除 C/D 盘旧缓存，也不会修改系统或用户环境变量。

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

注册项目可以在 `rust-cache.project.json` 中声明 `allowed_domains` 和
`unknown_domain_fallback`。这样外部代理误把任务名、会话名或一次性验证名传给
`-Domain` 时，会在验证指纹和 build-dir 路由前收敛到稳定兼容域，避免每个任务
复制一套依赖中间产物。一龙项目把未知 domain 收敛到 `agent-validation`；发布入口
仍必须显式使用 `node-agent-release` 等已登记稳定域。GC 会把当前项目中不再位于
白名单的旧 domain 标记为 `retired-domain`，即使它尚未达到常规 TTL 也会优先回收；
带锁分区和正在运行的 Cargo/rustc 仍受原有安全保护。

## 使用入口

状态与真实大小：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 status -IncludeSizes
```

通过平台执行 Cargo：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 run `
  -ProjectRoot . -Domain dev-windows-msvc `
  check --manifest-path server\Cargo.toml --locked
```

一龙仓库日常验证继续使用稳定入口，它已委托给平台：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 -- check --manifest-path server\Cargo.toml --locked
```

`--` 是包装器参数与 Cargo 参数的强制边界；边界后的 `-p`、`-F`、`-r`、`-j`、`-q`、`-v` 等短选项和值逐项原样透传。包装器自身选项必须写在边界之前。`validate-rust.ps1` 使用相同契约，并在委托 `cargo-dev.ps1` 时保留该边界。

安装到机器缓存根，但不修改 Cargo 父级配置：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 install
```

确认预览后激活父级 Cargo 配置：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 install -Apply `
  -CargoConfigPath D:\rust\.cargo\config.toml
```

若父级配置还永久设置了 `source.crates-io.replace-with`，只在已审查并确认无
Cargo/rustc 写入者时显式迁移：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\rust-cache.ps1 install -Apply `
  -CargoConfigPath $env:USERPROFILE\.cargo\config.toml -ResetCargoSourcePolicy
```

安装器先拒绝活动写入者、备份原文件，再原子替换；只移除 crates.io 的
`replace-with` 键，保留未激活镜像定义和其他用户配置。日常验证不会永久改写
全局源。

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

Win NodeAgent 发布中的 Rust 构建必须通过 `Invoke-RustCacheCargo` 进入
`node-agent-release` domain，不能在发布脚本中裸跑 `cargo build`。节点主程序
仍按 commit 身份重新链接；未变化的 Desktop 壳和 PC 前端则按 Git tree、
工具链及相关构建环境计算输入哈希，复用经过完整构建产生的不可变产物。
Linux PC 节点发布默认关闭，显式 `-IncludeLinux` 时才进入同一缓存入口。

开发 profile 可以继续使用 incremental；sccache 会复用可缓存的非 incremental 编译。平台从 workspace 注册表生成 `config/sccache-config`，其中的 `basedirs` 同时包含仍存在的 workspace 根、项目根、build-dir 和最终 target-dir，而不是它们的共同父目录；这样源码绝对根和每个分区特有的 `--out-dir` 都不会进入缓存键，相同编译输入才能跨目录命中。

sccache 是常驻客户端/服务器模型，配置变化必须由服务器重新加载。平台只在没有 Cargo/rustc 写入者时重启；并发构建期间会把配置哈希和 pending 状态写入 `state/sccache-sync.json`，以后每次平台构建都重试，直到空闲窗口完成加载。安装器不会把 pending 报告成成功。安装激活还会写入当前 Windows 用户的 `SCCACHE_CONF`、`SCCACHE_DIR`、`SCCACHE_CACHE_SIZE`，所以服务器空闲退出后也会从托管配置自动启动。

sccache 会把所有 `CARGO_*` 环境变量计入 Rust 缓存键。平台用当前 Rust toolchain 生成原生 `platform/rustc-sccache-wrapper.exe`：Cargo 仍先读取真实的 `CARGO_BUILD_BUILD_DIR` / `CARGO_TARGET_DIR` 完成路由，但 wrapper 在启动 rustc/sccache 前移除这两个仅用于目录选择的变量，避免每个隔离分区人为制造不同缓存键。其他会影响编译结果的 Cargo 环境变量不会被移除。这里必须使用原生可执行文件，不能使用 `.cmd`，否则大型 Windows crate 的 rustc 参数会撞上批处理命令行长度上限。

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

### locked、网络诊断与受信源

应用仓库跟踪 `server/Cargo.lock`，所有正式 check/fetch 都带 `--locked`。验证先用
当前依赖缓存执行 `--offline`；命中时完全不探网。只有分类为
`CARGO_OFFLINE_MISSING` 才进入网络阶段，编译错误、缓存锁和磁盘临界不会换源重试。

受信策略位于 `scripts/validation/cargo-sources.json`，初始仅含 crates.io 官方 sparse、
RsProxy 运营方 sparse 和 USTC 官方帮助确认的 sparse。每个源先检查 HTTPS、受限
重定向、`config.json` 和由 Cargo.lock 推导的下载端点，再在独立持久 Cargo home 中
执行限时 `cargo fetch --locked`；Cargo 校验 lockfile checksum 后，以同一源缓存运行
`cargo check --locked --offline`。短期健康结果会缓存，连续失败会打开熔断器，整个
failover 有总预算；全局 Cargo 配置和凭据不会被读取、复制或改写到这些受管 home。

稳定诊断码包括 `CARGO_INDEX_FAILURE`、`CARGO_CRATE_DOWNLOAD_FAILURE`、
`CARGO_DNS_FAILURE`、`CARGO_TLS_FAILURE`、`CARGO_PROXY_FAILURE`、
`CARGO_GIT_DEPENDENCY_FAILURE`、`CARGO_CACHE_LOCKED`、`CARGO_DISK_CRITICAL`、
`CARGO_OFFLINE_MISSING` 和 `RUST_COMPILE_ERROR`。报告使用
`elon.cargo_network_report.v1`。

全部受信源失败时输出 `CARGO_SOURCE_REPAIR_REQUIRED`、报告路径以及
`elon.ai.cargo_source_repair.v1` 接管协议。AI 只能从官方/运营方页面寻找候选，再用：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-source-repair.ps1 `
  -Index sparse+https://operator.example/index/ `
  -Evidence https://operator.example/official-cargo-doc
```

候选必须通过同运营方 HTTPS 证据、重定向、config/download、Cargo.lock/checksum 和
隔离 locked fetch/check；该入口只允许临时继续。永久加入必须提交受信策略和故障注入
测试。随机网页镜像、HTTP 源、跨运营方危险重定向和带凭据 URL 一律拒绝。

### 受管验证证据与调度

代理、Desktop cargo-dev 和 pre-push 使用 `scripts/validate-rust.ps1`。昂贵 Cargo 首次执行的完整 stdout、stderr、退出码、失败项和有界摘要写入 `<cache-root>/validation-v1/evidence/<fingerprint>/`；对话流截断不再成为重跑理由。相同成功指纹直接复用，相同运行中指纹合并等待；锁 owner PID 失效时可恢复。证据默认保留 14 天且最多 100 份，活动锁不会被回收。

指纹覆盖 server/build/dependency 输入、Cargo.lock、`.cargo/config*`、`rust-toolchain*`、`rustc -vV`、`cargo -V`、domain、target/features/rustflags、验证/网络脚本版本、执行选项和规范化 Cargo 参数。编译相关环境变量只保存值的 SHA-256，不持久化原值；无 origin 项目使用规范化根目录的哈希身份。

调度锁使用随机 lease id、PID 与进程启动身份三元组；PID 被复用时旧锁失效，旧 owner 不能删除后继 lease。最多两个 light 验证并行；heavy 验证持有 heavy gate 并预留全部 light 槽，因此与所有 light/heavy 任务互斥。证据暴露 owner、waiter、queue wait 和 resource class。直接调用 `validate-rust.ps1` 默认使用 `agent-validation` domain；普通 `cargo-dev.ps1` 仍默认使用增量开发 domain，并完整转发验证参数。

sccache 每轮明确输出 `SCCACHE_STATUS`、`SCCACHE_PATH`、命中/未命中统计或 `SCCACHE_DEGRADED_REASON`。缺失时继续必要验证，不联网安装；恢复方式是显式安装 sccache 后运行 `scripts/rust-cache.ps1 install -Apply`。`agent-validation` domain 和 release 设置 `CARGO_INCREMENTAL=0`，普通交互开发不改变 incremental。

Rust 推送收据门禁默认关闭；普通 push 不准备或要求收据，也不会因此运行 `cargo check`。需要恢复原有 fail-closed 门禁时，在当前进程显式启用：

```powershell
$env:ELON_ENABLE_RUST_PUSH_RECEIPT='1'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\prepare-push.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\push.ps1
```

`prepare-push` 对当前 Rust 输入计算精确键，有效收据立即复用；缺失或失效才执行完整
验证。启用变量后，`push.ps1` 先准备收据再打开 Git push，版本化 pre-push 也恢复
fail-closed：有效收据只跑廉价门禁，缺失/失效则补完整验证。未启用时两个入口均明确
输出 `RUST_PUSH_RECEIPT_GATE=disabled` 并跳过收据门禁。

缓存根优先接受 `ELON_NODE_DATA_ROOT/cache/rust-cache-v2`。系统盘低水位时仅输出 `elon.rust_cache.migration_advice.v1` 建议，不自动移动或删除；迁移必须先设置新根、登记 legacy，再运行 `purge-legacy` dry-run，确认后才可 `-Apply`。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-rust-cache-platform.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-validation-orchestrator.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-cargo-network.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-rust-push-receipt-gate.ps1
bash scripts/test-cargo-source-contract.sh
```

测试覆盖注册/隔离路由、release incremental、环境恢复、陈旧锁、GC dry-run、安全路径边界和 Cargo 父配置迁移。
