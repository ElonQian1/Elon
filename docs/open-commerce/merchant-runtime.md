---
title: 商户自有运行时接入手册
status: current
owner: backend
reviewed_at: 2026-07-31
---

# 商户自有运行时接入手册

## 作用

商户运行时把一龙平台的 Merchant、Capability、Grant、Invocation、Meter 和 Audit 主干，连接到商户自己的 ERP 后端。平台负责发现、授权、签名、计量和审计；商户后端负责真实商品、价格、库存、报价与订单。

```text
消费者 App / AI
  -> 一龙发现与 Grant
  -> 幂等 Invocation
  -> 已验证 RuntimeBinding
  -> 商户 /commerce/v1/invoke
  -> 商户 ERP 商品、报价、订单
  -> 业务结果 + 平台计量和结算回执
```

APK 或网页是商户管理入口，真实能力由持续在线的商户后端或本地网关提供。

## 平台配置

1. 在项目开放商业工作区创建商户节点。
2. 配置 `merchant_runtime` 运行绑定：白名单 HTTPS 地址、凭据环境变量引用、可选 Manifest SHA-256 和超时。
3. 在平台服务端设置该凭据引用对应的共享密钥，并将主机加入 `OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS`。
4. 执行“签名验证”；只有状态为 `active` 才能调用真实能力。
5. 发布 `merchant_runtime` 能力。交易能力使用 `authorized`，并为调用 App 创建最小范围 Grant。

平台环境示例：

```text
OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS=coffee.example.com
OPEN_COMMERCE_RUNTIME_SECRET_COFFICE=<与商户后端一致的32位以上随机密钥>
```

共享密钥不进入浏览器、能力配置、Git 或项目文档。

## 请求签名

商户端接收原始 JSON body，并验证：

```text
HMAC-SHA256(secret, unix_timestamp + "." + raw_json_body)
```

请求头为 `x-yilong-runtime-key-id`、`x-yilong-runtime-timestamp` 和 `x-yilong-runtime-signature: v1=<hex>`。商户端必须校验时间窗口、商户 ID、信封版本、调用方和幂等键。

## 参考能力

| 能力 | 访问级别 | 服务端事实 |
|---|---|---|
| `merchant.profile.read` | public | 门店与品牌资料 |
| `catalog.search` | public | 当前在售商品 |
| `product.detail.read` | public | 单个商品、服务端价格与库存 |
| `order.quote.create` | authorized | 服务端价格生成的短期报价 |
| `order.commit` | authorized | 用户确认后事务下单与扣减库存 |
| `order.status.read` | authorized | 调用方自己的订单状态 |

平台健康验证使用保留能力 `system.health`，不作为消费者能力发布。

## 金额和回执

- 商户业务金额使用整数最小货币单位，例如 `2600 CNY minor = 26.00 元`。
- 平台调用计量使用整数微单位 `amount_micros`。
- 商品订单总额不包含平台 API 调用费。
- 平台返回 `settlement_receipt`，当前固定 `funds_moved=false`；真实收费仍未启用。
- 商户运行时对同一商户、App、能力和幂等键返回同一业务结果，同键不同输入返回冲突。

商户运行时可以在业务结果中附带可选的标准业务回执，供商户工作台和后续 ERP/CRM 适配器识别；平台不会从任意 `order_id` 字段猜测订单事实：

```json
{
  "_yilong_business_receipt": {
    "schema": "open_commerce.merchant_business_receipt.v1",
    "entity_type": "order",
    "reference_id": "order-1001",
    "state": "confirmed",
    "occurred_at": "2026-08-03T01:00:00Z",
    "amount_minor": 2600,
    "currency": "CNY"
  }
}
```

新响应中的非法标准回执会使调用失败并降级运行绑定。标准回执仍是商户运行时声明；平台只保存调用证据和摘要，不把它解释为真实支付或履约证明。

## 失败处理

- 签名、时间戳或密钥标识错误：拒绝，不进入业务逻辑。
- 运行绑定未验证或已降级：平台拒绝转发。
- 报价过期、商品停售或库存不足：拒绝提交，要求重新报价。
- 未获得用户明确确认：拒绝 `order.commit`。
- 商户后端超时或返回身份不匹配：平台记录失败审计并把运行绑定标为 `degraded`。

## 参考节点

`cofficethinking` 参考节点的部署配置、商品管理接口和订单落库说明见其 `docs/open_commerce_runtime.md`。跨仓库协议真源为两仓库内容一致的 `contracts/open-commerce/merchant-runtime-v1.json`。

新的 Node.js 商户项目可以复用 `sdk/open-commerce-connector` 的 `createMerchantRuntime`，直接获得签名验证、重放窗口、商户与 Grant 边界、订单明确确认、幂等生命周期、Manifest 摘要和标准信封。商户仍需实现自己的能力处理器和持久化幂等存储；SDK 的内存存储仅供本机开发，不能用于生产订单。

## 验证

```powershell
powershell -NoProfile -File scripts\test-open-commerce-merchant-runtime-contract.ps1 `
  -CoffeeRepo D:\rust\active-projects\cofficethinking
```

平台 Rust 测试还会启动临时商户服务，验证签名、Manifest、真实处理器调用、计量、审计和平台幂等重放。
