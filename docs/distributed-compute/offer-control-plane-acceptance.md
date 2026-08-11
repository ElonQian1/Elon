---
title: Offer 草稿、发布与生命周期控制面验收证据
status: current
reviewed_at: 2026-08-11
owners: backend, ai-economy
implementation_status: implementation_partially_verified
---

# Offer 草稿、发布与生命周期控制面验收证据

## 1. 验收结论

V5 Offer 已有生产实现，不需要平行重写。当前已在既有 Store/Service 上完成本人 HTTP/MCP、平台管理员 HTTP/MCP 和磁盘重开验收：商户可创建、读取、修订或撤销 draft；管理员可读取待审合同，发布 active Offer，安全排空，并受控进入 expired/revoked 终态。

状态为 `implementation_partially_verified`。该结论只覆盖进程内接口和临时文件 Store，不代表真实 TCP、浏览器、并发压力、生产数据库、真实节点、撮合、容量预留或资金路径已经验证。

## 2. 服务端证据

2026-08-11 执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_offer_service --locked
```

结果：通过，验证指纹为 `815154ea47c36f33ec1ad4b80023b7b616943953a579d44f6778b5688d6e43a6`。定向过滤运行 2 项 Store/Service 测试：

- 所有者只能按当前版本和摘要撤销自己的 draft，陈旧摘要失败关闭；
- `draft v1 -> 幂等重放 -> draft v2 -> active v3 -> draining v4 -> revoked v5` 保存连续历史和不可变回执；
- 未到 `valid_until` 时请求 `expired` 被拒绝，提前退出只能使用 `revoked`；
- 发布、排空和终止响应均证明不生成 Price Snapshot、不改变既有 Reservation、不派发 Attempt、不移动资金。

首次运行暴露的是测试夹具错误：新 Provider 被直接登记为 active，违反既有 `registering v1` 门卫。夹具改为 `registering v1 -> 建池和供给 -> active v2 -> Pool active` 后通过；生产门卫未被放宽。

2026-08-11 又执行整个算力 MCP 聚合器回归：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-offer-interface -- test --manifest-path server/Cargo.toml --bin elon-server compute_federation_mcp:: -- --nocapture
```

10 项全部通过，其中 Offer 专项覆盖：

- 普通用户不可发现或直调 10 个管理员 Offer MCP 工具；
- 未登录和普通用户不能访问管理员 HTTP，`admin/owner` 可读取待审队列；
- owner MCP 创建 draft，管理员 MCP 发布并精确重放；
- owner HTTP 读取发布回执，管理员 HTTP 执行 `active -> draining`；
- 管理员 MCP 执行 `draining -> revoked`，owner HTTP 读取同一终态回执；
- 关闭并重开文件 Store 后，Offer 当前投影、发布和终态回执保持一致。

验证指纹：`ced6ab0c014ec1a65d0e7fe4ea054104fe747f3c2b4fe371c9a50a968900ec1a`；验证回执：`0125d89d9469e61070d30fad7374b72675b2706d5ac77d8b9f4db18236e69ef1`。

## 3. PC 静态证据

同日，包含 `/compute-supply` Offer 草稿区和 `/compute-offers` 管理区的 PC 前端已通过 `npm ci`、严格 TypeScript、ESLint、生产构建和 bundle budget。该证据仅说明源码可静态生产构建，不证明真实 HTTP、浏览器交互、权限行为或视觉验收。

## 4. 尚未验证

- 相同幂等键及版本摘要的并发竞争；
- 生产数据库副本升级、异常断电、真实 TCP 和浏览器操作；
- 有活动 Reservation 时的终态失败路径和到期自动调度；
- Price Snapshot、Broker、Attempt、结算、外部矿池、Sui 或真实付款。

以上缺口必须沿既有 Offer 链继续验收，不得另建第二套 Offer、发布或生命周期模型。
