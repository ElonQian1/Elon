---
title: 外部矿池 Adapter 惰性安装与撤销验收
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
---

# 外部矿池 Adapter 惰性安装与撤销验收

## V246/V247 当前证据

V246/V247 已进入源码交付阶段，当前遵守架构铺设期约束：没有执行 Cargo 编译、Rust 测试、SQLite migration、HTTP 服务、真实安装或撤销。当前运行验收为 `passed=0`，状态只能记为 `implementation_uncompiled`，不得把源码测试或静态审计写成真实通过。

源码合同覆盖以下职责：

- V244 current authority 的显式 UTC 纳秒 `checked_at`，到期等号失败关闭；
- V244 adoption、V232 package、V227 source/CAS 的 exact lineage；
- 同一已复验 CAS 文件句柄上的 ZIP 读取、manifest exact set、逐文件 SHA-256/长度复算；
- staging、`create_new`、同步、不覆盖发布、已有树全量复验和数据库失败后的幂等恢复；
- V246 receipts/files 的追加式、no-replace、完整 JSON/标量投影与 current view；
- 管理 HTTP 的认证、显式确认、未知字段、幂等重放、摘要漂移、响应脱敏；
- 安装前后 Provider 仍为 `registering`，v213 route/credential/service actor/outbox 行数保持不变。
- V247 terminal receipt 的精确 installation ID/digest、认证 actor、原因、确认、canonical digest、追加式单终态与精确重放；
- terminal 缺席才可能得到 `installed_upstreams_current`，撤销后固定 `historical_only`/`revoked`，同时保留脱敏 terminal summary；
- 撤销不要求上游仍当前、不删除安装字节，也不写 Provider、v213 route/credential/service actor/outbox；
- sealed current authority 必须携带同一 `checked_at` 的回执与已复验文件树能力，SQL view/管理 GET 不能替代它。

## 解除架构约束后必须运行

至少执行：

```powershell
cargo test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_installation --no-fail-fast
cargo test --manifest-path server/Cargo.toml --bin elon-server adapter_installation_http_test --no-fail-fast
cargo test --manifest-path server/Cargo.toml --bin elon-server adapter_installation_revocation_http_test --no-fail-fast
```

运行验收必须覆盖：全新库与 V245→V246→V247 升级、迁移重放与重开、真实 ZIP happy path、路径穿越/大小写冲突/链接/加密/压缩膨胀、CAS 缺失或漂移、manifest 缺失/额外/摘要或长度漂移、目标目录精确复用与漂移拒绝、文件系统成功后数据库失败再试、同 adoption 并发、上游撤销/到期、`checked_at<installed_at` 与 `expiry-1ns/expiry/expiry+1ns`、terminal exact replay/changed replay/second terminal/no-replace、撤销后 byte retention 与 sealed-authority 拒绝，以及 HTTP 401/403/400/404/409/422/201/200 和响应脱敏。

## 明确未验证

- Cargo 编译、测试源码实际执行、迁移实际执行和生产数据库原位升级；
- Windows reparse/分享模式与 Linux symlink/hardlink 竞争的真实恶意文件系统矩阵；
- 崩溃注入、断电耐久、并发压力、备份恢复和磁盘权限/ACL；
- 真实 Adapter 启动、Sidecar 隔离、credential resolver/KMS/gateway、外部矿池网络、ACK/Runner；
- Provider activation 的同事务 actor/route/companion binding、v213 route、任务派发、可信计量、市场、结算、付款和生产部署。

因此本批只能表述为“惰性内容寻址安装与追加式撤销权威源码已写入，运行证据为零”，不能表述为“已完成真实安装/撤销验收”“Adapter 已可启动”或“外部算力已可接单”。
