---
version_status: current
reviewed_at: 2026-09-12
implementation_status: planned
---

# 游戏奖励备付来源查询

## 目标与边界

主项目已原子登记经签名核算的收益和预算，但外部备付执行器尚不能按预算唯一编号取得可信来源，只能依赖最近 100 条用户视图或操作者导出的 JSON。本批增加真实管理员、TLS 保护的只读来源接口，供后续独立 Sui 执行器复用。

查询返回当前政策、原始结算签名、精确预算意图、当前累计净收益和全部已占用金额；同一 SQLite 读事务内检查当前管理员会话、用户状态和政策。结果只是来源快照，绝不成为链上付款、私钥访问或资金已到账的证明。

## 验收

- 按 allocation_hash 和预期 policy_digest 精确查询，重算意图哈希、金额并重新验证原始结算签名；未知、被篡改、跨政策数据失败关闭。
- 管理员待备付列表使用有界 keyset 分页，最多 20 条；包含被停用用户的阻塞状态，已经确认备付的预算不进入列表。下一轮从头扫描，不能把末页视为永久消费游标。
- 单次返回的数据来自同一读事务，会话在事务开始和返回前都有效；普通用户、Paper 身份、撤销或过期管理员被拒绝。
- 显示原始签名来源和最新签名结算；发生后续亏损或账户停用时明确阻塞，不更改原来的占用或已备付资金。
- 已确认预算可按编号追溯，并继续验证独立观察者签名；来源响应始终 offchain_payment_authorized=false。
- JSON 拒绝未知字段，空或不规范游标、超出范围的页大小、伪造政策及跨来源请求被拒绝；错误与成功均 no-store。
- 独立 SQL harness 覆盖分页、鉴权、签名、亏损、只读性和过期；真实 TLS 路由回归覆盖接入。

## 实现计划

| 文件 | 职责 | 预算 |
|---|---|---|
| server/src/esk_platform/game_rewards/funding_source.rs | 来源模型、只读快照与分页 | 300 行 |
| server/src/esk_platform/game_rewards/funding_source_api.rs | 真实管理员路由适配 | 110 行 |
| server/src/esk_platform/game_rewards/funding_source_tests.rs | SQL 与协议回归 | 350 行 |
| server/src/esk_platform/game_rewards/mod.rs | 模块及路由接入，原 39 行 | +20 行 |
| server/src/esk_platform/game_rewards/api.rs | 复用响应映射，原 139 行 | +0 行 |
| server/src/esk_platform/game_rewards/tests.rs | 复用合成测试 fixture，原 569 行 | +0 行 |
| server/src/esk_platform/game_rewards/http_tests.rs | 现有真实路由验收，原 200 行 | +90 行 |
| server/tests/game-rewards-harness/src/lib.rs | 测试入口，原 19 行 | +6 行 |
| docs/esk-game-funding-source.md | 调用合同与恢复边界 | 90 行 |
| docs/esk-game-rewards.md | 链接新入口，原 45 行 | +3 行 |

本批不迁移数据库、不改账本金额、不发送链上交易、不替代真实交易利润适配器。后续独立执行器负责完整交易校验、发送前持久化、唯一交易恢复及独立观察者回执；当前链上 allocation_hash 唯一性仍是阻止同一预算重复创建的最终约束。
