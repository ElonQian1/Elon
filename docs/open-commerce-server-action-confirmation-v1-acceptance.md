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
- 本人可幂等取消尚未创建 Invocation 的 `pending` 或 `confirmed` 确认；重复取消只写一条审计，取消后同一幂等请求可准备新的确认。
- 错误短语、其他用户、其他 App、自然过期和已消费确认不能写入 `canceled_at`；安全投影把主动取消显示为 `canceled`。
- 取消与 Invocation 创建并发执行时恰好一方成功，持久记录不会同时出现 `canceled_at` 和 `invocation_id`。
- MCP 取消返回 `invocation_created=false`，随后读取返回 `status=canceled` 和 `next_step=stop`；动作确认定向 Rust 套件通过。
- MCP 读取覆盖 `pending`、`confirmed`、`consumed` 和即时派生的 `expired`，并为每种状态返回稳定下一步。
- 其他用户、其他 App 和未知确认 ID 返回同类不可见错误；响应不含原始输入、请求摘要、用户 ID、商户项目 ID 或能力内部 ID。
- 读取过期确认不改写持久状态，不新增审计或 Invocation；v166 可保留旧表四种既有状态，并可安全重复执行。
- 带调用次数和金额预算的 Grant 连续读取三次后，使用次数、使用金额和更新时间均不变。
- 取消与 Invocation 创建使用两个独立 WAL 数据库连接同时竞争时恰好一方成功，最终记录仍不能同时被取消和消费。

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
