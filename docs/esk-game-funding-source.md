---
version_status: current
reviewed_at: 2026-09-12
implementation_status: implemented
---

# 游戏奖励备付来源 API

本接口让独立备付执行器从主项目的真实预算账本读取来源。它不接收收益数字、签名私钥或交易 BCS，不改变占用，不发送资金。政策仍由 `ELON_GAME_REWARDS_CONFIG` 配置，首版只支持 testnet。

两个接口均为 POST JSON，必须走正式 TLS 入口并持有有效主项目管理员/owner 会话的 Bearer token；Paper、静态 owner、普通玩家、过期或撤销会话不能访问。有 Origin 时必须同源；拒绝查询参数和未知 JSON 字段，继承 16 KiB 请求上限及 `Cache-Control: no-store`。请由独立服务保存会话凭据，不写入命令行、URL、浏览器存储或日志。

## 精确查询

`POST /api/admin/game-rewards/v1/budgets/source`

```json
{
  "schema": "esk.game.rewards.funding-source.request.v1",
  "policy_digest": "<当前完整政策的 64 位小写十六进制摘要>",
  "allocation_hash": "<已预留预算的 64 位小写十六进制编号>"
}
```

响应 `esk.game.rewards.funding-source.v1` 包含 `policy`、`policy_digest`、`observed_at_ms`、`source` 和恒为 false 的 `offchain_payment_authorized`。

`source` 包含完整 `budget`（复用预算登记响应）、`original_settlement` 与 `latest_settlement`（均保留签名）、`latest_report_digest`、`cumulative_net_profit_units`、`reserved_profit_units` 和 `source_status`。所有金额均为当前政策指定的精确资产类型的最小单位十进制字符串，不表示 USDT/ESK 等价或浮动汇率报价。

| source_status | 含义 | 执行器动作 |
|---|---|---|
| reserved | 存在已预留预算，当前累计净收益覆盖该用户全部已预留金额，用户 active | 可继续准备交易；签名前仍须重新查询并验证链上状态 |
| user_inactive | 尚未备付，预算用户已停用 | 停止创建新付款，保留原占用 |
| profit_deficit | 尚未备付，最新净收益不足以覆盖全部占用 | 等待真实核算/备付处置；不能自动减少用户权益或用本金填补 |
| funding_attested | 主项目已保存并复验独立观察者的初始备付证据 | 按现有 budget_id 对账；不能再次创建预算 |

`reserved` 是账本来源状态，**不是付款授权**，也不证明项目金库有足额可花费币。后续亏损不会修改已确认预算的历史备付状态；当前链上余额应向独立观察者查询。

## 待备付扫描

`POST /api/admin/game-rewards/v1/budgets/pending`

```json
{
  "schema": "esk.game.rewards.pending-funding.request.v1",
  "policy_digest": "<当前政策摘要>",
  "after_allocation_hash": null,
  "limit": 20
}
```

`limit` 必填且为 1–20。游标可省略或 null；非空时必须是完整小写摘要。响应 `esk.game.rewards.pending-funding.v1` 包含相同政策/观察时间、`sources`、`next_after_allocation_hash` 和 false 的 `offchain_payment_authorized`。按 allocation_hash 升序查询，已确认备付记录排除；被停用或利润不足的记录仍返回明确阻塞状态。空页正常返回空数组。

有下一页时将 `next_after_allocation_hash` 原样用于下一请求；null 表示本轮扫描结束。各页是独立快照，扫描期间可能新增或确认预算，因此完成一轮后重新从 null 扫描。游标不是消费确认，也不是跨重启的永久增量水位；执行器必须以 allocation_hash 去重和追溯具体预算。

## 一致性与恢复

单次请求在有界的 `BEGIN IMMEDIATE` 事务中读取，以便与撤销会话、核算写入顺序一致。事务开始和结束检查管理员有效性及政策，超过 5 秒或系统时间回退失败关闭；查询不写任何账本记录。`observed_at_ms` 是读事务开始时间，不是可携带的长期授权。

服务端重算意图哈希和金额、核对 SQL 索引与意图关系、重新验证原始及最新结算签名和账户/钱包归属关系；已备付记录同时复验观察者签名。未知预算返回 404，不规范字段 400，身份失败 401，政策变化 409，不可用存储/证据失败关闭。不能凭调用者的 `verified: true` 或任意 JSON 获得来源。

后续执行器需要：固定同一政策与 Sui 创世/包/registry/完整资产类型；对真实资金 Coin 和固定 allocate PTB 做校验、模拟及签名；**发送前持久化准确交易摘要和原始签名交易**；未知结果只恢复同一交易；独立观察者从已确认链上 BCS 产生证据，再调用 `confirm-funding`。当前源查询只完成主项目侧读取环节，不替代这些未接入步骤。链上同一 registry 的 allocation_hash 唯一约束也必须保留。

普通玩家仍使用 `/api/me/game-rewards/v1/account`。原来已预留的受益地址不可被查询接口修改；更换钱包不得静默把既有预算重定向给新地址。真实收益接入与最低本金兑付保障仍需要独立实现和实际资产准备，不能用该接口宣称已经完成。

## 验证记录（2026-09-12）

独立 `game-rewards-harness` 19 项通过，指纹 `cd5d6fe9e0a5c14445961e1472db93d85e5fbed657033ff731735baa13e8d030`；包括 7 项新增来源测试。正式 `elon-server` 的 `game_rewards` 20 项定向回归通过，2455 项未运行，指纹 `dcc4b8f0e3132a7d3d687509226aeda93b7158a74f5ea3f105e53a7c998c6099`。这些测试复用生产 SQL、签名验证、真实 Store 和受保护路由，但不代表公开测试网或真实资金验收。

`check-source-size` 检查 8 个源文件通过；文档模块化检查通过。初次 SQL 编译因测试调用当前 rusqlite 没有的 `total_changes` 方法失败，修为 SQL `SELECT total_changes()` 后重跑通过；没有把失败计入成功。后端仍有既有编译警告，此批没有宣称全仓无警告。
