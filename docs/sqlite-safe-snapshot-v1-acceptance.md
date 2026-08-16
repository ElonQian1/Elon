---
version_status: current
reviewed_at: 2026-08-16
implementation_status: implemented
verification_status: targeted_local_verified
---

# 商户 SQLite 一致性备份与离线恢复 V1 验收

## 已验证

- Node 24.8.0 下从保持打开的 WAL 数据库创建在线快照，快照包含创建前已提交记录，不包含创建后记录，并通过 `quick_check`。
- 快照回执绑定 SHA-256、字节数、`user_version`、表集合和复制页数；同摘要可独立复验。
- 通用 ERP 的门店、商品、库存、采购、平衡分录、未付款订单、幂等和审计可恢复到新文件，由 `SqliteErpStore` 重开后与备份前快照一致。
- 商户运行时已完成的幂等结果可恢复到新文件，重开后返回 `replayed`，不会重新取得动作执行权。
- 已存在目标、源目标同路径、摘要错误、版本不符、缺表、损坏数据库和直接 junction 来源全部失败关闭。
- 两个并发快照争抢同一目标时恰好一个成功，另一个返回 `SNAPSHOT_TARGET_EXISTS`；最终文件可验证且没有工具临时文件残留。
- `npm test` 为 `6 passed / 0 failed / 0 skipped`，同时覆盖通用 SQLite、真实 ERP 和真实连接器幂等存储。

## 当前边界

- 当前工具要求 Node 22.16 或更高；本机只验证 Node 24.8.0，且 `node:sqlite` 仍输出实验性警告。
- 恢复只物化到全新文件。运行时停机、配置切换、业务读取验收、旧库保留和回滚由商户部署流程负责。
- 本批未实现定时任务、远端上传、加密、保留策略、容量监控、多机高可用或多主同步。
- 本批未迁移或恢复 `cofficethinking` 生产数据库，也未执行操作系统级磁盘故障和生产灾备演练。

## 证据入口

- `sdk/sqlite-safe-snapshot/src/index.js`
- `sdk/sqlite-safe-snapshot/src/index.d.ts`
- `sdk/sqlite-safe-snapshot/test/sqlite-safe-snapshot.test.mjs`
- `sdk/sqlite-safe-snapshot/README.md`
- `docs/requirements/sqlite-safe-snapshot-v1.md`
- `docs/decisions/sqlite-safe-snapshot-v1.md`
