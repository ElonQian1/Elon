---
title: 开放商业消费者关系凭证 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业消费者关系凭证 V1

## 背景

现有开放商业网络已经支持“商户授权第三方 App 调用商业能力”，但这仍是商户到 App 的访问关系，不能表达“消费者主动允许某个商户在多长时间内记住哪些关系信息”。如果只靠平台账号、订单或 CRM 私下推导客户关系，关系控制权仍会回到中心化平台。

V1 增加由消费者创建、持有和撤销的最小关系凭证。它只证明一段授权关系存在，不承载客户资料、偏好原文、联系方式、订单或支付数据。

## 决定

1. 消费者关系凭证属于“消费者项目 + 当前用户”。同一项目中的其他成员不能查看或撤销该用户的凭证。
2. 关系只能指向已经由商户主动发布到开放目录的有效商户节点。
3. V1 只支持 `preference.remember` 和 `membership.link` 两个固定范围，分别表示商户可以关联消费者主动提供的偏好值或商户自己的会员标识。凭证本身不保存这些值。
4. 凭证必须设置未来到期时间，最长 366 天；PC 默认 90 天，不提供永久关系选项。
5. 商户只能看到随机 `subject_alias`、来源 App、用途、范围、期限和状态，不返回消费者账号、用户 ID 或消费者项目 ID。
6. 同一消费者重新与同一商户建立关系时，旧凭证原子撤销，并生成新的匿名关系标识。平台不把新旧标识自动关联给商户。
7. 消费者可随时撤销本人凭证；撤销幂等且不会自动恢复。到期状态由服务端失败关闭派生，历史凭证保留用于双方审计。
8. HTTP、PC 与 MCP 共用同一领域服务。AI 代理不能冒充其他 App 或系统保留身份创建关系。
9. 审计只写入消费者项目，元数据不包含偏好值、联系方式、订单、消费者账号或外部令牌。

## 非目标

V1 不包含偏好数据仓库、CRM 联系人、消息推送、订单绑定、跨运营方身份互认、商户删除历史数据的技术强制、公开信誉评分、支付和链上关系对象。

## 实现入口

- 数据模型：`server/src/open_commerce_relationship_model.rs`
- 迁移：`server/src/open_commerce_relationship_migration.rs`
- 存储：`server/src/store/open_commerce_consumer_relationships.rs`
- 领域服务：`server/src/open_commerce_relationship_service.rs`
- HTTP：`server/src/open_commerce_relationship_api.rs`
- MCP：`server/src/open_commerce_mcp.rs`、`server/src/open_commerce_mcp_tools.rs`
- PC：`pc-frontend/src/features/open-commerce/ConsumerRelationshipManager.tsx`、`pc-frontend/src/features/open-commerce/MerchantRelationshipInbox.tsx`
- 验收：`docs/open-commerce-consumer-relationships-v1-acceptance.md`
