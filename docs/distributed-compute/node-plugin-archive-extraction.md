---
title: 节点插件候选归档受控解包边界
status: current
implementation_status: implementation_uncompiled
reviewed_at: 2026-08-05
owners: node, compute
---

# 节点插件候选归档受控解包边界

## 1. 目标与状态

本能力把已经完成候选闭包验证的 ZIP 包解压到该候选专属的随机 staging run，并形成逐文件证据和持续持有的文件句柄。它只关闭“已验证原始包如何落为受控文件”这一段，不把结果提升为 `staged`、`installed`、`ready`、可 promotion 或可结算状态。

当前状态为 `implementation_uncompiled`：源码已经写入，尚未编译、运行归档样例、接入本地权威 Store、跨重启恢复或 Host 启动流程。

## 2. 唯一输入边界

解包入口不接收普通文件路径或任意 `Read`。调用方必须交出：

- 已完成 exact closure、全文件 pin、完整 SHA-256 和原子 resolution 的 `VerifiedComputePluginCandidateArtifactSet`；
- 与该候选签名 Manifest 对应的 `ValidatedComputePluginManifest`；
- 精确 package item ordinal；
- 绑定同一安装身份和数据根的 `PinnedComputePluginRoot`。

安装身份不一致时在创建任何 staging 目录前失败。原始候选保管权被线性消费；成功后由解包结果继续持有，失败后由失败对象返还，不能从普通 DTO 重新构造。

## 3. ZIP 扫描与计划闭包

当前只接受 Manifest 明确声明的 `application/zip` 与 `zip`，压缩方法仅允许 Stored 或 Deflated。扫描器使用已验证 package 的同一只读句柄，拒绝：

- 加密条目、符号链接和特殊文件；
- ZIP 前缀数据与重叠文件数据；
- 非 UTF-8、名称歧义、重复路径和大小不一致；
- Manifest 外文件、Manifest 外目录、缺失文件；
- 非规范相对路径、Windows 保留名、大小写碰撞和文件/目录碰撞；
- 超过条目数、文件数、目录数或解包总字节上限的归档。

扫描结果先进入规范 extraction plan，并以 JCS/SHA-256 固定 release、签名发布者、package、文件路径、预期摘要、大小和目录闭包。扫描不写磁盘，也不证明解包已经完成。

## 4. 隔离 staging 与受管写入

每次尝试使用候选 token、计划摘要和本机随机数派生新的 staging run 摘要，在已钉住根下以 create-new 语义创建：

`compute-plugin/candidates/{candidate}/staging/{staging_run}`

目录和文件路径只能来自已验证计划。路径必须由普通相对分量组成；绝对路径、父目录、反斜杠、空分量和隐式创建计划外父目录都会失败。最终文件只在已经创建并重新钉住的父目录中 create-new，不能覆盖旧文件。

解包时再次打开同一已验证 package 句柄并重新校验 ZIP 结构，逐条对照原计划。每个文件采用不超过 64 KiB 的缓冲区流式写入，并在每次读写前检查原候选取消门卫；完成后要求精确 EOF、精确长度、SHA-256 与 Manifest 一致，再执行 flush、`sync_all` 和同句柄文件身份/类型/卷/长度复验。

## 5. 成功证据与保管权

成功结果继续持有：

- 原始 verified artifact-set；
- staging run 根目录和全部计划目录句柄；
- 按 Manifest 顺序排列的全部输出文件句柄；
- extraction plan；
- 完成单调时间点；
- 绑定安装身份、根身份、候选 token、staging run、计划摘要、逐文件摘要/大小/文件身份的 JCS/SHA-256 证据。

这些句柄只证明本进程仍掌握本次输出，证据尚未进入耐久 Store。目录自身也没有独立的跨重启耐久回执，因此文档不得把该结果称为“已耐久 staged”。

## 6. 失败语义

失败对象返回错误、原 verified 保管权、可选 staging run 摘要和 `filesystem_mutated`。一旦目录创建或文件写入已经发生，失败必须报告磁盘可能已变更；当前不会自动删除失败目录，也不会沿用旧 run 重试。

这使后续恢复器能够依据 Store 和目录证据决定隔离或清理，而不是在异常后盲目重开路径、覆盖旧输出或假定磁盘未变。

## 7. 后续必须完成

- 在本地权威 Store 中原子登记 extraction plan、输出证据、staging run 和精确库存 revision；
- 定义 Store commit 成功、失败和结果不确定时的线性恢复保管权；
- 启动时识别完整、失败、孤立和未知 staging run，并提供安全隔离/清理策略；
- 把可执行位、平台目标和 Runner 入口复核纳入 staged 事务；
- 在 staged 之后实现 Sidecar 沙箱、健康探针、promotion、回滚和 GC；
- 增加受控 ZIP 样例、恶意归档和崩溃恢复验证；
- 接入真实 HTTPS downloader、socket-read 取消、NodeAgent 启动与 Host 生命周期。

在这些工作完成前，本能力不能证明插件已经安装、可执行、健康、可被消费者 AI 调度或可以产生算力收入。

## 8. 实现入口

- `server/src/node_agent_compute_plugin_host/candidate_extraction.rs`
- `server/src/node_agent_compute_plugin_host/candidate_extraction/zip/scan.rs`
- `server/src/node_agent_compute_plugin_host/candidate_extraction/zip/extract.rs`
- `server/src/node_agent_compute_plugin_host/candidate_extraction/zip/types.rs`
- `server/src/node_agent_compute_plugin_host/fetch_file/staging.rs`
- `server/src/node_agent_managed_fs/copy.rs`
