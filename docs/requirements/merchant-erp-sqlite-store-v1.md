---
title: 通用 ERP SQLite 持久化适配器 V1
status: current
reviewed_at: 2026-08-16
owners: merchant-erp, open-commerce
implementation_status: implemented_locally_verified
---

# 通用 ERP SQLite 持久化适配器 V1

## 问题

`@yilong/merchant-erp-kernel` 已提供门店、商品、库存、采购、平衡分录、未付款订单、审计和消费者 AI 能力映射，但当前仓库内只有用于测试和本地开发的 `MemoryErpStore`。每个商户项目若都自行实现 SQLite 事务、迁移、幂等和恢复，会重复造轮子，也难以证明同一套 ERP 规则在进程重启后仍然成立。

## 目标

提供一个由内核包维护、商户持有数据库文件的 SQLite V1 适配器，使单机 Node 商户运行时可以直接复用通用 ERP 领域规则，并把 UI、主题和私有插件继续留在各自项目中。

## 合同

- SQLite 适配器通过独立 `./sqlite` 子路径导出；核心入口和 `MemoryErpStore` 继续兼容 Node 20，不因可选适配器提高整个包的最低版本。
- SQLite 子路径使用 Node 内置 `node:sqlite`，推荐 Node 22.13 或更高版本；22.5 至 22.12 需要显式 `--experimental-sqlite`，不增加原生第三方依赖。
- 适配器实现现有 `ErpStorageAdapter` 的 `read` 与 `transact` 边界，并提供显式 `close` 和只读 `snapshot`。
- 数据库迁移版本化、幂等执行；未知的新版本失败关闭，不能自动降级或覆盖。
- 写事务使用 SQLite 原子事务；业务回调失败时，库存、订单、采购、分录、幂等记录和审计全部回滚。
- 同一适配器内的操作串行化；不同 SQLite 连接竞争时服从数据库锁和 busy timeout，不以最后写入覆盖另一事务。
- 所有记录继续按 `merchant_id` 和 `store_id` 隔离。存储层不加入支付、履约、外部平台授权、云端同步或多副本共识。
- 初始化种子只允许写入空数据库；已有业务记录时拒绝再次播种，避免测试夹具覆盖商户数据。
- 数据库文件、目录、凭据和经营正文不进入一龙平台日志或功能注册表。

## 非目标

- 不迁移 `cofficethinking` 的现有生产数据库。
- 不提供 PostgreSQL、云数据库或浏览器端 SQLite 实现。
- 不声明多进程高可用、多主写入、支付完成或真实消费者交易已经上线。
- 不改变现有 ERP 领域规则、开放商业能力键或物化合同。

## 验收

1. 新建 SQLite 文件后可查询门店和商品，完成采购与未付款订单，并在关闭、重新打开后读取相同库存、订单、分录和审计。
2. 相同幂等键的重放不重复写入或扣减库存，参数变化仍返回幂等冲突。
3. 业务回调或插件失败时，同一事务内的所有变更均不落盘。
4. 两个适配器连接同一文件并发写入时，不产生丢失更新、重复订单或部分提交；无法取得锁时返回结构化存储错误。
5. 迁移可重复执行，未知 schema 版本拒绝打开；非空数据库不能再次应用 seed。
6. 既有内存适配器和开放商业测试全部继续通过，SQLite 专项在 Node 24 当前环境通过。

## 预计实现范围

- `sdk/merchant-erp-kernel/src/sqlite-schema.js`
- `sdk/merchant-erp-kernel/src/sqlite-store.js`
- `sdk/merchant-erp-kernel/src/sqlite.d.ts`
- `sdk/merchant-erp-kernel/test/sqlite-store.test.mjs`
- `sdk/merchant-erp-kernel/package.json`
- `sdk/merchant-erp-kernel/README.md`
- `docs/erp/sqlite-storage-adapter-v1-acceptance.md`
