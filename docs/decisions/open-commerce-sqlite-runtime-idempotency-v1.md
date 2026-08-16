---
title: 开放商业商户运行时 SQLite 幂等存储 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-16
---

# 开放商业商户运行时 SQLite 幂等存储 V1

## 决定

1. `@elon/open-commerce-connector/sqlite-idempotency` 提供可选 SQLite 实现；连接器根入口
   不导入 `node:sqlite`，继续保持既有 Node 20 加载边界。使用该子路径的宿主需要具备
   `node:sqlite` 的 Node 版本。
2. 存储键继续遵守现有协议的“商户、App、能力、幂等键”，不自行加入用户、凭据环境或
   动作确认字段，以免内存与 SQLite 实现产生不同重放语义。
3. 领取使用 `BEGIN IMMEDIATE` 串行化同一文件内写事务；记录保存请求摘要、当前
   invocation、`processing/succeeded` 状态、JSON 结果和毫秒更新时间。
4. 未超时重复返回 `busy`；超时后新 invocation 替换所有者。完成和释放仍必须同时匹配
   当前 invocation 与请求摘要，旧执行者不能影响新所有者。
5. 数据库使用独立元数据表、V1 Schema、WAL、`synchronous=FULL` 和有界 busy timeout；
   未知版本、锁冲突和损坏结果转为不含原始记录的稳定错误。
6. 成功结果限制为 1 MiB UTF-8 JSON。适配器不自动清理成功记录，避免在没有业务保留
   策略时破坏重放保证。

## 兼容与回滚

- 内存实现和商户自有实现保持可用；新子路径不改变 `MerchantRuntimeIdempotencyStore`。
- 回滚时可停止使用 SQLite 子路径，但已保存的重放结果不会自动迁移到其他实现。
- 数据库文件可与商户其他 SQLite 数据放在同一文件，但本模块只拥有两个
  `yilong_merchant_runtime_idempotency*` 表，不使用全局 `user_version`。

## 非目标

该实现不是网络文件系统锁、跨机器共识、自动备份、磁盘配额、跨区域复制或灾难恢复方案，
也不代替商户订单数据库自身的库存与订单事务。
