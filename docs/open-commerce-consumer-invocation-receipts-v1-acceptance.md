# 开放商业消费者调用凭证 V1 验收

## 已验证

- 本人可以按账户列出跨项目终态调用摘要，并读取单条详情。
- 列表不携带商户返回结果；详情只对调用发起账户开放，其他用户收到未找到。
- 进行中的调用不会进入凭证列表，也不能按详情读取。
- 凭证不包含用户 ID、项目 ID、能力内部 ID、Grant ID、幂等键、请求哈希或原始输入值。
- 请求形状只保留字段数、序列化字节数和 `contains_raw_values=false`。
- 仅 `recorded_not_charged` 可生成 V1 凭证；PC 明确显示“未扣真实资金”。
- 服务端以规范 `payload_json` 计算 SHA-256；PC 下载前重新计算摘要、解析负载并核对外层对象。
- HTTP 和 MCP 都从登录身份派生账户范围，不接受调用方指定用户 ID。
- PC 消费者沙盒可刷新、查看并下载本人调用凭证。
- Rust 编译检查、定向凭证测试、全部 `open_commerce` 回归、PC 开放商业静态契约、定向 ESLint 和生产构建通过。

## 仍未完成

- 真实支付、退款、配送与完整订单生命周期。
- 商户签名、外部时间戳、链上提交或不可抵赖证明。
- 跨运营方导入、接收方身份互认、冲突处理和重新授权；调用凭证已由可携带数据包 V3 纳入本人快照。
- 消费者项目级隔离；V1 因现有 Invocation 真源只保存账户身份而采用账户级读取。
- 对商户结果内容进行敏感字段识别或业务真实性验证。

## 验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- check --manifest-path server\Cargo.toml
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce_consumer_receipt -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce -- --nocapture
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
cd pc-frontend
npx eslint src/features/open-commerce/ConsumerInvocationReceipts.tsx src/features/open-commerce/ConsumerCommerceSandbox.tsx src/features/open-commerce/openCommerceClientApi.ts src/features/open-commerce/openCommerceClientTypes.ts --max-warnings 0
```
