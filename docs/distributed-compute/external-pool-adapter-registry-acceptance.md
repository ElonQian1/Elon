---
title: 外部矿池 Adapter Provider-neutral Registry 验收
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Provider-neutral Registry 验收

## V249/V251 当前证据

V249 已完成服务端编译、SQLite migration 与本地 HTTP 定向验收；V251 修复了 V249 release trigger 对凭据验证器 JSON 做原始字符串比较、因合法键序差异拒绝生产写入的问题。V251 会在原子事务中替换已应用数据库中的旧 trigger，新数据库也直接安装逐字段语义校验版本。当前状态为 `implementation_partially_verified`，不等于生产部署、真实 Provider 激活或外部矿池接单。独立的 V250 漏洞情报 re-attestation 不属于本页行为通过证据；本次完整服务端测试目标编译与全新 `Store::open` 仅将其提升为 `implementation_compiled/migration_smoke_passed`，专属行为仍为 `passed=0`。

已运行：

```powershell
cargo test --manifest-path server/Cargo.toml --bin elon-server adapter_registry_http_test --locked --offline
cargo test --manifest-path server/Cargo.toml --bin elon-server store_migrations::compute_external_pool_adapter_registry::tests:: --locked --offline
```

- HTTP：`4 passed`，覆盖鉴权、显式确认、fresh/replay、文件漂移、同一 neutral release 的跨 Provider companion、安装撤销后的 fail-closed currentness 和无激活副作用；
- migration：`6 passed`，覆盖 V249 重放/重开、不可变与 exact-root trigger、两 Provider 独立 companion、terminal/current view，以及已应用 V249 到 V251 的原位修复与重复执行；
- 合并后受管正式筛选回归 `adapter_registry` 为 `10 passed`，验证指纹为 `f08db4e7be78f34f62b917b7a64a4e9a55e5615432d7cbbdb37822c6598391e0`；
- 编译期间发现并修复测试 support 子模块可见性、超大 `json!` 宏递归和 HTTP 测试连接生命周期问题；这些问题此前使“有测试源码”不能等同于“测试可运行”。

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

## 后续仍需运行

至少执行：

```powershell
cargo test --manifest-path server/Cargo.toml --bin elon-server adapter_registry --no-fail-fast
```

运行验收必须覆盖 fresh database、V248→V249 升级、迁移重放与两次重开；fresh neutral+companion、exact replay、同 release 跨 Provider neutral 复用与 companion 独立；全局键下 bytes/capability/verifier 漂移；两个连接并发；数据库提交失败与响应丢失重试；安装树文件缺失/额外/内容或句柄漂移；创建前 V239/V243 到期失败关闭、创建后纯时间到期保持 registry current、V247 terminal/V244 explicit terminal/release admission/package terminal 或 Provider revision 漂移后失败关闭；canonical `checked_at` 边界；SQL update/delete/replace 与投影漂移；HTTP 状态矩阵和所有敏感字段脱敏。

## 明确未验证

- 生产数据库原位升级、生产部署、数据库提交失败/响应丢失恢复和两个连接的竞争压力；
- Windows/Unix 恶意文件系统竞争、崩溃、断电、磁盘写满、备份恢复与并发压力；
- 可续签 security re-attestation；V239 当前不可刷新，因此 companion 可保持 registry current，但后续 fresh route security gate 仍被阻断；
- Provider activation、service actor、credential resolver、v213 compatibility binding、route/seal/outbox；
- Adapter/Sidecar 启动、外部矿池网络、ACK/event、Runner、任务派发、可信计量、市场和结算。

因此本批只能表述为“Provider-neutral registry 与 installation companion 已通过本地定向运行验收”，不能表述为“Adapter 已注册可用”“Provider 已激活”“route 已可派发”或“外部算力已可接单”。
