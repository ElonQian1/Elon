# Rust 版本化历史 target 安全回收 V1

## 目标

让缓存平台能够对 AI 历史任务创建的 `target-v<数字>-<名称>` 目录执行既有
`register-legacy` 与 `purge-legacy` 流程，同时保持默认 dry-run 和严格失败关闭。

## 范围

1. 仅新增 `target-v[0-9]+-[a-z0-9._-]+` 叶目录名称。
2. 该名称还必须同时包含 Cargo `.rustc_info.json` 和 `CACHEDIR.TAG` 文件标记。
3. 继续要求绝对路径、非磁盘根、非托管 cache root、非重解析点、已登记且 retired。
4. `-Apply` 继续在任何 cargo/rustc 进程活动时拒绝执行。
5. 默认 dry-run 只统计并报告，不删除文件。

## 非目标

- 不允许任意 `target-*`、任务临时目录或源码目录。
- 不自动扫描、登记、迁移或删除外部缓存。
- 不改变托管 `rust-cache-v2` GC。
- 不在本批次执行真实回收。

## 验收标准

1. 已登记 retired 且带双 Cargo 标记的版本化 target 可以生成 dry-run 报告。
2. 缺任一标记的同名目录失败关闭。
3. 未登记、未 retired、重解析点和宽泛路径继续拒绝。
4. 原有 `target`、`*-target` 和 `sccache` 合同保持兼容。
