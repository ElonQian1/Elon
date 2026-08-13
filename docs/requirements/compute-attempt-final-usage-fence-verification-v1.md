---
status: current
owner: ai-economy
reviewed_at: 2026-08-13
review_interval_days: 90
---

# Attempt 最终声明用量栅栏动态验收 V1

## 目标

动态验证既有 V226 最终声明用量栅栏，确保 Provider 为 running Attempt 创建终态候选后，同一 Lease 的累计声明流永久封口，且消费者审核、平台观测、Verification、Execution Receipt、可信终态与待结算链不能消费已经不是当前流头的候选。

## 范围

- 编译并运行 V226 迁移、Store 封口、精确重放和候选 currentness 代码。
- 使用完整 Provider、Offer、Job、Reservation、Capacity Claim 与 running Lease 夹具验证真实 SQLite 状态链。
- 覆盖候选后精确重放、新序号拒绝、候选与新用量竞争、旧库漂移迁移拒绝及下游失败关闭。
- 仅在动态测试暴露缺陷时修改 V226 实现，保持 v188/v189 摘要与历史行不变。

## 非目标

- 不开放被关闭的人工 Start、Renew 或 no-start Abort 入口。
- 不接线真实 Attempt Gateway、节点插件、外部矿池 Adapter 或生产路由。
- 不生成可信计量、自动 Verification、Execution Receipt、容量消费、退款、Provider 收益、提现或链上结算。
- 不运行真实节点、浏览器、生产数据库或长时压力测试。

## 验收标准

1. V226 随全量迁移在全新文件数据库中成功安装，两条数据库 trigger 唯一存在。
2. running Lease 可追加声明并创建精确绑定当前流头的唯一终态候选。
3. 候选后既有用量和候选请求可精确重放，新幂等键、新序号或不同内容均失败关闭。
4. 两个独立 SQLite 连接竞争“追加新用量”和“创建候选”时最多一个方向成功，最终状态不存在候选落后于用量流头的情况。
5. 停在 V225 的旧库若候选未绑定当前流头，V226 迁移失败且不安装任一 trigger；一致旧库可迁移。
6. 人工构造的历史漂移在候选读取及下游审核入口失败关闭，不新增审核或经济事实。
7. 定向 Rust 测试、源码大小、文档模块化和格式检查通过，并记录当前代码与测试证据。

## 完成边界

通过本验收后只能宣称 V226 在本地 Rust/SQLite 动态链路中完成验证。真实 Gateway、生产数据库升级、节点派发、真实计量、浏览器和资金结算仍保持未实现或未验收状态。
