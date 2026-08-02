---
title: 开放商业消费者关系安全续期 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业消费者关系安全续期 V1

## 背景

消费者关系凭证有明确期限，但仅允许用户重新创建关系会产生两个问题：PC 无法在到期前给出明确操作，AI 或网络重试还可能生成多个匿名标识。直接延长原凭证又会让旧匿名身份长期存在，不利于最小化关联。

V1 把续期定义为“撤销旧关系并创建一个继承授权的新关系”，同时给同一来源关系增加严格的幂等约束。

## 决定

1. 只有关系所属的消费者用户可以续期；同项目其他成员不能续期。
2. 首次续期在领域服务和写事务内重新确认商户仍主动发布、来源 App 仍有效且归当前用户所有。PC 和 MCP 均不能冒充其他 App。
3. 续期继承原关系的商户、授权范围和用途，但使用调用方选择的新期限；期限必须晚于当前时间且不超过 366 天。
4. 续期在即时事务内撤销旧关系、创建新关系并轮换随机 `subject_alias`。旧记录保留，不恢复为有效状态。
5. 内部字段 `renewed_from_relationship_id` 对非空值建立唯一索引。同一来源关系至多产生一个直接后继，重复请求返回同一后继，不生成新别名或重复审计。
6. 已成功续期后的重试优先返回既有后继，即使商户随后撤回目录发布或来源 App 停用，也不会把成功结果变成未知结果。继续延长时必须以当前后继发起下一次续期。
7. 续期来源和新旧别名映射只进入消费者项目审计，不进入公开关系模型和商户读取接口。商户只能分别看到匿名历史，不能由平台获得续期链。
8. PC 在到期前 14 天显示提醒，只对同一商户的最新关系提供续期操作；不会后台自动续期。
9. 有效、过期或已撤销的本人关系均可作为新的主动授权来源。既有删除请求继续独立处理，续期不会撤回、完成或改写删除请求。

## 非目标

V1 不包含短信、邮件或系统推送，不包含自动续期、商户代续期、授权范围扩张、跨运营方身份迁移、真实 CRM 数据重绑、支付或链上关系对象。

## 实现入口

- 迁移：`server/src/open_commerce_relationship_renewal_migration.rs`
- 存储：`server/src/store/open_commerce_consumer_relationships.rs`
- 领域服务：`server/src/open_commerce_relationship_service.rs`
- HTTP：`server/src/open_commerce_relationship_api.rs`
- MCP：`server/src/open_commerce_mcp.rs`、`server/src/open_commerce_mcp_tools.rs`
- PC：`pc-frontend/src/features/open-commerce/ConsumerRelationshipManager.tsx`
- 验收：`docs/open-commerce-consumer-relationship-renewal-v1-acceptance.md`
