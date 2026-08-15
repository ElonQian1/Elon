---
title: 开放商业 ERP 写入与回读验证处理器 V1 验收
status: current
reviewed_at: 2026-08-15
implementation_status: verified_sdk_contract
---

# 开放商业 ERP 写入与回读验证处理器 V1 验收

## 验收范围

本批在 `@elon/open-commerce-connector` 内新增厂商无关的 ERP 处理器，并复用现有
`createAdapterHandoffWorker`。平台 Claim 仍负责租约、续租、完成和释放；新处理器只负责把终态
业务证据转换为稳定来源信封，调用商户插件写入，再通过独立回读结果决定是否允许 `applied`。

## 已验证行为

1. 处理器要求活动 V1 Claim、正数且一致的尝试号、Invocation 级幂等键、成功商户运行时证据、
   有效标准业务回执、可用结果、64 位结果摘要和 `funds_moved=false`。
2. `task.result._yilong_business_receipt` 必须与证据投影一致；不一致时在调用 ERP 前失败关闭。
3. 不可变来源信封包含项目、商户、Invocation、数据接入、机器凭据与版本、结果摘要、标准业务
   回执、稳定幂等键和来源摘要，不包含 Claim ID、租约密钥、机器 Token 或生产凭据。
4. 相同 Invocation 在 Claim ID、尝试次数和机器凭据版本变化后仍产生相同 `sourceDigest`；完整
   信封保留当前凭据版本，供 ERP 幂等重放时更新来源元数据。
5. `apply` 返回目标记录号后必定调用 `readBack`。回读目标记录号及来源信封中项目、商户、
   Invocation、数据接入、凭据版本、结果摘要或来源摘要任一变化都会阻止 `applied`。
6. 回读不匹配、ERP 不可用和普通异常继续由 worker 作为临时故障释放租约；插件显式抛出的
   `AdapterHandoffRejectError` 沿用既有永久业务拒绝路径。

## 验证证据

| 验证 | 结果 |
|---|---|
| `node --test test/verified-erp-handoff-handler.test.mjs` | 23 项通过，覆盖正常写入/回读、稳定幂等、非法证据、8 类回读篡改、临时释放和业务拒绝 |
| `npm test` | SDK 全量 88 项通过，既有连接器、运行时、衔接 worker、Sui 链外预检和新处理器无回归 |
| TypeScript 5.9.2 严格 `--noEmit` | `verified-erp-handoff-handler-types.ts` 通过；处理器可直接赋给现有 worker 的 handler |
| 临时类型工具清理 | 工作树内 `node_modules` 已删除，未提交 package lock 或独享依赖缓存 |

## 事实边界

- 测试中的 ERP 是注入式内存替身；没有连接生产数据库、咖啡商户服务器或第三方 ERP。
- SDK 强制调用 `readBack` 并核对其返回值，但不能证明插件确实查询了外部持久化系统；生产验收
  必须在目标 ERP 上独立读取同一记录，必要时再增加第三方签名或可信执行证明。
- 本批没有实现美团、抖音、京东、淘宝闪购、支付、履约、退款、财务记账或营销发布适配器。
- 所有来源信封固定 `fundsMoved=false`，不得解释为真实资金移动或链上结算。

## 下一步

选择一个受控商户 ERP 插件，保存完整来源信封并实现真实数据库 `upsert + readBack`，使用测试
机器凭据跑通“领取 -> 幂等写入 -> 独立回读 -> applied -> 消费者订单闭环读取”。该验收仍应
保持订单未支付，生产凭据、官方平台授权和真实资金另行审批。
