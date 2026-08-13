---
title: 分布式算力 Attempt 最终声明用量栅栏权威
status: current
reviewed_at: 2026-08-12
owners: backend, node, ai-economy
implementation_status: verified_rust_sqlite
---

# 分布式算力 Attempt 最终声明用量栅栏权威

## 1. 目标与当前状态

本权威冻结 v226 的最窄修复合同：v189 Provider 终态候选必须原子绑定当时最新的 v188 累计声明用量；候选一旦存在，同一 Lease 的声明用量流即被封口。之后只允许对候选前已经保存的声明做精确幂等重放，不允许追加任何新序号。

v226 迁移、Store 写入封口、模板门卫和统一候选 currentness 已完成 Rust/SQLite 动态验收。定向测试覆盖全量新库迁移、精确重放、候选后封口、两个独立连接竞争、legacy 一致历史迁移、legacy 漂移拒绝和历史漂移阻断下游审核。真实 Gateway Adapter 尚不可构造，HTTP/TCP、PC、节点、生产数据库原位升级和发布仍未验收。

## 2. 被修复的完整性缺口

现有 v189 写入会读取当时最新的 v188 快照，但候选不改变 running Lease，也没有阻止 v188 后续追加。Provider 因而可能在候选创建后写入更高序号，而 v190-v195 仍沿候选固定的旧序号读取历史快照。

这会破坏“候选中的 final usage 是该 Attempt 最终 Provider 声明”的含义。即使后续证据自身摘要都正确，验证和结算链也可能基于已经不是流头的旧声明。

## 3. 唯一线性化合同

同一 Lease 的合法事件顺序只有：

1. running Lease 可追加严格递增、累计不回退的 v188 声明；
2. v189 在一个 `BEGIN IMMEDIATE` 事务内读取当前流头并登记唯一终态候选；
3. 候选提交成功即永久封口该 Lease 的 v188 流；
4. 候选后可读取历史声明，也可精确重放候选前已经成功的声明请求；
5. 候选后任何新幂等键、新序号或不同请求均失败关闭。

候选不会关闭 Lease、Job、Reservation 或 Claim。这里的“封口”只约束 Provider 声明用量流，不代表平台已经观察、验证、消费或结算该用量。

## 4. v226 数据库门卫

v226 不建新表，也不改写 v188/v189 历史行。迁移必须在一个 `BEGIN IMMEDIATE` 中完成以下步骤：

- 先检查每份既有候选是否精确绑定同 Lease 当前最高序号的用量声明；
- 检查候选和声明的 snapshot、sequence、usage digest、Provider、消费者、Lease、fencing、Job、Reservation 与 Claim 投影是否完全一致；
- 发现任何既有漂移时拒绝迁移，不能把不一致历史静默祖父化；
- 安装 v188 `BEFORE INSERT` 门卫：同一 Lease 已有候选时拒绝新声明；
- 安装 v189 `BEFORE INSERT` 门卫：候选必须精确绑定当前最高序号声明；
- 只有检查与两条门卫全部成功后才提交迁移。

`IMMEDIATE` 写锁保证“检查最新声明”和“封口流”与并发 v188 写入串行化。应用层检查只提供清晰错误，不能代替数据库门卫。

## 5. Store 写入与重放

v188 写入顺序固定为：

1. 先按 Provider 幂等作用域查找既有声明；
2. 再按 Lease 与序号查找既有声明；
3. 只有两种精确重放都未命中时，才检查 Lease 是否已有 v189 候选；
4. 已有候选则拒绝新写；否则继续既有 Lease、业务因果、meter 与单调性审计。

因此候选不会使已经成功的请求失去重放能力，但也不能借相同序号或幂等键夹带不同内容。

v189 的首次写入与两条重放路径都必须在返回前重新确认候选固定的声明仍是当前流头。

## 6. 读取与下游继承

所有 v189 候选读取统一经过一条 final-usage currentness 审计：

- 读取该 Lease 当前最高序号的 v188 声明；
- 精确比较候选保存的最终快照及完整业务因果投影；
- 只有仍为同一流头时才返回候选。

v190 消费者审核、v191 平台观测、v192 Verification、v193 Execution Receipt、v194 可信终态和 v195 Settlement 都通过该候选读取入口继承门卫。历史数据库若存在漂移，读取和新下游写入必须失败关闭，不能继续沿旧 final usage 推进经济效果。

## 7. 不变的信任与经济边界

v226 不改变既有证据级别：

- v188 仍为 `unverified_provider_declaration`；
- v189 仍为 `candidate_only`；
- 不生成平台观测、Verification 或 Execution Receipt；
- 不推进 Lease、Job、Reservation 或 Capacity Claim；
- 不消费或归还容量；
- 不扣款、退款、登记收益、释放余额或调用外部支付。

“final”只表示 Provider 声明流已封口，不表示该数值真实、完整、经过签名或已被平台接受。

## 8. P0 禁线

- 不新建第二套 usage、candidate、verification 或 settlement 账本；
- 不修改既有 v188/v189 摘要算法或不可变历史行；
- 不以 Lease 仍为 running 为由允许候选后的迟到声明；
- 不把 exact replay 误实现为再次 INSERT；
- 不静默忽略 legacy 漂移，也不在 pending 队列中继续消费漂移候选；
- 不借本修复引入节点签名、真实计量、自动 Verification、容量或资金效果。

## 9. 动态验收与后续验证

2026-08-13 已通过 5 项定向 Rust/SQLite 测试：

- 新库全量迁移只安装一组双向 trigger；
- 候选绑定当前流头，候选后的原请求可精确重放，新幂等键、新序号和不同内容失败关闭；
- 两个独立 SQLite 连接竞争追加用量与登记候选时只有一个方向成功，不产生落后流头的候选；
- 一致 legacy 历史可安装 v226，漂移历史拒绝迁移且不留下半安装 trigger；
- 人工构造的历史漂移使候选读取和消费者审核失败，审核表保持零写入。

验证命令为 `scripts/validate-rust.ps1 ... compute_attempt_final_usage_fence_tests -- --test-threads=1`，指纹为 `4c63c04f42aae96180be0ec3d53e93e4f4d6789d18f8ee828ea9e1353ea2b32a`。测试仅为建立 V226 前置 running Lease 临时移除 v211 Gateway acceptance trigger，并在进入 V226 被测阶段前立即幂等重装；这不证明真实 Gateway、Adapter ACK 或生产派发已实现。

仍需后续独立验证 HTTP/TCP、PC、节点、真实 Gateway/Adapter、生产数据库备份恢复与原位升级、异常断电和发布链路。

## 10. 实现入口

- `server/src/store/compute_attempt_usage.rs`
- `server/src/store/compute_attempt_terminals.rs`
- `server/src/store/compute_attempt_terminals/final_usage.rs`
- `server/src/compute_attempt_terminal_migration/final_usage_fence.rs`
- `server/src/store_migrations.rs`
