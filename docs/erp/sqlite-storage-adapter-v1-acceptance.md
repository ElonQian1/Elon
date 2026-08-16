---
version_status: current
reviewed_at: 2026-08-16
implementation_status: implemented
verification_status: targeted_local_verified
---

# ERP SQLite 持久化适配器 V1 验收

## 已验证

- `@yilong/merchant-erp-kernel/sqlite` 作为独立子路径导出，核心入口不加载 `node:sqlite`，既有 Node 20 存储适配器合同保持不变。
- 适配器使用版本化 V1 schema、WAL、busy timeout、只读事务和 `BEGIN IMMEDIATE` 写事务；未知未来版本及缺列的伪 V1 数据库在打开阶段失败关闭。
- 商户种子只能进入没有业务记录的数据库，不能覆盖已有门店、商品、库存、订单、幂等或审计记录。
- 采购、平衡分录、未付款订单、库存和幂等记录在关闭并重新打开 SQLite 文件后保持一致；相同请求重放不重复写入或扣库存。
- 回调失败会回滚同一事务内已经写入的库存和订单；读事务写入被结构化拒绝。
- 同一适配器并发订单按队列串行执行，不会双重扣减库存；另一个连接持有写锁时返回 `STORAGE_BUSY`，不留下第二连接的部分写入。
- 当前 Node `24.8.0` 执行 `npm test` 为 `11 passed / 0 failed`，同时覆盖原内存适配器、开放商业 Provider 和 5 项 SQLite 专项。

## 当前边界

- SQLite 子路径推荐 Node 22.13 或更高版本；22.5 至 22.12 需要 `--experimental-sqlite`。当前 Node 24 仍会输出内置 SQLite 的实验性警告，使用方必须按自己的运行时发布策略评估升级。
- 这是商户自有单机/单写参考适配器，不是多主、分布式或跨设备同步数据库；建议同一进程对一个数据库只创建一个适配器实例。
- 本批未迁移 `cofficethinking`，未生成商户生产 schema 迁移，也未发布 npm 包或部署商户运行时。
- 订单继续固定为未付款；没有支付、履约、外部平台授权、真实消费者公网调用或资金移动。

## 证据入口

- `sdk/merchant-erp-kernel/src/sqlite-schema.js`
- `sdk/merchant-erp-kernel/src/sqlite-store.js`
- `sdk/merchant-erp-kernel/src/sqlite.d.ts`
- `sdk/merchant-erp-kernel/test/sqlite-store.test.mjs`
- `docs/requirements/merchant-erp-sqlite-store-v1.md`
