---
title: 外部矿池 Adapter supervisor/session policy companion 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_rust_sqlite_axum
---

# 外部矿池 Adapter supervisor/session policy companion 验收边界

## 本批状态

V259 durable inert Domain、migration、Store、owner/admin Service/API 与源码合同已随完整 `elon-server` 测试目标编译，并执行 13 项 Windows 本地定向验收：6 项 migration/Store 合同、5 项源码边界合同和 2 项 owner/admin 进程内 Axum HTTP，结果为 `13 passed / 0 failed`。验证指纹为 `af27aff6c90e44409ee3da8d7fbc5a32dd2910766a962c2cbc1cc08ab5eda17f`，receipt 为 `D:\rust\shared\rust-cache-v2\validation-v1\receipts\b581810545819c43bbb9a1c74b0ee01d95b9f7a3b56d713d98b033dbedbd799c.json`。

首次动态执行发现源码合同错误地要求 Service 文件包含 API 层的完整 admin actor 构造字符串；合同已改为分别验证 Service 枚举/分支和 API 身份映射，运行时授权没有放宽。上述测试没有执行 process/syscall、capsule exec、namespace/cgroup/seccomp/rlimit/pidfd、IPC/socketpair/session、secret 读取或交付、DNS/TLS/network、probe/ACK/runtime identity、route/service actor、Provider activation、market、usage或 settlement。

## 已运行本地矩阵

- migration/Store 合同冻结表列、注册 receipt、server policy catalog、canonical JCS/digest、roots/lineage/time、current inert view、append-only 和 V254 18 项 deny；
- owner HTTP 覆盖 server-fixed policy、fresh create、actor-bound replay、currentness、revoke、路径/owner 边界、响应脱敏和零下游业务表写入；
- admin HTTP 覆盖平台角色、线性 recovery successor、fresh/exact replay revoke、已撤销 latest 恢复及惰性状态；
- 源码合同冻结 owner/admin 路由与 Store ABI、fd/seccomp catalog、递归脱敏、九项 effect=`none`、七项 readiness=false，并拒绝 runtime、network、secret、probe 与市场 consumer。

## 仍待运行或扩大验证

- V258→V259 文件升级、重复 migration、文件重开、历史库迁移、真实并发 create/revoke、崩溃恢复、真实 TCP 和生产数据库；
- 更完整的 401/403/400/404/409/422、malformed JSON、nested unknown field、全部 digest/policy/Prepared 漂移与直接 SQL 篡改动态矩阵；
- Linux x86-64 上的真实 fd 拓扑、sealed capsule、namespace/cgroup/seccomp/rlimit/pidfd/shutdown/reap fixture，以及 ordered syscall catalog 是否足以启动受支持 static ELF；
- future Store-private authority 在同一 Immediate 事务/checked_at 组合 V259/V258/V255、V257 capsule、V256 locked bundle 和 V250/V252/V253 TTL roots 的动态证明。

## 源码合同与未来 seam

Service/API consumer源码扫描必须拒绝 `std::process::Command`、`tokio::process`、fork/exec/clone调用、namespace/cgroup/seccomp/rlimit/prctl/pidfd enforcement、socket/TCP/DNS/TLS、runtime bundle sensitive-byte consumer、probe、route与activation调用；Domain declarative catalog中的协议/syscall名称由另一项exact policy合同逐字冻结，不能误判为执行证据。migration源码合同必须冻结完整 receipt/policy投影、roots/lineage/timestamp/immutability，并校验V254 18 deny source SHA-256与trigger names exact parity。

future Store-private seam须证明同一Immediate事务与checked_at组合 current V259/V258/V255、V257 capsule、V256 locked bundle及V250/V252/V253 TTL roots，且authority不可Clone/Debug/Serde、raw endpoint与secret不越界。本批不实现或调用该 consumer。

因此本批只能记录 `verified_rust_sqlite_axum / 13 passed / 0 failed`；未运行 confinement fixture，ordered syscall catalog 尚未证明足以启动真实 static ELF。不得宣称 supervisor、authenticated session、secret-safe delivery、Linux isolation、broker transport、probe、runtime readiness或 production Adapter 已验收。
