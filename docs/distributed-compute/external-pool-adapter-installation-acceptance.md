---
title: 外部矿池 Adapter 惰性安装与撤销验收
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 惰性安装与撤销验收

## V245-V247 当前证据

2026-08-13 已在 Windows 本地文件数据库与进程内 Axum 链路完成三组定向验收：

- V245 credential verification projection 加固迁移 3 项通过，验证迁移重放、完整签名字段投影和升级前伪造过期时间拒绝；证据指纹 `8f960662f5632c1062da4298170aa0fa4a97caa9ed36a12f5f161f64bd852189`。
- V246/V247 文件系统、安装回执与撤销终态迁移 10 项通过，验证真实 ZIP 物化、内容地址复用、字节漂移/缺失/hardlink 失败关闭、迁移重放、投影和追加式终态；证据指纹 `be60c2aa2cb8e4a2ac648ca9ba9ae4f680483e79337670cce623618bd8196d55`。
- 安装与撤销 HTTP 3 项通过，验证认证链路、显式确认、幂等重放、输入/字节漂移拒绝、撤销终态和撤销后惰性字节保留；证据指纹 `7f3d68b357d0b665a95e328e72efbee0f734f9d3550be8508bacb24cf576c0e4`。

首次运行发现 Windows `MoveFileExW` 在深层安装命名空间超过传统 `MAX_PATH` 后返回 `ERROR_PATH_NOT_FOUND`。发布路径现统一转换为绝对 Win32 扩展路径，保持原子、不覆盖发布语义；针对性长路径物化回归另以指纹 `25bd0cefb42b0025ba207287b03ae1be765ecea95c1c0aedd0b2a085cdf5db76` 通过。

已验证合同覆盖以下职责：

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

## 已执行命令

至少执行：

```powershell
cargo test --manifest-path server/Cargo.toml --bin elon-server compute_external_pool_adapter_credential_verification_hardening --no-fail-fast
cargo test --manifest-path server/Cargo.toml --bin elon-server external_pool_adapter_installation --no-fail-fast
cargo test --manifest-path server/Cargo.toml --bin elon-server Adapter_installation_ --no-fail-fast
```

这些命令通过项目 Rust 验证器执行，包含源码大小、Rust 格式、离线锁定依赖和持久证据回执。它们证明定向本地合同通过，不等于生产数据库、真实外部矿池或生产 Sidecar 已验收。

## 明确未验证

- 生产数据库原位升级、历史大库数据审计与回滚演练；
- Windows reparse/分享模式竞争、Linux symlink/hardlink 竞争和跨平台恶意文件系统矩阵；
- 崩溃注入、断电耐久、并发压力、备份恢复和磁盘权限/ACL；
- 真实 Adapter 启动、Sidecar 隔离、credential resolver/KMS/gateway、外部矿池网络、ACK/Runner；
- Provider activation 的同事务 actor/route/companion binding、v213 route、任务派发、可信计量、市场、结算、付款和生产部署。

因此本批可以表述为“惰性内容寻址安装与追加式撤销的本地定向链路已通过”，不能表述为“生产安装验收完成”“Adapter 已可启动”或“外部算力已可接单”。
