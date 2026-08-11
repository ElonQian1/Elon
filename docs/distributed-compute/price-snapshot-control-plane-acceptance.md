---
title: Price Snapshot 控制面验收证据
status: current
reviewed_at: 2026-08-11
owners: backend, ai-economy
implementation_status: implementation_partially_verified
---

# Price Snapshot 控制面验收证据

## 1. 验收结论

V5 Price Snapshot 已有服务、Store、HTTP/MCP 和 PC 实现，本轮没有创建第二套报价模型。临时 SQLite 可执行当前全量迁移，既有 Store/Service 可从精确 active Offer 发布、幂等重放、读取并列出不可变 `fallback_curve` 报价；Offer 进入 draining 后不能再发布新快照。

状态提升为 `implementation_partially_verified`。该结论不代表真实价格源、消费者候选 HTTP/MCP、容量预留、资金冻结或成交已经验证。

## 2. 服务端证据

2026-08-11 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_price_snapshot_service --locked
```

结果：2 项测试通过，验证指纹为 `f646ec343091ec21de3e710c57964fccaad26f6fabe9070d289c6f4bfaafa5b3`。覆盖：

- 来源固定为 `fallback_curve`，版本绑定 active Offer，样本数为 0；
- Snapshot 失效时间不晚于 Offer 合同，发布不预留容量、不冻结资金；
- 同一幂等键和同一合同精确重放，金额变化失败关闭；
- 陈旧 Offer 摘要失败关闭，draining Offer 不能发布新快照；
- 单份读取和列表读取都会重新审计不可变摘要与 Offer 历史版本。

共享 Offer 测试支持层重构后，原 Offer 2 项测试以指纹 `913ac08f114d962a6d77e5de96ea9711ac1f08a2db75b35b82d35a3e03e42eca` 再次通过。

## 3. PC 静态证据

包含 `/compute-supply` Offer 和 Price Snapshot 面板的 PC 前端已通过 `npm ci`、严格 TypeScript、ESLint、生产构建和 bundle budget。该证据只说明源码可静态生产构建，不证明真实接口、浏览器交互、视觉验收或发布。

## 4. 尚未验证或实现

- HTTP/MCP 真实调用、Bearer 权限、跨用户隔离和并发幂等；
- 生产磁盘迁移、进程重启、真实 TCP 和浏览器操作；
- Job 候选发现、预算匹配、Broker 预授权和 Reservation 组合链；
- 平台签名价格源、指数/成交价、期货曲线、批量刷新和到期调度；
- 多币种、外部市场、Sui 和真实资金清算。

后续必须复用当前 Price Snapshot Registry 和服务，不得以“真实价格源”名义新建平行快照权威。
