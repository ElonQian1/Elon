# 开放商业服务端动作确认 V1 验收

## 已验证

- 未携带服务端确认的 `action` 在 Invocation 创建前失败，不产生计量记录。
- 准备确认会复用现有身份、App、目录、Grant、能力状态和输入 Schema 校验。
- 确认默认 5 分钟过期；其他用户或 App 不能确认和消费。
- 确认只保存输入字段形状和 SHA-256 请求摘要，不保存原始业务值。
- 输入、Grant、幂等键、商户或能力发生变化后，旧确认不能执行。
- Invocation 创建与确认消费在一个事务内完成；确认只绑定一个 Invocation。
- 同一幂等请求可使用已消费确认安全重放，不能借此创建第二次动作。
- 同一精确请求重复准备会复用原确认；动作成功但响应丢失后，可凭保留的幂等键恢复已消费确认和原 Invocation。
- 相同幂等键不能换用其他输入或 Grant；每个用户与 App 的活动确认最多 20 份。
- 准备新确认会标记已过期状态，并清理创建超过 7 天且没有 Invocation 的废弃确认。
- `query` 继续无需确认，原调用协议兼容。
- HTTP、MCP、PC 消费者表单、商户测试器和开发者测试凭据均接入同一服务端确认规则。
- Rust 迁移与领域测试、开放商业 PC 契约测试、定向 ESLint 和生产构建通过。

## 仍未完成

- WebAuthn、设备签名、跨设备确认、可信展示证明或外部时间戳。
- 生产第三方应用审核和跨运营方身份互认。
- 商户运行时内部订单、支付、退款、配送和履约状态机的统一确认协议。
- 真实扣款、资金托管、链上提交和自动结算。

## 验证入口

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- test --manifest-path server\Cargo.toml open_commerce_action_confirmation --no-fail-fast
npm --prefix pc-frontend run test:open-commerce
npm --prefix pc-frontend run build
```
