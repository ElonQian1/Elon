---
title: 开放商业消费者可携带数据迁移预演与偏好采用 V1
status: accepted
date: 2026-08-03
owners: backend, product
---

# 开放商业消费者可携带数据迁移预演与偏好采用 V1

## 背景

隔离导入和来源签名解决了“文件能否带走”和“由谁签发”，但导入包仍只是静态快照。直接恢复全部关系、授权、订单和 ERP 状态会把来源环境的内部标识错误写入目标环境，也可能绕过目标商户重新授权。因此首个采用能力只处理消费者自己的低敏偏好，并把其他内容转化为只读迁移预演。

## 决定

1. 用户可为任意本人隔离导入生成迁移预演。预演逐字段比较类别、标签、城市、价格上限和公开商户偏好，并展示当前偏好修订号。
2. 来源关系只生成重新授权候选，保留来源商户 ID、来源状态、请求范围和用途；固定 `requires_reauthorization=true`，不创建目标关系或 Grant。
3. 预演固定 `automatic_relationship_restore=false` 和 `automatic_business_write=false`。数据请求、披露、调用凭证、订单结果和 ERP 状态都不自动采用。
4. 用户可明确确认采用导入包中的低敏偏好。服务端先按当前偏好规则重新规范化，再以预演时的修订号执行乐观并发检查；档案已变化时拒绝覆盖。
5. 每次采用在同一数据库事务中保存采用前偏好、采用前修订、采用后偏好和结果修订。一个导入包同一时间只能存在一个未回滚的偏好采用记录。
6. 回滚要求用户再次明确确认，并要求当前档案修订仍等于采用结果修订。采用后发生任何其他修改时，回滚失败关闭，不能覆盖后续用户操作。
7. 若采用前没有偏好档案，回滚删除新档案；若采用前存在档案，回滚写回旧偏好并产生新修订。采用与回滚均写审计日志。

## 边界

- 未签名包也可由数据所有者主动采用，但预演必须明显展示来源未认证状态。
- 当前不自动匹配跨运营方商户身份；独立 V1 允许消费者手工确认目标商户并重新发起授权，但仍不复制旧关系、Grant、披露或删除请求。
- 当前不迁移订单、支付、退款、履约、ERP、CRM 或商户私有数据。
- 字段级选择已由独立 V1 承接；当前仍没有多个包合并、三方冲突解决或跨设备审批。

## 实现入口

- `server/src/open_commerce_portability_adoption_model.rs`
- `server/src/open_commerce_portability_adoption_service.rs`
- `server/src/store/open_commerce_consumer_portability_adoptions.rs`
- `server/src/open_commerce_portability_adoption_api.rs`
- `server/src/open_commerce_portability_adoption_migration.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityAdoptions.tsx`
