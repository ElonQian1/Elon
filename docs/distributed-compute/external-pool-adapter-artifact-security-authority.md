---
title: 外部矿池 Adapter Artifact 静态安全证明权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Artifact 静态安全证明权威

## 1. 冻结结论

V233 在 V232 静态包格式证明之后增加确定性的 SBOM、许可证声明和本地静态安全门禁。它重新消费同一个已复核的 CAS 文件句柄，不执行入口、不解压到文件系统、不访问网络，也不授予 Adapter 安装、运行或路由权限。

V233 证明的是：精确制品包含平台规定的 canonical SBOM；每个非 SBOM 文件恰好归属一个组件；声明的许可证符合首版允许策略；所有文件在扫描时仍与 V232 manifest 的长度和 SHA-256 一致；固定本地规则未发现嵌入私钥、已知访问令牌前缀或嵌套 ZIP。

它只授予 `artifact_security_effect=static_policy_verified`。`vulnerability_intelligence_effect`、`conformance_effect`、`adapter_effect` 和 `route_effect` 均保持 `none`。

## 2. 固定 SBOM 合同

首版要求包内存在 `elon-adapter-sbom.json`，并由 V232 manifest 以 `resource` 角色声明。SBOM 合同为：

- schema 固定为 `compute_federation.external_pool_adapter_sbom.v1`；
- 原始字节必须是 RFC 8785 JCS canonical JSON，最大 256 KiB；
- Adapter ID 与 release version 必须和 V232 manifest 精确一致；
- 组件数量为 1 至 128，按 `component_id` 严格递增；
- 每个组件声明名称、版本、供应方、`pkg:` package URL、单一 SPDX 许可证标识及文件路径；
- 每个非 SBOM 文件必须且只能属于一个组件，不允许遗漏、重复或额外路径；
- 首版允许 `Apache-2.0`、`BSD-2-Clause`、`BSD-3-Clause`、`ISC`、`MIT`、`MPL-2.0`、`Unicode-3.0` 与 `Zlib`。

这是声明与结构门禁，不代表依赖真实无漏洞，也不把 package URL、许可证明细或文件路径公开给 API 调用者。

## 3. 固定本地规则

规则集 ID 固定为 `elon_adapter_static_safety_v1`，规则名称及顺序参与 domain-separated SHA-256 摘要：

1. 拒绝常见 PEM 私钥标记；
2. 拒绝 AWS `AKIA`、GitHub `ghp_` 和 `github_pat_` 等已知访问令牌前缀；
3. 拒绝非 SBOM 文件中的嵌套 ZIP 头；
4. 重新核对 manifest 中每个文件的长度和 SHA-256。

扫描规则摘要在检查、收据验证和数据库回读时均由固定规则表重算。该规则集是有意保持较小且可审计的 deterministic gate，不是杀毒引擎、沙箱或联网 CVE 扫描器。

## 4. 权威和 TOCTOU 边界

Service 先读取当前 V232 package authority，再重新打开并复核 exact CAS。V232 检查器返回的 non-Clone/non-Serde 句柄继续传入 V233，V233 完成扫描后仍保留同一个文件句柄。Store 在 `BEGIN IMMEDIATE` 中再次读取当前 V232 根，并精确核对 admission、source、provenance、package receipt、manifest、包检查摘要和 CAS 身份后才允许写入。

读取历史收据时，Store 会重新验证 canonical receipt、完整数据库投影、固定规则摘要、原始 canonical SBOM、SBOM 与历史 V232 manifest 的逐文件归属，以及所有上游历史根。路径替换、规则摘要伪造、SBOM 漂移或过期上游权威均不能形成新的 current 收据。

## 5. 不可变账本与 currentness

V233 新增：

- `compute_external_pool_adapter_artifact_security_receipts`：每个 admission/package 一份不可变 exact receipt；
- `compute_external_pool_adapter_artifact_security_current`：从 V232 currentness 派生 `verified_current|historical_only`。

数据库通过唯一约束、外键、不可更新/不可删除触发器、exact package 触发器和 JSON projection 触发器保护。上游 admission 终态或 signer 撤销后，V233 收据只保留为历史证据，不继续提供 current authority。

## 6. 管理 API 与脱敏

- `POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-security`；
- `GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-security`。

仅 `admin|owner` 可用。POST 必须提交 exact admission/source/provenance/package digest、幂等键和显式确认。响应只返回必要摘要、计数、currentness 和 effect，不返回原始 SBOM、package URL、许可证明细、文件路径、签名、公钥、幂等材料或服务器路径。

## 7. 后续硬门卫

V233 之后仍不得直接创建可运行 Adapter 或 v213 route。至少还需独立完成：

1. V235 已建立独立 scanner key 信任根；仍缺可信依赖解析、漏洞情报源、情报时间点与可复核的已签扫描证明；
2. 动态恶意行为检测及隔离沙箱中的六能力 conformance；
3. credential verifier 运行时 registry、currentness 与 revoke；
4. 安装根、运行身份、Sidecar IPC、健康探针、升级和回滚；
5. Adapter/version 采用、v213 route authorization、Worker/ACK 和真实派发；
6. 生产用量、结算、支付及真实外部矿池联调。

V233 完成的是供应链本地静态策略门，不是“第三方 Adapter 已安全运行”，也不是“项目总体已经完成”。

## 8. 当前验收状态

本地 3 项 V233 定向测试及 3 项 V232 回归通过，覆盖 migration 重复执行、有效 SBOM、首写与幂等、认证/角色、脱敏、历史化、许可证拒绝、文件归属缺口和嵌入私钥拒绝。证据见 [`external-pool-adapter-artifact-security-acceptance.md`](external-pool-adapter-artifact-security-acceptance.md)。

真实供应商 SBOM、依赖解析、联网漏洞情报、动态沙箱、fuzz、生产数据库升级、真实 TCP、部署、MCP/PC 及后续采用/派发/结算链仍未验证。
