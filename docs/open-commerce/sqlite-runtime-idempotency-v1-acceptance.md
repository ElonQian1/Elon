---
version_status: current
reviewed_at: 2026-08-16
implementation_status: implemented
verification_status: full_local_verified
---

# 商户运行时 SQLite 幂等存储 V1 验收

## 已验证

- 真实 SQLite 文件关闭并重新打开后，同一平台键和输入重放原业务结果，处理器只执行一次，
  调用方修改首次响应不会改写持久化结果。
- 同键异输入返回冲突；两个独立数据库句柄在租约期内返回 busy，超时后新 invocation
  接管，旧 invocation 无法完成或释放新记录。
- 处理失败显式释放后可重新领取；超过 1 MiB 的 JSON 结果在写库前拒绝。
- 成功结果被篡改为非法 JSON 时返回稳定损坏错误且不泄漏正文；未知 Schema 版本在原子
  初始化中失败关闭，不创建 V1 业务表。
- 外部写锁会映射为稳定 busy 错误；关闭操作幂等，关闭后调用失败关闭。
- SQLite 专项 `8 passed / 0 failed`；连接器完整 Node 回归 `98 passed / 0 failed`；
  `npm pack --dry-run --json` 通过并包含新的 JavaScript 与类型声明。
- 包名子路径 `@elon/open-commerce-connector/sqlite-idempotency` 已在当前 Node 24.8 环境完成
  自引用导入和内存库打开/关闭 smoke；测试结束后专项临时目录为零。

## 当前边界

- 本批验证本机 Node SQLite 文件，不代表多机、高可用、网络磁盘或生产备份恢复验收。
- 当前没有独立 `tsc` 命令，也未用 Node 20 进程实测根入口；Node 20 兼容结论仅限于根导出
  图未静态引入新子路径，使用 SQLite 子路径仍须选择提供 `node:sqlite` 的 Node 版本。
- 适配器不启动 HTTP、不管理 HMAC 密钥、不创建目录、不轮换文件，也不自动清理成功记录。
- 幂等存储只保护连接器调用结果；ERP 订单、库存、支付和外部平台仍需各自事务和幂等合同。

## 证据入口

- `sdk/open-commerce-connector/src/merchant-runtime-sqlite-store.js`
- `sdk/open-commerce-connector/src/merchant-runtime-sqlite-store.d.ts`
- `sdk/open-commerce-connector/test/merchant-runtime-sqlite-store.test.mjs`
- `docs/decisions/open-commerce-sqlite-runtime-idempotency-v1.md`
- `docs/requirements/open-commerce-sqlite-runtime-idempotency-v1.md`
