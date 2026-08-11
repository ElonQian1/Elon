---
title: Offer 草稿、发布与生命周期控制面验收证据
status: current
reviewed_at: 2026-08-11
owners: backend, ai-economy
implementation_status: implementation_partially_verified
---

# Offer 草稿、发布与生命周期控制面验收证据

## 1. 验收结论

V5 Offer 已有生产实现，不需要平行重写。本轮只补 Store/Service 状态链测试和状态校准：临时 SQLite 可执行当前全量迁移，既有服务可完成草稿创建、幂等重放、精确修订、管理员发布、安全排空和受控撤销，并可重新读取追加式 Offer 版本及发布、排空、终态回执。

状态提升为 `implementation_partially_verified`。该结论不代表 HTTP/MCP、并发竞争、生产磁盘、真实节点、撮合、容量预留或资金路径已经验证。

## 2. 服务端证据

2026-08-11 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_offer_service --locked
```

结果：通过，验证指纹为 `815154ea47c36f33ec1ad4b80023b7b616943953a579d44f6778b5688d6e43a6`。定向过滤运行 2 项测试：

- 所有者只能按当前版本和摘要撤销自己的 draft，陈旧摘要失败关闭；
- `draft v1 -> 幂等重放 -> draft v2 -> active v3 -> draining v4 -> revoked v5` 保存连续历史和不可变回执；
- 未到 `valid_until` 时请求 `expired` 被拒绝，提前退出只能使用 `revoked`；
- 发布、排空和终止响应均证明不生成 Price Snapshot、不改变既有 Reservation、不派发 Attempt、不移动资金。

首次运行暴露的是测试夹具错误：新 Provider 被直接登记为 active，违反既有 `registering v1` 门卫。夹具改为 `registering v1 -> 建池和供给 -> active v2 -> Pool active` 后通过；生产门卫未被放宽。

## 3. PC 静态证据

同日，包含 `/compute-supply` Offer 草稿区和 `/compute-offers` 管理区的 PC 前端已通过 `npm ci`、严格 TypeScript、ESLint、生产构建和 bundle budget。该证据仅说明源码可静态生产构建，不证明真实 HTTP、浏览器交互、权限行为或视觉验收。

## 4. 尚未验证

- HTTP/MCP 真实调用、Bearer/管理员角色和跨用户隔离；
- 相同幂等键及版本摘要的并发竞争；
- 生产磁盘迁移、进程重启、真实 TCP 和浏览器操作；
- 有活动 Reservation 时的终态失败路径和到期自动调度；
- Price Snapshot、Broker、Attempt、结算、外部矿池、Sui 或真实付款。

以上缺口必须沿既有 Offer 链继续验收，不得另建第二套 Offer、发布或生命周期模型。
