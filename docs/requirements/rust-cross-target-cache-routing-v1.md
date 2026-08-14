# Rust 跨目标验证缓存路由 V1

## 目标

为 Windows 上的 Linux-musl 等 Rust 跨目标验证提供唯一、稳定的项目入口，避免 AI
代理按功能编号或会话编号创建新的 `CARGO_TARGET_DIR`，也避免把构建产物写入 WSL
发行版的 `/tmp`、系统盘 VHDX 或仓库外未治理目录。

## 当前问题

V260、V261 的 Linux-musl 验证曾直接使用 `D:\rust\shared\target-v260-linux-musl`、
`D:\rust\shared\target-v261-linux-musl` 等独立 target。源码和 worktree 本身体积很小，
但每个 target 都会复制完整依赖、中间产物和 fingerprint，无法进入现有
`rust-cache-v2` 的分区锁、LRU、TTL 与磁盘水位治理。

## 范围

1. 新增 `scripts/cargo-cross.ps1`，要求 Cargo 参数包含一个标准 target triple。
2. target triple 确定稳定的 `agent-validation/shared-cross-<target>` 构建分区；不得包含
   Git SHA、任务 ID、功能编号或会话编号。
3. 最终产物固定写入当前工作区 `.ai-tmp/cargo-cross-target/<target>`，由任务统一收尾
   清理；依赖和中间产物进入 `rust-cache-v2` 托管 build-dir。
4. 支持 `cargo zigbuild`、`cargo build`、`cargo check` 等 Cargo 子命令原样透传，调用方
   仍需自行提供目标工具链和必要的编译器环境。
5. 提供不启动 Cargo 的 `-PlanOnly`，供代理和合同测试核对真实路由。

## 非目标

- 不删除或迁移任何现存外部 target。
- 不把历史 target 自动纳入平台 GC；需要释放空间时先用 `register-legacy` 登记，再用
  `purge-legacy` 预演和显式执行。
- 不替代服务器发布脚本，也不改变生产发布 target。
- 不在本批次安装 Zig、cargo-zigbuild、Linux 工具链或 WSL。
- 不为自定义 JSON target 生成隐式缓存键；V1 对这类 target 失败关闭。

## 验收标准

1. 相同 target triple 在不同任务中生成相同共享分区名。
2. 最终 target 始终位于当前 Git 工作区的 `.ai-tmp` 内。
3. 缺少 target、冲突 target、自定义 JSON target 和不稳定名称均失败关闭。
4. Cargo 参数在 `--` 后逐项原样透传，包装器不直接运行裸 Cargo。
5. 计划模式不创建目录、不启动编译，并输出有界 JSON 合同。
6. 项目 Rust 缓存文档和 AI Git/验证手册指向该入口，禁止新的会话专属 target。
