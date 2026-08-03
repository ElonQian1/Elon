---
title: 开放商业服务端动作确认 V1
status: accepted
owner: open-commerce
reviewed_at: 2026-08-02
---

# 开放商业服务端动作确认 V1

## 背景

PC 已能要求消费者在动作能力表单上确认当前输入，但该约束只存在于页面。HTTP、MCP 或第三方测试应用仍可绕过页面直接调用 `action`，服务端也无法判断一份确认是否对应当前商户、能力、授权、幂等键和输入。客户端布尔值 `confirmed=true` 同样可被调用方自行填写，不能成为安全边界。

## 决定

1. `query` 保持单阶段调用；所有 `action` 必须使用服务端两阶段确认。
2. 调用方先提交完整待调用请求，服务端完成身份、App、目录、授权、能力状态和输入契约校验，再生成 5 分钟有效的 `pending` 确认。
3. 确认绑定当前用户、App、商户、能力、Grant、幂等键和服务端计算的输入摘要。数据库只保存字段形状和摘要，不保存原始输入值。
4. 当前用户必须通过独立确认操作把状态从 `pending` 改为 `confirmed`；确认短语固定为 `CONFIRM_ACTION`，用于阻止误调用和保持协议明确。
5. Invocation 创建和确认消费必须在同一数据库事务内完成。确认从 `confirmed` 原子变为 `consumed` 并绑定 Invocation ID，不能用于第二个幂等键或另一份输入。
6. 同一幂等调用可以携带已消费的同一确认读取既有结果；不同输入、Grant、幂等键、用户或 App 均失败关闭。
7. HTTP、MCP、消费者 PC、商户能力测试器和开发者测试凭据共用同一领域服务。MCP 的确认工具标记为有副作用，并要求宿主只在用户明确同意后调用。
8. 能力状态、Grant、App 封禁、目录发布、配额和预算在真正调用时再次校验；准备确认不预留调用配额或资金。
9. 准备接口本身按用户、App、商户、能力和幂等键复用。同一精确请求会返回已有的 `pending`、`confirmed` 或已绑定 Invocation 的 `consumed` 确认，使动作成功但网络回包丢失时仍可恢复原结果；相同幂等键更换输入或 Grant 时失败关闭。
10. 每个用户与 App 同时最多保留 20 份未过期的 `pending` 或 `confirmed` 确认。准备新确认时会标记已过期记录，并清理创建超过 7 天且未产生 Invocation 的过期记录。
11. MCP 新增 `open_commerce_get_my_action_confirmation`，允许代理在跨轮次或准备响应丢失后按确认 ID 重新读取。服务端再次绑定当前用户与当前 App，只返回商户、能力、Grant、幂等键、输入字段形状、时间、状态、Invocation ID 和稳定下一步，不返回原始输入值、请求摘要或内部项目身份。
12. 读取时对 `pending` 和 `confirmed` 即时计算是否过期，但不改写数据库；它不替代宿主向用户展示本轮实际输入，也不确认或执行动作。
13. 用户明确停止时，可提交固定短语 `CANCEL_ACTION` 调用 `open_commerce_cancel_my_action_confirmation`。服务端只允许当前用户和当前 App 取消尚未创建 Invocation 的 `pending` 或 `confirmed` 确认；重复取消幂等返回原结果。
14. 为兼容既有状态约束，取消后的持久状态仍为 `expired`，并由新增 `canceled_at` 区分主动取消和自然过期；消费者安全投影显示 `canceled`。自然过期、已消费或其他调用方均不能写入取消时间。

## 安全边界

- 该协议证明服务端收到了一次独立、已认证且与具体请求绑定的确认事件；它不能从技术上证明鼠标或屏幕前一定是自然人，可信宿主仍必须落实用户交互。
- 商户仍须正确声明 `query` 与 `action`。平台确认不替代商户运行时的报价、库存事务、订单状态机、退款、支付、风控或履约确认。
- 确认和调用仍只产生 `recorded_not_charged` 技术服务计量，不移动真实资金，也不构成订单、支付或链上证明。
- 开发者测试 Token 只用于沙盒验证，不等于生产应用身份互认。
- 准备确认会写入短时状态和审计，因此不是只读操作；幂等复用只减少重复状态，不意味着可以跳过独立确认。
- 状态读取只能恢复服务端已经持有的脱敏确认元数据。由于服务端按设计不保存原始输入值，可信宿主仍须从当前会话或本地安全状态向用户展示将要执行的真实参数。
- 取消只终止确认凭证，不能撤销已创建的 Invocation，也不是订单撤销、退款、支付冲正、库存回滚或外部商户动作补偿。

## 实现引用

- `server/src/open_commerce_action_confirmation_model.rs`
- `server/src/open_commerce_action_confirmation_service.rs`
- `server/src/open_commerce_action_confirmation_mcp.rs`
- `server/src/store/open_commerce_action_confirmations.rs`
- `server/src/open_commerce_action_confirmation_api.rs`
- `pc-frontend/src/features/open-commerce/ConsumerCommerceSandbox.tsx`
