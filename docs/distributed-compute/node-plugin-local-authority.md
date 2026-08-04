---
title: 节点插件本机权威库与下载栅栏
status: current
reviewed_at: 2026-08-05
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

| 持久化载体 | 关键事实 |
|---|---|
| `authority_meta` | schema、安装身份、全局 state revision、inventory revision、共享开关/binding、authority epoch、策略/档案/catalog/keyring binding、可信时间高水位、时钟状态，以及规范化的 inventory v2 JSON/digest |
| inventory v2 内的逻辑 `plugin_records` / `plugin_slots` | 插件期望存在性、安装、激活、准入、运行、active/candidate 指针，以及 slot release/阶段；当前没有同名关系表，不能把逻辑模型误写成物理表 |
| `keyring_bundles` | 经 Bootstrap 根公钥验证的原始 signed bundle、用途 revision/digest、有效期与激活事实 |
| `keyring_keys` | publisher/control 用途、主体、key ID、公钥指纹、有效期、状态与撤销事实 |
| `keyring_seals` | 两类 key 数量完成后的不可变封存点；封存后禁止为历史 bundle 追加 key |
| `plan_applications` | 不可变 signed plan、signed manifests、准入绑定、应用 inventory revision 与幂等回执 |
| `plan_application_seals` / `plan_events` | 完整 request、准入、子行与回执的不可变封存点；当前写入 typed `applied` 事件，后续事件仍须连续追加 |
| `candidate_owners` | 本机随机 candidate token、插件/槽/release、单调代次、grant、owner/closing plan 与终态 |
| `planned_downloads` | ordinal、工件 binding、长度、committed offset、cursor generation 与状态 |
| `fetch_claims` | claim ID、authority epoch、cursor generation、range、redirect generation、状态与时间 |
| `candidate_verification_runs` | candidate 级 verification ID/generation、完整工件集摘要、本机 pinned file-set binding、authority/process fence、prepared/terminal 状态与不可变结果；当前 schema 只开放撤销，验证终结尚不可达 |

启动配置复用节点现有 SQLite 模式：有界 `busy_timeout`、WAL、`synchronous=FULL`、外键和 `BEGIN IMMEDIATE`。数据库打开与恢复发生在 NodeAgent 取得实例锁之后、Plugin Host 可接收任务或上报 capability 之前。

## 3. Keyring 信任启动

磁盘里的普通 JSON 或 SQLite 公钥不是信任根；本机可写状态若能自行替换公钥，攻击者即可自签 Manifest 和 InstallPlan。因此：

1. Bootstrap 只内置离线根的公钥 pin，根私钥永不进入仓库、节点或环境变量；
2. 磁盘保存 root-signed JCS keyring bundle，使用独立签名 domain；
3. bundle 同时携带单调 `bundle_revision`、Publisher 与 Control 各自的 revision/digest、生成/失效时间，以及带 purpose、主体、状态、有效期、撤销时间和 SHA-256 指纹的 Ed25519 key；
4. 整包拒绝未知字段、非规范 UTC/Base64、重复 identity、重复指纹和跨用途/跨主体公钥复用；指纹必须由公钥重新计算；
5. resolver 只返回与可信时间、预期 revision/digest、purpose、publisher 和 key ID 全部吻合的 active key；缺包、过期、回滚、撤销、损坏和错误用途均失败关闭；
6. keyring 替换必须先验根签名与整包不变量，再以 revision/digest CAS 原子提交，commit 后才发布内存快照。

本机安装事务还必须强制：`bundle_revision` 只能前进；Publisher/Control ring revision 不能下降；同一 ring revision 永远只能对应同一 digest；同一 bundle revision 只有完整 signed-envelope digest、root key ID/指纹和双 ring binding 全部相同才作为重放。root 轮换或重新签名必须发布更高 bundle revision。bundle、全部规范化 key 与 seal 写完后，才能把 `authority_meta` 指向新 binding；切换时 `state_revision` 与 `authority_epoch` 同时加一。重启加载时从 `signed_bundle_json` 重新用当前发布内置的根公钥验签，并逐字段、逐 key、安装/封存时间对账分解列；SQLite 行本身不是信任根。只有当前活动的持久快照实现 leaf resolver，且快照自身拒绝低于创建时可信时间下界的解析；单纯验根成功的未安装 bundle 和 archived tip 都不能直接充当 Manifest/Plan resolver。安装新 bundle 时允许旧活动 bundle 已经过期，但仍必须通过归档签名与内容完整性检查；客户端 root 集合必须在该次轮换完成前保留验证旧活动 tip 所需的 root。

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

幂等键是 `plan_id`，但不可变身份同时包含 plan digest 与完整 application request digest；后者覆盖 signed plan envelope 和规范排序后的完整 signed Manifest 集。同一请求重放返回原应用回执，不再递增 revision；同一 plan ID 的任一规范字段、JCS envelope 身份或规范排序后的 Manifest 集身份不同都永久冲突。回放会重新对账 v2 inventory、sealed keyring row、typed admission、候选/关闭记录、完整下载身份、event、seal、回执和当前 fencing 状态，而不是只相信单行 JSON 或摘要。

`last_plan_id` 只供审计，不能授权候选槽。每个 downloading/verifying/staged 槽必须恰好由当前 candidate 指针引用，并绑定本机随机 candidate token、owner plan ID/digest、application inventory revision 和单调 candidate generation。普通新计划不能接管既有候选；`cancel_candidate` 必须精确绑定完整候选所有权。`complete` download 只代表耐久字节游标到达签名长度，不代表摘要正确。candidate 只有在完整下载闭包形成唯一 verified artifact-set run 后才能进入内容验证；安全解包、无额外文件的 Manifest closure、逐文件摘要、预热和 candidate health 仍是后续门禁。active 槽切换只能消费完整内容与健康回执，原子换指针并增加 activation generation；authority v3 在这些回执尚未实现前直接阻断 `owned -> promoted`。

当前 reducer 对 action 的耐久语义固定为：install/upgrade 创建新 candidate owner 与精确下载闭包；keep/disable 更新期望存在/激活与审计字段，但不创建候选或下载；remove 写入 `desired_presence=absent`，仅在已停止、零 attempt 且无 candidate 时立即把槽推进 removing，否则保留明确的待卸载意图；cancel_candidate 关闭精确 owner、撤销 prepared claim、取消未完成下载并把候选槽推进 removing。库存最多 256 条，投影生成第 257 条时整笔事务失败。每次首次 applied（非 replay）都推进 authority epoch，并在封存前撤销旧 epoch 的 prepared claim；任何残留 prepared claim 都阻止 seal。

PlanApply 只形成 authority SQLite 事实、候选私有句柄和 sealed execution capsule。除 authority 数据库自身外，它不访问网络或插件工件文件系统，不下载、解压、激活插件，也不启动 Sidecar；这些副作用只能由后续重新取权的执行层产生。

## 5. 三段式下载认领

旧的“先 claim、后校验”单调用不能覆盖写文件后的崩溃窗口，还会在拒绝路径遗留 prepared claim。取数入口先从进程内 cancellation source 捕获不可刷新的 guard，再执行无副作用 `read_fresh_segment_authority`，校验 plan/live/inventory/key/candidate/cursor 全部事实，最后用 `claim_validated_segment` 原子复读同一快照并 CAS；任一 fence 改变必须在写 claim 前失败。Store 返回后立即再次检查同一 guard，覆盖认领期间发生的撤销；此时失败只留下 claim recovery key。成功返回的 Authorized 同时拥有 guard 与 typed `PreparedComputePluginFetchClaim`，后者精确携带 claim ID、plan/download/candidate、authority/process/cursor/redirect generation、range 和时间，供后续 commit/abort 使用。完整权威流程固定为：

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

同句柄写入层先核对 payload 长度精确等于 claim range，再核对文件仍位于 committed offset；它按最多 64 KiB 的有界 buffer 手工调用 `write`，每次 syscall 前检查认领前捕获的 cancellation guard，随后依次执行 flush、`sync_all`，并在同一句柄复验 FileId、类型、reparse、卷、单链接和 claim end 精确长度。第一次 write syscall 一经发起，即使返回错误或零字节，旧 Authorized 也不得返还。

fsync 后的结果先成为只含 Authorized、`PinnedManagedFile` 与进程内 `Instant` barrier 的 synced capability；只有一个随后取得的、不可由普通墙钟或调用方毫秒数构造的可信时间 observation，才能把它封装成单一 `DurablyWrittenComputePluginSegment`。该 durable capability 直接拥有 Authorized、原文件句柄与 post-sync authority session，commit 不再接受可错配的“两件证据”。提交前再次在同一句柄复验精确长度和取消状态，再重读全部权威事实并 CAS，把 committed offset 精确推进到 claim end。任何 commit 失败都消费 mutation capability，只保留 recovery key 与原 `PinnedManagedFile`；共享关闭、key 撤销、计划过期、owner 改变或旧 generation 均失败关闭。

### abort、崩溃与完整工件

abort/revoke 不推进 cursor；重试创建新 generation 并仍从 committed offset 开始。恢复时文件长于 cursor 则截断未提交尾部，短于 cursor 则把候选标为损坏。每个 planned download 的 `complete` 只表示字节闭合；不能把最后一段的长度事实冒充完整摘要。

候选验证采用独立的 candidate-level run：只有同一 candidate 的精确计划闭包全部 `complete`、无 prepared fetch、当前 owner/plan/keyring/共享/inventory 仍一致时，才允许先从长期 pinned root 以父句柄相对方式打开全部 read-only/share-none 文件，再把规范 artifact-set digest 与本机 file-set binding digest CAS 为 prepared run。事务外必须从每个文件 byte 0 以有界 buffer 重算原始字节 SHA-256，每次 read 前检查同一 cancellation guard，检查 exact EOF，并在同一句柄复验 FileId、类型、reparse、卷、单链接与长度；不持久化跨版本不可移植的 SHA 内部状态。

完整文件集 hash 后必须取得严格晚于最后 hash barrier 的新 sealed trusted-time observation。未来短事务只有在 fresh authority 与 prepared run 全部未漂移时，才能把 run 终结为 verified 并把唯一 candidate 槽从 downloading 原子推进 verifying；任一摘要错配则终结为 rejected 并推进 failed。崩溃、取消、进程/authority 代次变化或 Store 结果不确定时不能复用内存 digest 或盲重试，只能凭 verification recovery key 查询、撤销旧 run 并从 byte 0 重算。raw artifact-set verified 仍不等于安全解包、staged、installed、健康或可 promotion。

## 6. 共享关闭与下一字节边界

关闭共享在同一事务中：

- 写入 `sharing_enabled=false` 并增加 authority epoch；
- 撤销所有 active fetch claim；
- 撤销所有 prepared candidate verification run；
- 停止所有 candidate，不遗漏“desired 已 disabled 但仍在 downloading”的记录；
- 把期望激活态推进为 disabled/draining；
- 阻止未提交 segment 的 commit。

下载器除每段、每次 redirect 前查库，还订阅进程内 cancellation epoch。canonical source 由 process fence 唯一持有，并同时绑定 authority instance、installation 与 process owner epoch；guard 只能从该 source 取快照，session 会按 source 的 `Arc` 身份及全部 binding 对账，因此新建另一 source 不能绕过已发生的 invalidate。当前代码已把 guard 强制捕获在 claim 之前、在 Store 返回后复查，并在每次文件 write buffer 前检查；未来真实 downloader 仍必须在每次 socket read 前检查同一 guard。只有两侧均接线后，“下一字节停止”才不被 16 MiB 分段放宽；关闭后的未提交尾部在恢复时截断。

## 7. 可信时间与恢复

可信时间取 `max(墙钟, 启动基线 + monotonic elapsed, persisted high-water)`，每次 plan 应用、claim 和 commit 都在同一事务推进 high-water。重启后若墙钟显著落后高水位，状态变为 `clock_untrusted` 并阻断下载，直到经认证的服务端 time attestation 刷新；不能仅停在旧高水位，否则反复重启可能延长计划寿命。

Keyring 安装和活动快照加载现在也必须在事务中读取、拒绝回退并 CAS 推进 `trusted_time_high_water_ms`；这些较早接口传入的 `trusted_now` 仍只是未来可信时间内核的前置条件，不能由 Host 直接拿普通墙钟冒充。逐段取数链已经收紧为 non-Clone、字段私有且没有生产构造器的 `ComputePluginTrustedTimeObservation`：process fence 消耗 observation 并固定 installation 与 clock epoch，普通 claim/redirect/abort session 也只能消费同 epoch observation，不再接裸 `DateTime`。fsync 后 observation 的 `Instant` 必须严格晚于 barrier；等值也失败关闭，因为相同 tick 无法证明先后。UTC 值才用于 Store 高水位，两者不能互相替代；若 UTC observation 与 claim 或高水位落在同一毫秒，只能重新取得真实可信观察或失败，禁止自行 `+1`。当前尚未形成启动基线、服务端 time attestation 或产生该 observation 的可信入口，因此 fetch fence/session 与 durable proof 都没有生产可达构造路径，本层仍不接运行路径。

每个 `ComputePluginLocalAuthority` facade 还持有不可序列化的进程内 instance binding；只有该 facade 的 clone 才共享身份。process fence、claim recovery key 和后续 authority session 必须对账同一 binding，防止把标量恰好相同的另一 facade 当成原 Store。该绑定随进程消失，只解决进程内 provenance，不能替代数据库外认证单调锚点。

SQLite 内部 revision、trigger 和 CAS 只能阻止当前数据库中的正常写入回退，不能独立识别“整个数据库文件被替换成一份更旧但内部自洽的副本”。Host 接线前必须增加数据库外的认证单调锚点，例如服务端保存并签回 installation digest + bundle revision/digest + authority epoch checkpoint，或平台受保护的单调存储；启动恢复低于锚点时失败关闭。不得把现有本机历史检查描述成已解决整库回滚。

恢复顺序固定为：

1. 取得 NodeAgent 独占实例锁；
2. 打开数据库、验证 authority schema 与完整性；当前预生产 v3 对旧版失败关闭，不执行原地迁移；
3. 建立可信时钟并递增进程 owner epoch，栅栏旧 fetch claim 与 prepared verification run；
4. 用当前 root/keyring 重验未终结 plan/manifests；
5. 对齐 fetch claim、文件长度与 committed cursor；
6. 对齐 candidate 指针、槽 marker、唯一 verified artifact-set run 和当前文件集合；没有终态 run 时绝不自动进入 verifying；
7. 只恢复 owner、授权、期限和 keyring 全部未漂移的工作，其余撤销并等待显式清理。

恢复不得自动接管孤儿候选、自动激活、静默清空库存或根据旧内存快照继续下载。

## 8. 当前实现状态

本合同已接受。当前已有严格网络 DTO、JCS/SHA-256/Ed25519 验签、Manifest 校验、InstallPlan 首次准入、root-signed keyring bundle DTO、Bootstrap root resolver seam、整包校验与两类 ring binding 派生。经整包验证的不可变快照只按预期 revision/digest、可信时间、用途、主体、状态和 key 有效期返回公钥；InstallPlan 与 live state 同时绑定 Publisher/Control 两类 ring revision/digest。

独立 `compute-plugin-state.sqlite3` 的路径型 facade、WAL/FULL/foreign-key/foreign-key-check 配置、私有 `BEGIN IMMEDIATE` seam、authority schema v3 与 inventory payload v2 已形成代码。v3 在原 meta、不可变 keyring/plan journal、candidate owner、download cursor 和 fetch claim 之上加入 candidate verification run、单 candidate prepared/verified 唯一性、跨 authority/process/cancel 的撤销门卫，并在内容与健康回执落地前禁止 promotion。此前 v1/v2 都从未接入 NodeAgent、未真实建库；本批采用明确的预生产 fail-close/rebuild 边界，不提供旧 schema 原地迁移，发现旧版或未知数据库直接拒绝打开，不能同版本静默采用。

原子 PlanApply 内核也已形成代码：它以完整 request digest 幂等封存 signed plan/Manifest 集和 typed admission，执行 install/upgrade/keep/disable/remove/cancel_candidate 投影，生成本机随机候选 token、下载闭包、inventory v2、receipt、event 与 seal，推进 state/inventory/authority fences，并可从 sealed application 恢复仍需 fresh authority read 的 execution capsule。回放会逐字段交叉验证 key 指纹/有效期、库存、候选、下载、时间和当前 epoch；两阶段 read + validated CAS claim 合同及 typed prepared claim 句柄已替代先写 claim 后校验的接口形状。

逐段 Store 内核已覆盖 begin、同 claim redirect、commit、abort、跨 authority/process 代次撤销、稳定终态读取和 prepared claim 恢复中止。所有写入都在 `BEGIN IMMEDIATE` 内重读精确事实并执行 CAS；commit 只有在单一 opaque durable capability 同时拥有 Authorized、原 `PinnedManagedFile`、精确 end length、fsync barrier 和写后可信 authority session 时才可达。claim identity 在 mutation 前生成，非授权 recovery key 同时绑定原 Store session 与不可序列化的 authority instance binding；因此 claim 返回成功或 mutation 结果不确定时都有同一份恢复身份。初始 claim 的 uncertain error 还暂存不可授权的 mutation 前 authority/download 快照；只有 exact claim 不存在、预期 cursor 和当前 prepared 均无冲突，且 authority revision/epoch/time 与 download identity/progress 全部精确未变时才返回 `NotCreated`。Store 一旦返回成功就删除这份 absence 快照；redirect claim 缺失和任何字段漂移都继续视为损坏。commit 发生在文件变更之后，所以其任何失败都不会返还 Authorized；失败只区分 Store 尚未调用或结果不确定，并保留 recovery key 与原打开文件句柄，恢复器只能稳定读取结果，不能盲重试。

Windows 文件安全层现已形成代码：`NodeDataPaths` 把 `compute-plugin` 作为不会被 cache/temp 清理的第五个受管根；类型化 `ComputePluginInstallationIdentity` 从原始 installation ID 唯一派生 Store digest，installation marker、受管根和 Authority 初始化不能再各自传入互不相干的身份。启动层可在实例锁后构造长期 `PinnedComputePluginRoot`，先取得并校验 Volume GUID 卷根句柄；卷根以下每个目录和最终文件都通过 `NtCreateFile` 的 `OBJECT_ATTRIBUTES.RootDirectory` 相对已钉住父句柄打开或创建，并对最终分量使用 `FILE_OPEN_REPARSE_POINT`。完整字符串路径只用于失败诊断和稳定卷检查，不参与根以下的授权性 lookup；UNC/DFS、reparse、目录型最终文件、跨卷和 hardlink 均失败关闭。claim 文件路径必须重新精确等于 `compute-plugin/candidates/{candidate}/downloads/{ordinal:04}-{artifact}.part`，文件身份只从同一句柄 `FileIdInfo` 派生并绑定 installation/root。offset 为零只允许 `create_new`；若文件已存在，Authorized 会先被消费，再以同一父目录 capability 安全重开并只返回恢复结果。正游标缺文件或短文件产出不可写 damage evidence；长于 Store cursor 的尾部只在同一句柄截断、`sync_all` 并复验身份/长度后返回线性 `ReconciledComputePluginPartFile`。目录创建、最终文件 create-new 或长尾截断中的任一变更会被带入后续 prior-mutation 判断。分段写入现在沿用该同一文件句柄，执行精确 payload/range、每 write buffer 取消检查、flush、fsync 和最终 identity/type/reparse/link-count/volume/length 复验；发生任何既有或本次变更后，失败只保留 recovery key 与原句柄。commit 前还会再次对原句柄做精确复核，不按路径重开。非 Windows 当前明确失败关闭，尚未宣称 portable beneath 实现。

以上新增代码仍未编译、未测试、未执行 DDL，也未在 NodeAgent 启动或 Host 路径接线，因此没有真实数据库、网络下载、插件安装或 Sidecar 运行。代码层已形成 fence-owned cancellation、分段耐久写入与 commit 恢复边界；authority v3 还形成 candidate verification journal、撤销 fence、promotion 禁入门卫，以及受管独占句柄从 byte 0 有界 SHA-256、每次 read 取消检查、exact EOF 和 hash 后身份复验底座。verification 的 fresh file-set read、prepared CAS、线性 Authorized/Hashed capability、post-hash trusted binder、verified/rejected 原子 resolution 和 outcome recovery 尚未形成，因此 schema 当前只允许 prepared run 被既有 fence 撤销，不能写出 verified/rejected 或推进槽。可信时间 observation 生成/attestation、数据库外防整库回滚锚点、短文件 damage 终结、socket-read 取消、真实 HTTPS downloader、安全解包/逐文件 closure、candidate health/promotion/GC、跨重启恢复和 Host/Sidecar/IPC/沙箱仍待实现；本节不代表下载器或插件系统已经可用。
