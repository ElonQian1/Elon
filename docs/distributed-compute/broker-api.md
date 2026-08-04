---
title: 分布式算力 Broker HTTP 与 MCP 控制面
status: current
reviewed_at: 2026-08-04
owners: backend, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Broker HTTP 与 MCP 控制面

## 1. 当前状态

本控制面已写入代码，但尚未编译、执行 v165-v176 迁移或运行 HTTP/MCP 验证，状态固定为 `implementation_uncompiled`。它只开放平台人民币余额的原子 Reserve，以及 Attempt 尚未激活时的 Release/Expire；不能描述为完整算力交易或结算系统。

HTTP 与项目级 MCP 共用 `compute_federation_broker_service`，最终都进入同一 Store Broker 和不可变回执。客户端不能提交 `consumer_account_id`，Release/Expire 的 `occurred_at` 也由服务端生成。

## 2. HTTP 接口

全部接口要求一龙用户 Bearer 会话，只能读取或操作当前登录用户自己的对象。

| 方法 | 路径 | 作用 |
|---|---|---|
| GET | `/api/me/compute/jobs?limit=20` | 列出本人最近的当前 Job |
| GET | `/api/me/compute/jobs/:job_id` | 读取本人 Job 的当前 revision、digest 与合同 |
| GET | `/api/me/compute/reservations?limit=20` | 列出本人最近的当前 Reservation |
| GET | `/api/me/compute/reservations/:reservation_id` | 读取本人 Reservation 的当前 revision、digest 与绑定 |
| POST | `/api/me/compute/reservations` | 以当前用户余额原子预留预算和容量 |
| POST | `/api/me/compute/reservations/:reservation_id/release` | 主动取消尚未执行的预留并退款 |
| POST | `/api/me/compute/reservations/:reservation_id/expire` | 到期后终结尚未执行的预留并退款 |

Reserve 请求必须提供稳定 `reservation_id`、消费者幂等键、当前 quoted Job 的 revision/digest、所需 meter 数量和 UTC 到期时间。Release/Expire 必须提供新的终态幂等键及当前 Reservation revision/digest。

## 3. MCP 工具

这些工具加入现有项目级开放商业 MCP：`/api/projects/:project_id/open-commerce/mcp`。MCP 会话同时固定登录用户和项目成员身份，因此只能操作当前项目内属于当前用户的 Job。

| 工具 | 类型 | 作用 |
|---|---|---|
| `compute_get_my_job` | 只读 | 读取本人当前项目的一份 Job |
| `compute_list_my_jobs` | 只读 | 列出本人当前项目的 Job |
| `compute_get_my_reservation` | 只读 | 读取本人当前项目的一份 Reservation |
| `compute_list_my_reservations` | 只读 | 列出本人当前项目的 Reservation |
| `compute_reserve_my_job` | 有副作用 | 原子冻结平台人民币余额并持有容量 |
| `compute_release_my_reservation` | 有副作用 | 主动取消、退款并归还 held 容量 |
| `compute_expire_my_reservation` | 有副作用 | 到期后退款并归还 held 容量 |

`compute_reserve_my_job` 要求 `confirm_financial_action=true`，主动 Release 要求 `confirm_cancellation=true`。这些字段用于防止代理漏掉显式确认步骤，但不能替代宿主向真人展示工具参数和取得批准；工具注解均把写操作标为 destructive、idempotent。

## 4. 推荐调用顺序

1. 先用只读列表或单条读取取得当前 revision、digest 和状态。
2. 向用户展示拟冻结金额、meter、到期时间或取消影响。
3. 用户确认后，用新的稳定幂等键调用 Reserve 或 Release。
4. 网络响应丢失时原样重放；不得生成新 Reservation ID 规避原回执。
5. 发生 revision/digest 冲突时重新读取，不得猜测最新版本。
6. Expire 只能在真实到期后调用，服务端 Store 时间和 Claim 账本时间是最终权威。

## 5. 失败关闭边界

- 登录用户与 Job 消费者不一致时拒绝。
- 项目 MCP 的 Job 不属于当前项目时拒绝。
- Job、Reservation、Offer、Price Snapshot、Claim 或余额历史绑定不一致时拒绝。
- Reserve 不是完整 `reserved` 结果或缺少余额结果时整笔事务回滚。
- Finish 遇到 active Claim 或已经启动的 Attempt 时拒绝。
- 同一 ID 或幂等键只允许相同规范请求重放。
- 只有首次 Reserve 要求未来到期；不可变首次回执在合同到期或预算后来进入终态后仍按历史语义重放，不依赖余额表的可变到期字段。
- 通用余额释放和到期器跳过 Broker 管理的预授权；只有核对精确预授权 ID 的 Broker Finish 可以退款，避免预算、容量、Job 与 Reservation 被拆成单腿终态。
- 当前接口不接受客户端传入用户身份、终态时间、币种或任意退款金额。

## 6. 尚未实现

- Cargo 编译、迁移执行、HTTP/MCP 真实调用与并发验证；
- Job 创建、报价生成和 Offer 自动撮合的公共控制面；
- Attempt 激活、续租、取消、fencing 与节点派发；
- 运行中任务的容量归还、实际用量、验证和最终结算；
- 多币种、Sui 资产、外部矿池和 Provider 收益提现；
- 服务器持久化的独立真人确认凭证。
