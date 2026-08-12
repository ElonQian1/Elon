---
title: 外部矿池 Adapter Provider-neutral Registry 验收
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
---

# 外部矿池 Adapter Provider-neutral Registry 验收

## V249 当前证据

V249 已进入源码交付阶段，并遵守架构铺设期约束：没有执行 Cargo 编译、Rust 测试、SQLite migration、HTTP 服务或真实 registry binding。当前运行验收为 `passed=0`，状态只能记为 `implementation_uncompiled`；源码测试、静态检索和 schema 审计都不能写成真实通过。

源码合同覆盖：

- 真实 V247 current installation 与同一份重新打开、逐文件复算并保活的 Prepared 文件树能力；
- 单个 `BEGIN IMMEDIATE` 内的唯一 UTC 纳秒检查时间、neutral release 创建/精确复用、installation companion 创建/精确重放；
- `adapter_id + release_version` 全局唯一，Provider-neutral bytes/capability/verifier 材料漂移失败关闭；
- 同一 neutral release 可由不同 Provider installation 各自形成 companion，companion 互不替代；
- 追加式 receipt、禁止 update/delete/replace、JSON/标量投影、exact roots 与 current view；
- 管理 HTTP 的 `401/403/422/400/404/409/201/200`、显式确认、actor 注入拒绝、幂等重放与响应脱敏；
- 创建必须消费完整 V247 current authority；创建后仍要求 installation/adoption explicit terminal 缺席、neutral admission/package current、Provider exact `registering` 与文件树 exact；
- exact idempotency replay 重新审计文件树和不可变历史材料，但不会在短时上游自然到期后伪造新的 companion 或改写旧回执；
- 创建后的 V239/V243 纯时间到期不撤销 registry companion，但 future route 必须另取 fresh V243 与可续签 security re-attestation；V239 当前不可刷新，因此 route gate 保持阻断；
- Provider 保持 `registering`，v213 Adapter/credential/service actor/route/outbox 及 Offer/Job 均无新增效果。

## 解除架构约束后必须运行

至少执行：

```powershell
cargo test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_registry --no-fail-fast
cargo test --manifest-path server/Cargo.toml --bin elon-server adapter_registry_http_test --no-fail-fast
```

运行验收必须覆盖 fresh database、V248→V249 升级、迁移重放与两次重开；fresh neutral+companion、exact replay、同 release 跨 Provider neutral 复用与 companion 独立；全局键下 bytes/capability/verifier 漂移；两个连接并发；数据库提交失败与响应丢失重试；安装树文件缺失/额外/内容或句柄漂移；创建前 V239/V243 到期失败关闭、创建后纯时间到期保持 registry current、V247 terminal/V244 explicit terminal/release admission/package terminal 或 Provider revision 漂移后失败关闭；canonical `checked_at` 边界；SQL update/delete/replace 与投影漂移；HTTP 状态矩阵和所有敏感字段脱敏。

## 明确未验证

- Cargo 编译、测试源码实际执行、migration 实际执行、生产数据库原位升级和生产部署；
- Windows/Unix 恶意文件系统竞争、崩溃、断电、磁盘写满、备份恢复与并发压力；
- 可续签 security re-attestation；V239 当前不可刷新，因此 companion 可保持 registry current，但后续 fresh route security gate 仍被阻断；
- Provider activation、service actor、credential resolver、v213 compatibility binding、route/seal/outbox；
- Adapter/Sidecar 启动、外部矿池网络、ACK/event、Runner、任务派发、可信计量、市场和结算。

因此本批只能表述为“Provider-neutral registry 与 installation companion 权威源码已写入，运行证据为零”，不能表述为“Adapter 已注册可用”“Provider 已激活”“route 已可派发”或“外部算力已可接单”。
