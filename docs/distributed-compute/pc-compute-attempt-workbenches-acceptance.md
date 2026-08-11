---
title: 算力 Attempt 与结算 PC 工作台静态验收
status: current
reviewed_at: 2026-08-11
owners: pc, backend, ai-economy
implementation_status: implementation_partially_verified
---

# 算力 Attempt 与结算 PC 工作台静态验收

## 1. 结论

算力 Attempt、证据治理和内部结算 PC 工作台已通过跨层静态合同、全前端严格类型、lint、生产构建和 bundle budget。该结论证明页面能随当前 PC 前端构建、角色入口和 HTTP 路径没有静态漂移，不证明服务端操作级行为已运行、真实节点已接单、真实资金已支付或页面已在登录浏览器中联调。

本批同时关闭三个已知无效的人工作入口：Provider PC 页面不再调用旧的 Start、Renew 和 no-start Abort POST。后端已将这些入口固定失败关闭，未来只能由认证 Attempt Execution Gateway、durable command、ACK/event 与 service actor 推进。

## 2. 已覆盖工作台

参与者入口：

- `/compute-execution`：Provider 读取履约队列、Lease、累计用量模板和终态候选；
- `/compute-reviews`：消费者审核 Provider 终态候选；
- `/compute-challenges`：消费者提出或撤回内部结算挑战；
- `/my-compute-settlement`：Provider 读取内部账户、申请或取消内部提款保留。

平台管理员入口：

- `/compute-observations`、`/compute-verification`、`/compute-receipts`；
- `/compute-finalization`、`/compute-settlement-issuance`；
- `/compute-challenge-resolution`、`/compute-corrections`；
- `/compute-settlement`：到期释放批次、平台账户与提款终态登记。

管理员入口继续由 `admin/owner` 平台角色控制。PC 页面不提供 actor 输入，也不能把参与者页面提升为管理员写入口。

## 3. Gateway 与资金边界

- 待激活 Reservation 只读展示“等待 Gateway”，不会调用人工 activation；
- Lease 的人工续租和人工 no-start 中止固定禁用，不再让用户触发必然失败请求；
- Provider 累计用量仍显示为 `provider_declared`，终态仍显示为候选；
- v194 finalization 不结清消费者预授权，不生成 Provider 收益；
- v195 只操作平台内 CNY 账本，Provider 收益先进入 `pending`；
- challenge/resolution 本身不退款、不移动余额；v199 只纠正平台内余额；
- `available`、`withdrawn` 和 `external_paid_attested` 都不是银行、钱包或链上付款证明；外部付款终态只登记既有付款的脱敏证据声明。

## 4. 验证证据

```powershell
cd pc-frontend
npm run test:compute-attempt
npm run typecheck
npm run lint
npm run build
npm run check:bundle-budget
```

- 专项合同：通过，覆盖 12 条 PC 路由、参与者/管理员导航、9 组队列 API 与服务端路由、Gateway 失败关闭和资金边界；
- TypeScript：通过；
- ESLint：通过，零 warning；
- Vite 生产构建：通过，12 个页面均产出独立页面 chunk；
- bundle budget：通过。

## 5. 未验证边界

- 未运行 v188-v201 每个写操作的独立 Store/Service/HTTP 正向与并发专项；
- 未验证真实 TCP、登录浏览器交互、视觉验收、生产数据库原位升级或发布；
- 没有可生产使用的 v213 route、credential verifier、Adapter/worker、认证 ACK/event、Runner 或 accepted Gateway producer；
- 未执行真实银行、支付机构、钱包或 Sui 付款，也没有提现执行器；
- 静态页面可构建不代表队列中已有真实业务数据，不能宣称完整算力交易已可生产运行。
