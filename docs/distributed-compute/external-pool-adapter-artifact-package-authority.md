---
title: 外部矿池 Adapter Artifact 静态包格式证明权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 静态包格式证明权威

## 1. 冻结结论

v232 在 v222 staged admission、v227 quarantine bytes 与 v231 current signed provenance 之后，增加一层有界、无执行、无落盘解压的静态 ZIP/manifest 检查。它证明 exact CAS 字节采用平台固定包格式，且 manifest 中的 Adapter、版本、六项能力、能力集合摘要、credential verifier 意图、入口文件和完整文件清单与既有权威精确一致。

该收据只授予 `artifact_format_effect=static_format_verified`。它不授予安全性、SBOM 完整性、恶意代码扫描、协议 conformance、credential verifier 可用性、Adapter 安装/运行、v213 route、Worker、Attempt、用量或结算权限。

## 2. 固定包合同

首版只接受：

- ZIP 容器，且 archive/file comment 为空；
- 根目录唯一 `elon-adapter-manifest.json`；
- manifest 为 canonical RFC 8785 JCS JSON，schema 固定为 `compute_federation.external_pool_adapter_artifact_manifest.v1`；
- runtime 固定为 `server_sidecar_v1`，恰好一个 `entrypoint`，其他条目只能是 `resource`；
- manifest 文件列表按路径严格递增，并与 ZIP 中非 manifest 文件逐项等量、等路径、等长度、等 SHA-256；
- ZIP 最多 128 个业务条目，manifest 最大 64 KiB，单条目最大 32 MiB，总解压大小最大 64 MiB；
- 只接受 Stored/Deflated 普通文件，不接受目录、符号链接、加密条目、绝对路径、反斜杠、`.`/`..`、重复路径或大小写冲突路径；
- 单条目解压比例必须不超过 `compressed_size * 200 + 1 MiB`。

整个检查在内存中有界读取，不把 ZIP 解压到文件系统，不运行入口，不打开网络。manifest 不声明整个 ZIP 摘要，避免自引用摘要；archive 身份继续由 v227/v231 的 exact SHA-256 与长度提供。

## 3. 权威和 TOCTOU 边界

Service 先从 Store 读取 point-in-time inspection target，再重新打开 CAS 文件、完整复算 SHA-256/长度并取得不暴露路径的文件句柄。检查器消费该句柄并返回 non-Clone/non-Serde 的 `InspectedExternalPoolAdapterArtifactPackage`，其中继续持有同一个已验证文件句柄。

Store 在 `BEGIN IMMEDIATE` 中重新读取当前 v222 admission、v227 source 和 v231 signed provenance，再核对保留句柄、inspection 和 manifest。只有全部精确一致才写入收据。这样，检查后替换路径、旧 admission、旧 signer 或另一份 CAS 文件都不能成为 fresh write 权威。

## 4. 不可变账本与 currentness

v232 新增：

- `compute_external_pool_adapter_artifact_package_receipts`：每个 admission/provenance 一份不可变 exact receipt；
- `compute_external_pool_adapter_artifact_package_current`：从 v231 currentness 派生 `verified_current|historical_only`。

写入受 SQLite 不可更新/不可删除、重复替换、exact authority 和 JSON projection 触发器保护。读取时重新验证 canonical receipt、数据库投影及历史 v222/v227/v231 根。admission 终态或 signer key 撤销后，收据只转为历史证据；GET 仍要求 CAS 字节可安全重开并通过摘要与长度复核。

## 5. 管理 API 与脱敏

- `POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-package`；
- `GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-package`。

仅 `admin|owner` 可用。写请求必须带 exact admission/source/provenance digest、幂等键与显式确认。公开摘要不返回原始 manifest、入口路径、文件清单、完整 verifier 对象、签名、公钥、候选引用、幂等材料或本地路径；只返回对应摘要、计数、边界结果和明确的 `none` effects。

## 6. 后续硬门卫

后续不得直接从 V232 创建 Adapter 或路由。至少还需独立完成：

1. V233 已完成 exact SBOM、许可证允许策略及有限本地静态规则；可信依赖解析、漏洞情报和动态恶意行为检测仍未完成；
2. 隔离沙箱中的六能力 conformance；
3. credential verifier registry/currentness/revocation；
4. 安装根、运行身份、Sidecar IPC、健康探针和回滚；
5. 平台 release actor 与 Provider route actor 分权；
6. exact Adapter/version 采用和 v213 route authorization；
7. 真实派发、ACK、取消、恢复、计量和结算联调。

V232 完成的是供应链静态格式门，不是“外部矿池已接通”。

## 7. 当前验收状态

本地 3 项定向测试通过，覆盖 migration 重复执行、真实 RSA 来源链、有效 ZIP/manifest、首写与幂等重放、认证与角色隔离、输出脱敏、终态历史化、CAS 缺失失败关闭、路径穿越、manifest 身份漂移、大小写冲突和高压缩比炸弹拒绝。证据见 [`external-pool-adapter-artifact-package-acceptance.md`](external-pool-adapter-artifact-package-acceptance.md)。

真实生产包、第三方 ZIP 生成器兼容、超大边界压力、模糊测试、进程崩溃/断电、生产数据库升级、真实 TCP、部署、MCP/PC 和后续安全/conformance/采用链仍未验证。
