---
title: "通用 ERP 开放商业商户运行时桥 V1"
owner: merchant-erp
priority: p1
status: current
reviewed_at: 2026-08-16
implementation_status: implemented_locally_verified
---

# 通用 ERP 开放商业商户运行时桥 V1

## 背景

`@yilong/merchant-erp-kernel` 已提供商户隔离的商品、库存和未付款订单能力，
`@elon/open-commerce-connector` 已提供签名验证、Grant、动作确认、幂等和标准运行时信封。
目前两个 SDK 仍需商户重复编写能力定义、处理器、平台幂等键转换和订单业务回执，
且连接器只对历史能力键 `order.commit` 硬编码动作确认，不能覆盖 ERP 的
`order.create` 或未来其他声明为动作的能力。

## 范围

1. 连接器能力定义可显式声明 `action`，所有动作在进入幂等占位和业务处理器前都必须
   携带平台已消费的 `action_confirmation_id`。
2. 未声明 `action` 的历史 `order.commit` 继续按动作处理，避免旧商户降低保护等级。
3. ERP SDK 提供不依赖连接器包的绑定编译器，把 ERP Provider 转换为连接器所需的
   `merchantId`、能力定义和处理器。
4. `order.create` 的运行时公开输入不要求消费者重复提交幂等键；绑定必须使用签名信封
   中的平台幂等键调用 ERP 内核。
5. ERP 订单结果附带严格的 `open_commerce.merchant_business_receipt.v1`，引用真实 ERP
   订单、使用 ERP 创建时间并固定声明未移动资金。
6. 绑定在运行时商户身份与 ERP 商户身份不一致时失败关闭，不读取或返回密钥。

## 验收标准

1. 任意 `action: true` 能力缺少动作确认时不执行处理器、不占用幂等结果。
2. 历史 `order.commit` 在未声明 `action` 时仍要求动作确认。
3. ERP 绑定公开四项现有能力，且只有 `order.create` 标记为动作。
4. 相同平台幂等键重放同一 ERP 下单只生成一个订单并只扣减一次库存。
5. 订单结果包含可被现有证据和 ERP 衔接链识别的标准业务回执。
6. 商户身份失配、不同输入复用幂等键和处理失败均失败关闭。
7. 两个 SDK 的既有完整 Node 测试继续通过。

## 非目标

- 不创建支付、退款、配送、财务总账或真实资金移动。
- 不启动 HTTP 服务，不部署商户项目，也不创建生产凭据。
- 不接入美团、抖音、京东、淘宝闪购或其他外部平台。
- 不把内存幂等存储描述为生产存储；生产商户仍须提供耐久连接器幂等适配器。
