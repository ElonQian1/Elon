---
title: AI 任务外部 Rust scratch 生命周期 V1
status: accepted
reviewed_at: 2026-08-15
owners: developer-platform, operations
priority: p1
---

# AI 任务外部 Rust scratch 生命周期 V1

## 问题

一龙已经用 `rust-cache-v2` 统一日常 Cargo 构建，但历史 AI 任务仍可能为了隔离测试，
在 Windows `%TEMP%` 下手工创建任务专属 Rust cache root、sccache 或 target。任务正常结束后，
这些目录没有与不可变 `TaskContract` 绑定，也不属于仓库内 `.ai-tmp/`，统一收尾无法证明归属，
只能长期保留或等待人工清理。单个目录可达到数 GiB，持续任务会重复消耗系统盘。

## 决定

1. 任务确需独立 Rust scratch 时，必须通过项目入口在固定机器级根
   `%TEMP%/elon-ai-task-rust-v1` 下分配，不再手工拼接任务名、功能号或随机 target 路径。
2. 每个 scratch 精确绑定已验证的 `TaskContract`、工作树、分支和随机 nonce；完整本机路径只存
   在本机标记，并由分配命令作为当前终端的环境赋值返回，不写 Git、项目文档或正常收尾日志。
3. 分配入口同时返回独立 `cache` 与 `target` 子目录，供 `ELON_RUST_CACHE_ROOT`、
   `CARGO_TARGET_DIR` 或测试夹具显式使用；普通验证仍优先使用现有 D 盘受管共享缓存。
4. `finish-ai-task.ps1` 只有在验证同一个不可变合同后，才可回收该合同登记的 scratch。
   删除前重新验证固定根、精确目录、标记、工作树、合同、重解析点和目录结构；任一漂移失败关闭。
5. 未带当前合同标记、位于固定根外、属于其他合同或来源不明的目录绝不自动删除，继续由
   `inspect-node-disk-usage.ps1` 和 owner 决策处理。
6. `-SkipArtifactCleanup` 明确保留任务 scratch；普通成功收尾必须回收并输出结构化数量和路径摘要。
7. 该能力不迁移、不删除当前已经存在的历史缓存，也不改变 Cargo registry、共享
   `rust-cache-v2`、sccache 或发布 target 的现有策略。

## 验收条件

1. 有效合同可创建一个唯一 scratch，并返回 cache/target 绝对路径；相同 purpose 不覆盖旧目录。
2. 错误合同、错误工作树、空白或超长 purpose、固定根越界及已有目录全部失败关闭。
3. 精确合同收尾只删除本合同 scratch，保留其他合同、未知相邻目录和固定根本身。
4. 标记被删除、字段被篡改、路径变成重解析点或出现未知顶层成员时拒绝删除并阻止伪造完成。
5. 统一收尾测试证明 scratch 在任务完成前存在、`FINALIZABLE=true` 前已删除，且另一任务目录未受影响。
6. 预检、收尾、Prompt 审计、源码体积和文档模块化门禁通过。

## 非目标

- 不自动删除历史 `%TEMP%` Rust 缓存；
- 不为第三方工具或其他项目认领目录；
- 不在任务运行期间按时间或磁盘水位删除活动 scratch；
- 不以清理缓存替代功能测试、构建或发布。
