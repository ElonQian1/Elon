---
title: 开放商业消费者偏好档案与关系级披露 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业消费者偏好档案与关系级披露 V1

## 背景

消费者关系 V1 已允许用户授予 `preference.remember`，但关系凭证刻意不保存偏好值。因此商户只能知道“用户允许关联主动提供的偏好”，还没有平台内可验证的数据披露动作；消费者沙盒中的偏好也只存在于一次查询表单中。

V1 需要证明消费者可以保存自己的低敏结构化偏好，并在不暴露账号身份的前提下，选择字段向特定商户关系披露。它不是保存任意个人资料的完整数据保险箱。

## 决定

1. 偏好档案按“消费者项目 + 当前用户”隔离。同一项目中的其他成员不能读取、修改或删除该档案。
2. V1 只接受类别、标签、城市、单位调用价格上限和“优先公开能力”标记；不接受自由文本、联系方式、证件、订单、支付、健康或定位轨迹。
3. 保存档案不产生商户访问权，也不自动影响发现请求。用户必须显式选择“用于本次发现”。
4. 商户披露采用不可变语义的字段快照，而不是授予商户实时查询个人档案的权限。用户每次更新披露时明确选择字段，并绑定档案修订号。
5. 披露只能绑定本人持有、状态有效且包含 `preference.remember` 的关系。商户只看到 `subject_alias`、明确共享字段、快照值、档案修订号和时间，不看到消费者账号、用户 ID 或消费者项目。
6. 商户列表只返回仍有效且未到期关系的披露。关系撤销、到期或被删除请求撤销后立即失败关闭；关系续期产生新匿名标识，旧披露不会自动继承。
7. 用户可单独撤回披露。删除偏好档案会在同一事务中删除本项目内该用户的全部披露快照，但不声称能够删除商户已经复制到外部系统的数据；外部删除仍走匿名删除请求和商户履约流程。
8. HTTP、PC 与 MCP 共用同一领域服务。审计只记录字段名、数量、修订号、商户和匿名关系标识，不记录偏好值。
9. V1 数据保存在平台数据库中，通过应用层项目与用户权限控制；不声称已经实现终端持钥、字段级加密、跨运营方导入或完整消费者数据保险箱。
10. 消费者可携带数据包 V1 保持不可变契约，不自动加入偏好档案或披露。偏好迁移必须另行设计版本化导出、接收方验证和重新同意流程。

## 非目标

- 商户实时订阅消费者档案变化；
- 从订单、聊天或平台行为自动推导偏好；
- 敏感个人数据存储、出售或默认公开；
- 跨商户共享同一匿名标识；
- 自动营销、自动消息发送、真实订单绑定或外部 CRM 删除证明；
- 生产级密钥托管、端到端加密和跨运营方偏好迁移。

## 实现入口

- 模型与校验：`server/src/open_commerce_consumer_preference_model.rs`、`server/src/open_commerce_consumer_preference_service.rs`
- 迁移与存储：`server/src/open_commerce_consumer_preference_migration.rs`、`server/src/store/open_commerce_consumer_preferences.rs`
- HTTP 与 MCP：`server/src/open_commerce_consumer_preference_api.rs`、`server/src/open_commerce_consumer_preference_mcp.rs`
- PC：`pc-frontend/src/features/open-commerce/ConsumerPreferenceProfilePanel.tsx`、`pc-frontend/src/features/open-commerce/MerchantPreferenceInbox.tsx`
- 验收：`docs/open-commerce-consumer-preference-disclosures-v1-acceptance.md`
