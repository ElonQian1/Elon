---
title: 开放商业消费者关联数据删除请求 V1
status: accepted
date: 2026-08-02
owners: backend, product
---

# 开放商业消费者关联数据删除请求 V1

## 背景

消费者关系凭证已经允许用户建立、限时和撤销一段匿名商户关系，但“撤销未来授权”不等于“请求删除商户此前根据该关系关联的数据”。平台当前也没有接管商户外部 CRM、会员或订单系统，因此不能把一个按钮伪装成已经完成外部数据删除。

V1 增加最小删除请求回执：消费者可对本人关系发起请求，商户通过匿名别名接收并处理，双方共享状态和商户说明。平台保存请求事实，不保存待删除的偏好值、联系方式、订单或外部系统凭据。

## 决定

1. 删除请求属于“消费者项目 + 当前用户”，只能引用该用户持有的关系凭证。
2. 创建请求与撤销对应关系在同一即时事务中完成。无论关系此前有效、过期或已撤销，请求都不恢复或扩大任何授权。
3. 同一关系同时最多存在一个 `requested` 或 `in_progress` 请求；重复创建返回原请求，不重复写创建审计。
4. 状态机固定为 `requested -> in_progress -> completed/rejected`。消费者只能在 `requested` 时撤回为 `withdrawn`；撤回不恢复关系。
5. 只有商户项目编辑者可以接单、声明完成或拒绝。完成和拒绝必须填写说明；重复执行同一终态保持幂等。
6. 商户响应只包含关系 ID、商户 ID、随机 `subject_alias`、请求类型、状态、处理说明和时间，不返回消费者账号、用户 ID 或消费者项目 ID。
7. `completed` 的稳定含义是 `merchant_attested_completed`：商户声明已完成其控制范围内的处理。它不是平台验证、密码学证明、法律合规认证或外部适配器删除结果。
8. HTTP、PC 与 MCP 共用同一领域服务和状态机。所有有效变化写入对应消费者项目或商户项目的开放商业审计。

## 非目标

V1 不包含消费者偏好数据保险箱、字段级数据内容、外部 CRM 自动删除、平台级删除证明、法定时限判断、自动催办或处罚、跨运营方工单迁移、真实支付和链上对象。

## 实现入口

- 模型：`server/src/open_commerce_data_request_model.rs`
- 迁移：`server/src/open_commerce_data_request_migration.rs`
- 存储与状态机：`server/src/store/open_commerce_consumer_data_requests.rs`
- 领域服务：`server/src/open_commerce_data_request_service.rs`
- HTTP：`server/src/open_commerce_data_request_api.rs`
- MCP：`server/src/open_commerce_mcp.rs`、`server/src/open_commerce_mcp_tools.rs`
- PC：`pc-frontend/src/features/open-commerce/ConsumerDataRequestManager.tsx`、`pc-frontend/src/features/open-commerce/MerchantDataRequestInbox.tsx`
- 验收：`docs/open-commerce-consumer-data-erasure-requests-v1-acceptance.md`
