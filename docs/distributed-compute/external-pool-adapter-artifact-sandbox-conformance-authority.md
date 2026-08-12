---
title: 外部矿池 Adapter 动态沙箱符合性证据权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 动态沙箱符合性证据权威

## 结论

V239 接受一份由当前 V237 独立沙箱验证者签名的、绑定精确 V236 制品安全链的动态符合性报告。服务器不信任调用方自报的能力列表，而是从不可变 V222 admission 派生六项测试计划：`authenticated_ack`、`authenticated_events`、`cancel_no_start`、`idempotent_commit`、`prepare`、`reconcile`。

V239 证明的是“当前受信验证者对精确制品、服务器派生测试计划和有限观察结果签过名，且签名仍在有效期内”。当前服务器不下载或执行制品，不证明验证器自身实现、虚拟化内核或观察来源真实，也不授予 credential、Adapter、v213 route、worker、派发、计量或结算权限。

## 精确绑定

每份签名 challenge 固定：

- V222 admission、Adapter ID、release version、implementation digest、六项能力 revision、capability-set digest 和预期 credential verifier；
- V236 vulnerability-report receipt，以及其 V233 security/package/archive/SBOM 摘要和漏洞情报到期时间；
- 当前 V237 verifier root 的 record ID/digest、key ID、operator、product；
- sandbox runtime ID、runtime image digest、固定隔离策略、运行窗口、报告窗口；
- 服务器按 admission 为每项能力派生的 test-case ID 和 fixture digest；
- 六条 observation、输出 transcript digest、耗时、策略违规计数和资源观察。

challenge 使用 RFC 8785 JCS、domain-separated SHA-256 和 RSA PKCS#1 v1.5 SHA-256。Store 在写入事务和每次历史读回时重建上游权威、重算测试计划与摘要，并重新验证签名。

## 通过策略

首版仅接受以下有限结论：

- 六项能力各有一条顺序和 revision 完全一致的 `passed` 观察；
- 每条 observation 必须绑定服务器派生的 test-case ID、非空 transcript digest、1 至 300000 ms 的耗时和零策略违规；
- 固定隔离 profile 为 `offline_readonly_ephemeral_no_child_process_v1`；
- 外部网络尝试、临时目录外写入、子进程尝试均为零；
- peak memory 为 1 至 512 MiB，CPU time 为 1 至 900000 ms；
- 单次运行不超过 30 分钟，报告最长 24 小时；
- 运行开始不得早于 V236 报告形成，V239 到期不得晚于上游漏洞情报到期。

任何失败、缺项、乱序、能力漂移、策略违规、超限、过期或签名错误都失败关闭。首版不接受“部分通过”或管理员豁免。

## 数据与 API

V239 新增 append-only receipt 表和派生 current view。仅 `admin|owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/sandbox-conformance/challenge`；
- `POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/sandbox-conformance`；
- `GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/sandbox-conformance`。

响应只返回摘要，不返回签名原文、test plan、observations、credential verifier 或幂等材料。每个 admission 只有一份不可覆盖报告；相同幂等输入精确重放，冲突输入失败关闭。

后续 V240 增量为已发布的 V239 表补充显式 `INSERT OR REPLACE` 冲突门卫和 receipt ID 非空门卫；Store 同时新增不可克隆、不可序列化的同连接 current authority，供后续采用事务消费 `verified_current` 且摘要精确匹配的收据。当前尚无采用事务调用该 helper；该增量不会创建 Adapter，也不会把 current authority 暴露为 HTTP DTO。

## 当前性与失效

只有以下条件同时满足，current view 才返回 `verified_current`：

1. 精确 V236 报告仍为 `verified_current`；
2. 精确 V237 verifier key 仍为 `active`；
3. V239 报告尚未到期。

上游 admission/security/scanner 撤销、漏洞情报到期、sandbox verifier 撤销或本报告到期，都会使证据动态变为 `historical_only`。历史收据和签名仍可审计，但不能作为后续当前权威。

## 明确无效果

每份 V239 收据固定：

- `conformance_effect=signed_sandbox_report_verified_current`；
- `credential_effect=none`；
- `adapter_effect=none`；
- `route_effect=none`。

这意味着 V239 只完成动态符合性证据门，不创建可执行 Adapter。下一阶段仍须独立完成 credential-verifier registry/runtime、Adapter adoption/install authority、受限 Sidecar IPC、v213 route authority、worker/ACK 和真实外部矿池验收。

验收证据见 [`external-pool-adapter-artifact-sandbox-conformance-acceptance.md`](external-pool-adapter-artifact-sandbox-conformance-acceptance.md)。
