---
version_status: current
reviewed_at: 2026-08-23
implementation_status: proposed
---

# 后端 A2b2 fixture 编译导入修复 V1

## 目标

恢复 `elon-pc-node` 测试目标中 A2b2 静态库存 fixture 的类型可见性，使 Win 节点正式验证能够进入并执行目标测试，而不是在 `expected.rs` 的未解析类型导入阶段失败。

## 验收标准

1. `expected.rs` 显式从相邻 `model` 模块导入其构造 frozen inventory 所需的八种枚举类型。
2. 不改变 A2b2 已冻结的 117 个静态 case key、动态证据计数或任何生产 VFS 能力边界。
3. `elon-pc-node` 定向 Rust 编译与测试不再出现这些 `E0432` 错误。
4. Win 节点发布合同验证通过；不修改 PWA、用户任务、凭据或生产插件准入状态。

## 非目标

- 不宣告 A2b2 动态验收完成。
- 不开放生产 VFS、Runtime、Ready 或外部任务派发。
- 不修改 frozen inventory 内容或数量。

## 预计实现范围

- `server/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b2_cases/expected.rs`
