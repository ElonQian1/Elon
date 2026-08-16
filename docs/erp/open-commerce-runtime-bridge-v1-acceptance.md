---
version_status: current
reviewed_at: 2026-08-16
implementation_status: implemented
verification_status: targeted_local_verified
---

# ERP 开放商业商户运行时桥 V1 验收

## 已验证

- 连接器能力定义支持可选 `action`，任意动作缺少平台 `action_confirmation_id` 时在业务
  处理器和幂等占位前失败关闭；历史 `order.commit` 继续保留相同保护。
- `createMerchantRuntimeBinding` 将 ERP Provider 的四项现有能力编译为连接器定义和处理器，
  只把 `order.create` 标记为动作，并从其公开输入 Schema 删除重复的业务幂等键。
- 绑定要求签名信封商户与 ERP 商户一致，并将平台幂等键注入 ERP 事务；缺少确认时没有
  订单和库存写入。
- 首次确认下单生成真实 ERP 未付款订单和标准业务回执。连接器使用新的内存幂等存储重新
  创建后，同一平台键仍由 ERP 内核重放相同订单，库存只从 5 降至 4。
- 不同输入复用同一平台键以及运行时商户身份失配均失败关闭，不产生第二订单或第二次扣减。
- `@elon/open-commerce-connector` 完整 Node 测试为 `90 passed / 0 failed`；
  `@yilong/merchant-erp-kernel` 完整 Node 测试为 `13 passed / 0 failed`。

## 当前边界

- 运行时示例仍使用内存幂等存储；生产商户必须提供耐久 `claim/complete/release` 实现。
- 本批只验证 SDK 组合和真实内存 ERP 事务，没有启动 HTTP、生成生产凭据或部署商户项目。
- 订单固定 `awaiting_payment/unpaid`，标准回执是商户运行时声明，不代表支付、履约、退款、
  外部平台授权或真实资金移动。
- `cofficethinking` 尚未改用该绑定；其迁移必须在对应项目中单独盘点、适配和验收。

## 证据入口

- `sdk/open-commerce-connector/src/merchant-runtime.js`
- `sdk/open-commerce-connector/test/merchant-runtime-auth.test.mjs`
- `sdk/merchant-erp-kernel/src/open-commerce.js`
- `sdk/merchant-erp-kernel/test/open-commerce-runtime.test.mjs`
- `docs/requirements/merchant-erp-open-commerce-runtime-bridge-v1.md`
