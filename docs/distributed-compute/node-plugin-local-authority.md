---
title: 节点插件本机权威库与下载栅栏
status: current
reviewed_at: 2026-08-04
owners: node, security
---

# 节点插件本机权威库与下载栅栏

本文固定 Compute Bootstrap 在节点本机的安全真源。它承接 signed Manifest、signed InstallPlan 与插件生命周期合同，但不等于下载器、Sidecar、沙箱或云端调度已经可用。

## 1. 选择与边界

节点使用独立的 `%APPDATA%\elon-node-agent\compute-plugin-state.sqlite3` 作为插件控制状态的唯一权威库。它不写入 `node.json`，也不放进可切换的 `ELON_NODE_DATA_ROOT`：

- `node.json` 继续只保存节点身份与兼容配置，旧客户端重写未知字段时不会破坏插件事务；
- SQLite 在一次 `BEGIN IMMEDIATE` 中协调库存 CAS、计划应用、候选槽所有权、下载游标、可信时间与 keyring binding；
- 插件包、模型和 `.part` 文件以后可以放数据根，但数据库只保存 Bootstrap 生成并校验的相对槽引用，不接受 InstallPlan 提供本机路径；
- SQLite 主库及 WAL/SHM、内容槽和必要回执必须进入节点升级器的耐久状态保留清单；卸载策略另行显式处理。

数据库损坏、迁移失败、keyring 无法建立信任或可信时钟不可用时，插件子系统失败关闭：不下载、不应用计划、不生成 `ReadyCapability`。legacy LLM 和普通 NodeAgent 可以继续启动，但不能把空库存当成恢复结果。

## 2. 单一事务真源

最小逻辑表如下；实际列可拆分，但不能形成第二套真源：

| 表 | 关键事实 |
|---|---|
| `authority_meta` | schema、安装身份、全局 state revision、inventory revision、共享开关/binding、authority epoch、策略/档案/catalog/keyring binding、可信时间高水位与时钟状态 |
| `plugin_records` | 插件安装、激活、准入、运行与当前 active/candidate 指针 |
| `plugin_slots` | release、内容摘要、阶段、候选 token/代次及 owner plan binding |
| `keyring_bundles` | 经 Bootstrap 根公钥验证的原始 signed bundle、用途 revision/digest、有效期与激活事实 |
| `keyring_keys` | publisher/control 用途、主体、key ID、公钥指纹、有效期、状态与撤销事实 |
| `plan_applications` | 不可变 signed plan、signed manifests、准入绑定、应用 inventory revision 与幂等回执 |
| `plan_events` | prepared/applied/canceled/failed 等追加式恢复和审计事实 |
| `planned_downloads` | ordinal、工件 binding、长度、committed offset、cursor generation 与状态 |
| `fetch_claims` | claim ID、authority epoch、cursor generation、range、redirect generation、状态与时间 |

启动配置复用节点现有 SQLite 模式：有界 `busy_timeout`、WAL、`synchronous=FULL`、外键和 `BEGIN IMMEDIATE`。数据库打开与恢复发生在 NodeAgent 取得实例锁之后、Plugin Host 可接收任务或上报 capability 之前。

## 3. Keyring 信任启动

磁盘里的普通 JSON 或 SQLite 公钥不是信任根；本机可写状态若能自行替换公钥，攻击者即可自签 Manifest 和 InstallPlan。因此：

1. Bootstrap 只内置离线根的公钥 pin，根私钥永不进入仓库、节点或环境变量；
2. 磁盘保存 root-signed JCS keyring bundle，使用独立签名 domain；
3. bundle 同时携带单调 `bundle_revision`、Publisher 与 Control 各自的 revision/digest、生成/失效时间，以及带 purpose、主体、状态、有效期、撤销时间和 SHA-256 指纹的 Ed25519 key；
4. 整包拒绝未知字段、非规范 UTC/Base64、重复 identity、重复指纹和跨用途/跨主体公钥复用；指纹必须由公钥重新计算；
5. resolver 只返回与可信时间、预期 revision/digest、purpose、publisher 和 key ID 全部吻合的 active key；缺包、过期、回滚、撤销、损坏和错误用途均失败关闭；
6. keyring 替换必须先验根签名与整包不变量，再以 revision/digest CAS 原子提交，commit 后才发布内存快照。

当前仓库没有生产 Compute 根公钥。代码可以先形成 DTO、验证接口和可注入的 Bootstrap root resolver，但在真实 root pin 进入受信客户端发布前必须保持运行路径未接线；禁止用测试 key、任意 AppData key 或环境变量 fallback 冒充生产信任。

InstallPlan 必须绑定 Publisher 与 Control 两类 keyring 的 revision 和 digest。首次准入与每次下载认领都使用同一权威快照；任何一类 keyring 变化或本次已验签 key 被撤销，都在下一次取数前停止。

## 4. 计划应用与候选所有权

计划应用按以下单事务顺序执行：

1. `BEGIN IMMEDIATE` 后重读共享授权、策略、节点档案、catalog、keyring 和库存；
2. 从数据库事实重建严格 `ComputePluginInventorySnapshot`，计算 JCS SHA-256；
3. 核对 `expected_inventory_revision` 与 digest，重新验签 plan/manifests 并执行全量准入；
4. 克隆旧状态，构造完整下一状态并再次校验；
5. 同事务写入不可变应用、候选所有权、下载游标和库存，inventory revision 精确加一；
6. commit 成功后才发布内存快照或允许下载。

同一 `plan_id + plan_digest` 重放返回原应用回执，不再递增 revision；同一 plan ID 携带不同 digest 永久冲突。任何错误都不改变数据库、内存或网络状态。

`last_plan_id` 只供审计，不能授权候选槽。每个 downloading/verifying/staged 槽必须恰好由当前 candidate 指针引用，并绑定本机随机 candidate token、owner plan ID/digest、application inventory revision 和单调 candidate generation。普通新计划不能接管既有候选；`cancel_candidate` 必须精确绑定完整候选所有权。active 槽切换只在内容完整验证和健康门禁通过后原子换指针并增加 activation generation。

## 5. 三段式下载认领

旧的单调用 `claim_fresh_segment` 不能覆盖写文件后的崩溃窗口，也可能在完整授权校验前消耗游标。权威接口固定为：

```text
begin_claim -> write + flush/fsync .part -> commit_segment
       \-> abort/revoke
```

### begin_claim

事务内重新核对 plan window、两类 keyring binding 与实际签名 key、共享授权、策略、节点档案、catalog、应用状态、候选 owner、ordinal 和工件 binding，并要求：

- `offset == committed_offset`；
- `0 < length <= min(16 MiB, remaining)`；
- 每个工件最多一个 active claim；
- redirect hop 在同一 transfer generation 内严格递增，redirect 本身不推进 byte cursor；
- claim 绑定随机 claim ID、cursor generation、authority epoch 与当前进程 owner epoch。

只有所有检查通过后才追加 prepared claim。返回值不是可跨请求复用的下载授权。

### commit_segment

下载器先把 bytes 写入 `.part` 并执行 flush/fsync，再用 claim ID 和 fencing generation 提交。事务再次复核全部权威事实、文件事实与 fence，随后把 committed offset 精确推进到 claim end 并终结 claim。共享关闭、key 撤销、计划过期、owner 改变或旧 generation 的提交均失败。

### abort、崩溃与完整工件

abort/revoke 不推进 cursor；重试创建新 generation 并仍从 committed offset 开始。恢复时文件长于 cursor 则截断未提交尾部，短于 cursor 则把候选标为损坏。完整工件重新计算全量 SHA-256 后才能进入 verifying；不持久化跨版本不可移植的 SHA 内部状态。

## 6. 共享关闭与下一字节边界

关闭共享在同一事务中：

- 写入 `sharing_enabled=false` 并增加 authority epoch；
- 撤销所有 active fetch claim；
- 停止所有 candidate，不遗漏“desired 已 disabled 但仍在 downloading”的记录；
- 把期望激活态推进为 disabled/draining；
- 阻止未提交 segment 的 commit。

下载器除每段、每次 redirect 前查库，还订阅进程内 cancellation epoch，并在每次 socket read 和文件 write buffer 前检查。这样“下一字节停止”不被 16 MiB 分段放宽；关闭后的未提交尾部在恢复时截断。

## 7. 可信时间与恢复

可信时间取 `max(墙钟, 启动基线 + monotonic elapsed, persisted high-water)`，每次 plan 应用、claim 和 commit 都在同一事务推进 high-water。重启后若墙钟显著落后高水位，状态变为 `clock_untrusted` 并阻断下载，直到经认证的服务端 time attestation 刷新；不能仅停在旧高水位，否则反复重启可能延长计划寿命。

恢复顺序固定为：

1. 取得 NodeAgent 独占实例锁；
2. 打开数据库、执行有界迁移和完整性检查；
3. 建立可信时钟并递增进程 owner epoch，栅栏旧 claim；
4. 用当前 root/keyring 重验未终结 plan/manifests；
5. 对齐 fetch claim、文件长度与 committed cursor；
6. 对齐 candidate 指针、槽 marker 和内容摘要；
7. 只恢复 owner、授权、期限和 keyring 全部未漂移的工作，其余撤销并等待显式清理。

恢复不得自动接管孤儿候选、自动激活、静默清空库存或根据旧内存快照继续下载。

## 8. 当前实现状态

本合同已接受。当前已有严格网络 DTO、JCS/SHA-256/Ed25519 验签、Manifest 校验、InstallPlan 首次准入及旧的逐段权威 trait；专用 SQLite schema/store、root-signed keyring bundle、双 keyring binding、原子计划应用、候选 token、三段式 fetch claim、可信时间和启动恢复仍待分阶段形成代码并接线。所有新代码在统一验证阶段前继续标记 `implementation_uncompiled`。
