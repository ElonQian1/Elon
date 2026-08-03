---
title: 开放商业消费者可携带偏好字段级采用 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业消费者可携带偏好字段级采用 V1

## 背景

V1 已能预演并采用导入包中的完整低敏偏好档案，但消费者可能只想带走城市或标签，而保留目标环境已有的价格上限与公开商户偏好。整包覆盖不符合最小数据使用原则，也会增加误覆盖风险。

## 决定

1. 可选择字段固定为 `categories`、`tags`、`city`、`max_unit_price_micros` 和 `prefer_public`。
2. 用户必须至少选择一个预演中真实发生变化的字段；未知字段、重复字段和未变化字段不会形成新修订。
3. 服务端重新读取当前偏好档案并复核预演修订号，再把所选导入值合并到当前档案。未选字段保持目标环境当前值。
4. 合并结果继续通过既有偏好规范化和 V141 单事务采用流程，完整保存采用前和采用后快照。
5. 采用记录的 `selected_fields` 由前后快照确定性派生，不增加第二套选择状态或数据库迁移；旧记录保持兼容。
6. 回滚仍恢复采用前完整快照，并在采用后档案已变化时失败关闭。

## 边界

- 字段选择不恢复关系、Grant、披露、订单、支付、ERP、CRM 或结算。
- 来源签名状态继续只作为来源提示，不自动提高字段权限。
- 当前不支持多包合并、三方冲突解决或跨设备审批。
- 当前实现尚未编译、运行接口、执行 PC 构建或验证旧记录兼容。

## 实现引用

- `server/src/open_commerce_portability_adoption_model.rs`
- `server/src/open_commerce_portability_adoption_service.rs`
- `server/src/store/open_commerce_consumer_portability_adoptions.rs`
- `pc-frontend/src/features/open-commerce/ConsumerPortabilityAdoptions.tsx`
- `docs/open-commerce-consumer-portability-selective-adoption-v1-acceptance.md`
