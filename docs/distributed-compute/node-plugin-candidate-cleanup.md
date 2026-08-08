# 节点插件失败候选清理

## 1. 状态

当前状态为 `partial_implementation_mixed`。Windows 受管文件系统、私有 cleanup authorization Store、确定性 topology typed Store、首对象四阶段 journal（delete intent、强 disposition、parent absence、parent namespace durability）、原子 completion Store 内核，以及各 Store 的进程内 outcome-uncertain recovery 已形成代码。当前 canonical 已通过 `elon-pc-node` 编译、32 项候选清理定向测试及 1 项包含 topology/journal 对象的 schema 建库与重开测试；sequence 3/4 的新增证据覆盖纯 builder hash-chain/篡改拒绝及 SQLite exact-row/列篡改拒绝，不代表完整 Store 事务或真实 Windows namespace barrier 已验收。topology 与四个 step Store 均独立提交并 exact readback。序号 4 已写出“同一 retained parent handle 上重证 absence → Windows native barrier → 再次重证 absence → 独立 Store”的完整类型/事务形状，提交不确定恢复也不会重复物理操作；mutation fence 已从可复用借用改为线性租约，精确绑定对象/父 identity、cleanup/plan/authorization receipt、installation、authority/process epoch 与 ordinal，并随 pre/post retry、terminal custody、物理能力和 Store recovery 一直移动。v1 wire 现已固定 Describe/Acquire/Query/Release、完整 grant/session/scope 回读、稳定 release nonce 和规范 descriptor 摘要，并通过 9 项 descriptor/encode/decode/拒绝边界测试；特权组件 Manifest/InstallPlan 与普通插件 Publisher 权限完全分离，签名 shape 显式固定 release/install-plan purpose、拒绝同 key ID，并通过 5 项 non-authorizing shape/cross-binding/fail-closed gate 测试。但租约仍没有安全构造器，真实 key resolver/fingerprint/签名验证与安装门仍固定失败，仓库也没有 WDK 驱动、真实 Filter Manager transport 或 Windows catalog 验签。微软明确规定目录 R/RH oplock 对子项增删的 break 只是 advisory，变更不等待 ACK，因此不能作为此类型的后端。普通 parent handle 也无法排除同权限外部进程在 flush/post-proof 窗口执行瞬时同名 ABA；在签名 minifilter 或等价隔离执行域真正落地前，安全代码不能签发 `ManagedNamespaceDurable`，更不能接入 Host。后续还缺真实 fence 后端、其余 ordinal、terminal journal、跨重启物理恢复和生产 Host，当前不会自动清理生产失败候选。

本文只维护失败候选清理边界。候选本机真源见 `node-plugin-local-authority.md`，健康失败与 quarantine 见 `node-ready-capability.md`，staging 物化见 `node-plugin-archive-extraction.md`。

## 2. 已实现的底层能力

1. 新建受管文件和目录在创建时取得 `DELETE` 权限，但不会因此自动删除。
2. 既有候选 downloads 目录只在最终目录分量上取得删除权；受管根内中间前缀保留 traverse、目录 namespace-flush 所需的最小写位和 share-write，但不取得 `DELETE` 或 `FILE_SHARE_DELETE`。
3. 原始候选文件以父句柄相对、数据只读、share-none 且携带 `DELETE` 权限的句柄打开。
4. 删除调用 Windows `FileDispositionInfoEx`，绑定已经固定的对象，不重新解析完整路径，也不调用 `remove_dir_all`。
5. 旧 `delete_exact()` 在文件删除成功后消费文件句柄；失败则返回错误和同一文件 custody。它只服务尚未适配的 v1 执行器，返回值不是 completion 或 durability 证据。
6. 旧 `delete_exact()` 在目录删除成功后消费目录句柄；非空、共享冲突或权限错误时返回错误和同一目录 custody。它不会保留后续 namespace 观测所需的父句柄。
7. 非 Windows 平台继续失败关闭，尚未提供 portable beneath/no-follow 删除实现。
8. 每个非根受管文件或目录现在保存从同一对象句柄和同一直接父句柄派生的对象 identity digest、父 identity digest，以及父句柄相对打开时使用的原始单组件名称；可 Clone 的 `ManagedObjectBinding` 只作为 topology 输入，不授予删除权。
9. 独立强入口 `set_delete_disposition_exact()` 会运行时复验对象和父 identity；目录目标还要求 final `Arc<File>` 无别名。成功后明确关闭目标句柄，并返回不可 Clone、持续持有 exact parent handle chain 的 `ManagedDeleteDisposition`。旧批量执行器仍未调用该入口；新的私有单对象 executor 只在消费 durable intent 后调用。
10. disposition 可继续线性观测 `Absent`、`ExpectedIdentityMatch` 或 `IdentityConflict`。Windows 只把父句柄相对 `NtCreateFile` 返回的 `STATUS_OBJECT_NAME_NOT_FOUND` / `STATUS_NO_SUCH_FILE` 当作 absence；delete-pending、共享冲突、权限失败和其他状态都保持不确定。历史 identity digest 相同还必须对象种类相同才会标记为 `ExpectedIdentityMatch`；该名称不宣称仍是原对象，因为目标 handle 已关闭后 FileId 仍可能被文件系统复用。两类 name-present 结果都不提供返回 disposition/absence 的降级通道，不能自动重试或删除该名称下的对象。
11. 配置受管根的最终分量及根内所有目录首次固定时取得目录语义的最小 `FILE_WRITE_DATA`（等价 `FILE_ADD_FILE`）和 `FILE_SHARE_READ | FILE_SHARE_WRITE`；仍拒绝 `FILE_SHARE_DELETE`，所以 retained parent 本身不能在清理期间被 rename/delete。根之前的 volume/path prefix 保持只读 traverse 句柄。
12. `ManagedParentRelativeAbsence::make_namespace_durable()` 在同一 retained parent handle 上执行 pre-barrier absence 复验、`NtFlushBuffersFileEx(flags=0)` 和 post-barrier absence 复验。当前明确接受 ntdll user-mode native contract：只在函数返回与 `IO_STATUS_BLOCK.Status` 都为 `STATUS_SUCCESS` 时签发 `ManagedNamespaceDurable`；为与首个硬围栏后端保持同一可证明集合，序号 4 只允许 NTFS/ReFS，非 Windows、其他文件系统、权限或协议结果均失败关闭。
13. 明确失败的 barrier 可携带同一 absence custody 重做 pre-proof 后重试；barrier 已成功但 post-proof 普通失败只允许重做 post-proof，不能再次 flush；`STATUS_PENDING`、informational/返回值不一致、已打开同名对象、expected identity match 或 identity conflict 均进入不透明 terminal custody，不提供返回 absence/disposition 的回边。
14. `ComputePluginRootLockLease`、`cleanup_pending` owner、process fence 与 cancellation guard 共同排除 NodeAgent 权威域内的并发写入，但普通父目录 handle 不能阻止同权限外部进程在 barrier 后短暂创建并删除同名对象。pre/post 两次 `Absent` 无法单独排除这种 ABA。目录 R/RH `FSCTL_REQUEST_OPLOCK` 虽会在子项增删时 break，却不会让写操作等待 ACK，只有目录自身 rename/delete 的特定 break 才等待 ACK；因此 directory oplock、`ReadDirectoryChangesW`、USN、share mode、TxF 或循环观察都不能构造排他 `ManagedNamespaceMutationFence`。
15. mutation fence 已改成不可 Clone 的线性租约：首次物理入口按值消费，连入口拒绝、pre-barrier retry、post-barrier retry、terminal retained custody 与 `ManagedNamespaceDurable` 都拥有同一租约，不再接受可提前释放或跨对象复用的 `&Fence`。绑定门卫同时核对 handle-derived object/name/parent scope、cleanup、plan、authorization receipt、topology object、installation、authority epoch、process-owner epoch 与 ordinal；pre-proof 前后、barrier 后、post-proof/mint 前、Store prepare/事务写入前后及 recovery adoption 都强制 `ensure_active`。当前该查询是固定失败的后端未安装 stub，内核租约字段也不可构造，所以仅补一个构造器仍不能解锁序号 4。
16. 第一种允许构造该租约的后端固定为 `windows_signed_minifilter_child_namespace_fence_v1`。驱动 grant 必须从调用方 parent handle 在内核重建 volume instance、`FILE_ID_128` 与实际大小写语义下的单组件名称，并在回复前原子登记规则、排空已进入的冲突 mutation；不能信任用户态提交的摘要。driver/port 断连、grant/query/release 超时、volume teardown、无法判定的名称或 generation 漂移都进入 outcome-uncertain，不能降级到 oplock 监测结果。

`PinnedComputePluginCandidateArtifactSet` 现在保留完整 `PinnedComputePluginCandidateDownloads`，而不只保留根锁 lease。因此 verified、staged、健康评估和 quarantine 链没有提前丢失 downloads 目录句柄；新建 staging 文件与目录也天然保留删除权。

SQLite v3 schema 还具备以下约束：

1. `candidate_cleanup_authorizations` 只接受绑定 failed quarantine、staging receipt、candidate owner、inventory、authority/process fence 与可信时间的不可变授权回执。
2. candidate owner 只有在授权回执已经持久化后，才能从 `owned` 进入 `cleanup_pending`。
3. `cleanup_pending` 仍计入单插件活动候选唯一约束，清理未完成时不能创建替代候选。
4. `candidate_cleanup_completions` 只接受绑定原授权、执行证据和新 inventory fence 的不可变完成回执。
5. candidate owner 只有在完成回执已经持久化后，才能从 `cleanup_pending` 进入 `cleaned`。
6. `candidate_cleanup_execution_plans` 与 expected-object rows 在第一次物理删除前固定 installation/root、授权回执、规范相对路径、对象种类、同句柄 identity、父级 identity、文件内容摘要、大小和父子拓扑；candidate 根的父级 identity 作为不删除的 anchor。
7. plan seal 只接受 ordinal 连续、对象总数和字节数闭合、唯一 candidate 根、且每个父目录都晚于子对象执行的完整拓扑。
8. 每个对象的 step journal 严格执行 `delete_intent -> exact_handle_disposition_set | absence_recovered_after_intent -> parent_namespace_absence_observed -> namespace_durable`，序号由 step ordinal 唯一决定，前一事件摘要从 plan digest 起形成不可变 hash chain。
9. process 崩溃后恢复必须按 terminal event 与 expected identity 判定“同一对象仍存在、已缺失或身份冲突”；没有 durable intent 不能把路径缺失当成该 cleanup 的成功，身份冲突不能删除替代对象。
10. completion 必须同时绑定 sealed `execution_plan_digest`、既有物理 `execution_evidence_digest` 与独立 `terminal_journal_digest`；后者必须等于完整 journal 最后一个 `namespace_durable` 摘要。物理 delete-disposition 证据不能冒充 namespace durability；只设置删除 disposition、只观察 absence 或缺少任一对象 durability fence 都不能写 completion。

这些对象目前仍属于预生产 schema v3：缺少新对象或定义不一致的同版本库会失败关闭并要求重建，不提供旧 schema 原地迁移。execution plan 是 authorization v1 之后的第二个 fresh、不可变执行授权边界；只有 topology typed Store 对 plan、全部对象和 seal 完成原子写入与 exact readback 后，才会返回持有原始句柄、根锁和授权回执的 `SealedCandidateCleanupTopology`。单独的 cleanup authorization、手写 plan rows、未封存的内存计划或只读恢复结果都不能触发删除。

topology builder 会从完整逻辑路径计算父子关系和深度，强制所有子对象先于父目录、candidate 根最后执行，并要求根对象的父 identity 精确等于不删除的 candidate-parent anchor。对象 identity、父 identity、原始单组件名称、文件内容摘要与大小均来自当前 retained custody；缺失 binding、对象种类变化、名称变化、身份变化、父锚点变化、对象重复或拓扑不闭合都会失败关闭。Store 事务前后均重验授权、owner、installation/root、process fence 与可信时间，且 `planned_at` 必须严格晚于授权时间。恢复查询不得推进状态或重新准备计划，只能在相同进程 authority binding、相同 recovery key 和未漂移权威事实下采用已封存拓扑。

首对象 delete-intent Store 从 sealed plan 的 ordinal 0 派生事件，固定 plan/object/parent identity、事件序号 `1`、前序摘要 `plan_digest` 和严格晚于 plan seal 的可信时间。事务在写入前后重验 sealed topology、授权、owner、installation/process fence、可信时间与无 completion，并先独立推进 trusted-time high-water，再写入 JCS+SHA-256 事件和 exact readback。成功返回的 `DurableCandidateCleanupDeleteIntent` 继续持有 sealed topology、全部原始句柄与根锁；不确定结果在分类前不得物理删除。该 Store 目前只覆盖首个 `delete_intent`，不把其余三阶段或后续对象伪装为已完成。

首对象 disposition Store 只消费已经由原句柄设置强 disposition 的 `PhysicallyDisposedCandidateCleanupObject`。它固定同一 plan、intent digest、ordinal 0、对象/父 identity、事件序号 `2` 与严格晚于 intent 的可信时间；fresh 事务重验 plan seal、intent、authorization、owner、installation/process fence、可信时间与无 completion，推进 trusted-time high-water 后写入 `exact_handle_disposition_set` 并 exact readback。成功返回 `DurableCandidateCleanupDisposition`，继续持有 exact parent、剩余对象与根锁；提交结果不确定时 recovery 只能分类为 `NotCreated` 或 exact `Durable`，两种结果都不会重新执行已经发生的物理 disposition。该 capability 尚不能跳过父句柄相对 absence 观察或构造 terminal journal。

首对象 parent-absence 路线只消费 durable disposition，并用保留的 exact parent handle 相对原始单组件名称观察命名空间。只有 `Absent` 会产生 `ObservedCandidateCleanupParentAbsence`；expected identity match 与 identity conflict 都保留同名对象句柄且没有自动重试或删除回边，检查失败若已经打开同名对象则进入独立 quarantine custody，只有未打开对象的普通失败才允许携带同一 disposition 重试观察。absence Store 以严格晚于序号 2 的可信时间构造序号 `3`，固定 `observed_identity_digest = NULL`、原父 identity 和前序 disposition digest；写前要求 exact plan、授权回执 JSON/JCS、pending owner、序号 1/2、总事件数 2 和高水位等于序号 2，写后要求 exact row/JSON/JCS、序号 3 identity 唯一、总事件数 3 和高水位等于序号 3。结果不确定恢复只接受这两个完整状态；recovery key 保存完整 hashed authorization receipt 并逐字段对账 state revision、inventory 与 authority epoch。成功只返回 `DurableCandidateCleanupParentAbsence`，它是私有序号 4 物理入口唯一允许消费的前序能力，但仍不能直接构造 terminal journal。

首对象 namespace-durability 路线只消费上述 durable absence 与不可伪造的 mutation fence。物理阶段固定同一对象/父 identity 与同一 parent handle，pre-proof、native barrier、post-proof 任一可疑结果都按线性 custody 分类；fence 类型当前无安全构造器，所以这条代码形状尚不可执行。未来成功产生的 `PhysicallyDurableCandidateCleanupNamespace` 不能 Clone，也不能由调用方传入裸 kind/evidence 伪造。序号 4 builder 固定 `event_kind = namespace_durable`、`observed_identity_digest = NULL`、前序序号 3 digest 和 process-owner epoch，并用私有 JCS-SHA-256 evidence 绑定 cleanup/plan、ordinal 0 对象、父 identity、序号 3、固定 primitive kind、handle-derived filesystem kind 与记录时间。Store 写前要求 exact plan、完整 authorization/owner、序号 1/2/3、总事件数 3、无 completion 且 high-water/updated-at 都等于序号 3；推进可信时间后写入并 exact readback 序号 1–4、唯一 identity 和总数 4。恢复只接受 exact `Durable` 或 exact `NotCreated`，其他状态全部 ambiguous fail-close；即使未来成功，也仍只是 ordinal 0 journal，不会构造 terminal capability。

cleanup authorization Store 当前执行以下单一事务：fresh read failed slot、精确 quarantine/staging 回执、`owned` owner、inventory/state/authority/process fence 与可信时间；确认没有 prepared fetch/verification 后推进可信时间和 state/authority epoch，inventory 保持不变；随后写入 JCS+SHA-256 不可变授权回执，并把 owner 推进到 `cleanup_pending`。返回的 `AuthorizedCandidateCleanup` 继续持有原候选目录和文件句柄，不能序列化或克隆。提交结果不确定时，进程内 recovery key 只允许读取 `NotCreated` 或 exact `Authorized`；exact adoption 会重新读取权威状态并复验 retained staging 内容。

本批在远程 canonical API 上进一步加固组合语义：授权读与写事务都复验同一 process cancellation source；owner 对账绑定 plugin、slot、generation、release、owner plan 和 application inventory revision；恢复同时重放 quarantine→staging 的规范 JSON/JCS 链并拒绝已有 completion。PlanApply 投影继续把 `cleanup_pending` 计为占用 candidate pointer 的活动 owner，但拒绝任何直接修改该插件的 action；原计划历史重放接受 `cleanup_pending` 与未来 `cleaned`，只在 owner 仍为 `owned` 时返回可操作 candidate handle。当前 inventory invariant 也会让 `cleanup_pending` 期间的 sharing-off plan 失败关闭；“立即停止运行、后台继续保留清理 custody”的独立退出语义仍待设计。

旧式私有物理执行器会在首次删除前复验 retained staging 内容和 cleanup authorization，再尝试从既有受管根固定 candidate 根与 staging 父目录，并按 extracted 文件、staging seal、最深 extracted 目录、staging run、staging 父目录、download 文件、downloads 目录、candidate 根目录排序。它仍调用旧 `delete_exact()`，只聚合进程内 disposition evidence，没有消费 `DurableDeleteIntent`、保留父句柄或追加 journal。全树接受 disposition 后产生的 `PhysicallyExecutedCandidateCleanup` 已保留原始 staging recovery key、同一 cancellation guard、单调完成点、授权/隔离/staging 回执、执行证据和根锁，但这些 custody 不能替代缺失的 journal 中段。

旧 prepare 路线仍有已确认的 Windows 共享访问风险：retained 受管目录现在允许 `FILE_SHARE_READ | FILE_SHARE_WRITE` 以支持同一 parent 的 namespace barrier，但继续拒绝 `FILE_SHARE_DELETE`；旧执行器随后 late-open 同一 candidate/staging 目录并请求 `DELETE`，仍可能在真正执行前直接得到 sharing violation。不能通过全局追加 `FILE_SHARE_DELETE` 绕过，因为那会允许钉住期间 rename/delete 并削弱 namespace custody。新首对象路线从原始 custody 线性消费，不依赖该 late reopen；后续 ordinal 仍应沿用原始 custody 或收口为唯一 cleanup handle tree。在完整链验证前，`PhysicallyExecutedCandidateCleanup` 只代表旧式源码形状，不代表生产可执行或耐久清理。

远程 completion Store 内核会在一个 `BEGIN IMMEDIATE` 中消费不可伪造的 `DurableCandidateCleanupTerminalJournal`，重验 sealed plan、全部对象的 `namespace_durable` 计数与末端摘要、物理证据链、原授权、`cleanup_pending` owner、failed slot、inventory/state/authority/process fence、可信时间和无 prepared 工作；随后只移除精确 failed slot、清空 candidate pointer但保留插件记录与 active slot，推进全部 fence，写入同时绑定 plan/evidence/journal 的不可变 completion receipt，并把 owner 原子推进到 `cleaned`。事务后重读与 recovery 只允许 exact `NotCreated` 或 `Completed`。终态 journal capability 仍无构造入口，所以旧执行器不能绕过缺失的中段直接调用该内核。

## 3. 当前能力不代表什么

底层句柄拥有删除权，不等于业务层已经授权删除。以下对象都不能单独触发清理：

- verification verified/rejected outcome；
- staging receipt；
- health receipt；
- quarantine receipt；
- `failed` slot phase；
- 调用方传入的布尔值、路径或 candidate token。

当前生产路径不会自动删除失败候选，也不会创建新候选、恢复下载或允许重试。私有代码能够签发 cleanup authorization、封存 exact topology，并完成首对象 intent → 原句柄强 disposition → durable disposition event → durable parent absence → 真实 parent namespace barrier → durable namespace event 的分段能力链，同时具备终态 journal 之后的原子 completion 内核；但后续 ordinal 与 terminal journal producer 未实现，完整流水线不可达且未由 Host 调用。首对象到达 `namespace_durable` 仍不能对外表现为 owner 已释放或候选可重建。

`namespace_durable` 已从 schema 名称推进为 ordinal 0 的私有物理/typed Store 架构，但当前不可构造的 mutation fence 会阻止安全代码真正生成该能力。已写部分依赖同一 retained parent handle、Windows native barrier、post-barrier absence proof、可信时间与 exact journal readback，且调用方不能用裸字符串或摘要伪造；sequence 3/4 已随节点编译并通过纯 builder 与 exact-row 定向测试。外部同权限 writer 的瞬时 ABA fence、真实 barrier/fresh/recovery 事务夹具、旧执行器强路径适配、后续对象、物理 evidence v2 与 terminal journal producer 仍未实现，因此 completion 入口依然不可达。

现有物理执行器的 v1 `execution_evidence_digest` 只聚合进程内 delete-disposition 步骤，尚无不可变 evidence row，也未绑定 `execution_plan_digest` 与逐对象 `object_digest`。completion Store 之前必须持久化并重算该证据，或升级为包含这些绑定的 evidence v2；它与 `terminal_journal_digest` 是两条独立证据链，禁止重新合并为同一摘要。

SQLite trigger 只能证明前序 event 在当前事务视图中存在，不能证明它已由更早事务 durable commit。当前首 intent typed Store 不允许调用方传入或复用裸 `Transaction`，独立提交并 exact readback 后才返回非 Clone `DurableCandidateCleanupDeleteIntent`；旧批量 executor 的公开入口已撤回，不能从 Host 直接消费 sealed topology。后续物理删除必须只消费该 capability，disposition、absence、durability 与 completion 各自使用 fresh transaction；不得提供一次事务批量写完四阶段或 completion 的接口。

## 4. 已固定且后续必须保持的事务顺序

完整清理必须拆成以下七个边界：

1. **Fresh authorization**：在 `BEGIN IMMEDIATE` 内重读 failed slot、quarantine receipt、candidate owner、inventory/state/authority/process fence 和可信时间，写入不可变 cleanup authorization，并把 owner 推进到 `cleanup_pending`。
2. **Topology Store**：从仍持有的句柄生成 expected-object topology，在一个事务中写 plan、全部对象和 seal；任何对象缺少 identity 或 parent 都不能进入执行。
3. **Durable intent**：每一步先提交绑定 exact object identity 的 `delete_intent`，再调用删除原语；没有 intent 不得把路径缺失解释为该步骤结果。
4. **Physical execution**：只消费授权对象与保留句柄，按文件、seal、最深目录、staging run、staging 父目录、downloads 目录、candidate 根目录的拓扑顺序执行。
5. **Outcome/partial custody**：同句柄 disposition、父目录相对 absence 与 namespace durability 必须分别追加事件；崩溃恢复只可在已有 intent 后记录受约束的 recovered absence。任何中途失败都返回 terminal journal 位置与剩余句柄，身份冲突永久失败关闭。
6. **Completion Store**：只有所有对象都到达 `namespace_durable` 且 terminal hash 与执行证据一致后，才能把 owner 从 `cleanup_pending` 推进到 `cleaned`、移除 failed slot、清空 candidate pointer，并精确推进 inventory/state/authority fence。
7. **Retry gate**：新候选或旧计划重试只能基于 durable completion outcome；cleanup authorization、quarantine、plan seal 或内存执行成功都不能代替完成回执。

上述 authorization、topology、首对象四阶段 journal、旧物理执行和 completion 已有私有类型或 Store；后续对象 intent/journal 与 terminal producer 仍缺。如果 topology、intent、disposition、absence、namespace durability 或 completion Store 结果不确定，调用方必须只凭各自 recovery key 查询 exact `NotCreated/Sealed`、`NotCreated/Durable intent`、`NotCreated/Durable disposition`、`NotCreated/Durable absence`、`NotCreated/Durable namespace` 或 `NotCreated/Completed`。其中 namespace `NotCreated` 归还的是已完成物理 barrier 的 capability，只能重新准备可信时间并重试 Store；不能重复封存、写 intent、执行 disposition、观察已提交 absence、重做 barrier、释放 owner 或直接开始新候选。

## 5. 目录清理规则

- 清理过程必须持续持有 `ComputePluginRootLockLease`。
- 最终对象必须来自已固定父句柄的相对打开或原 create-new 句柄。
- 目录删除前必须已经消费全部子文件和子目录 custody。
- unexpected entry 会让目录删除失败关闭，不得递归跨越未知 reparse point。
- 路径字符串只用于规范逻辑名称、审计和错误信息，不参与授权性删除 lookup。
- 权限、共享冲突或非空错误不允许降级到普通路径 API。

### 5.1 Windows 硬围栏后端合同

首版 minifilter 只承诺本机 NTFS/ReFS，并通过 ACL 限制的单连接 Filter Manager port 与唯一 NodeAgent process object 建立会话。协议至少提供幂等 `AcquireFence(client_nonce, parent_handle, child_name, authority_binding)`、`QueryFence(fence_id)` 和 `ReleaseFence(fence_id, expected_generation)`；grant receipt 绑定 driver boot/session generation、volume instance/serial、parent `FILE_ID_128`、目录实际 case-sensitivity 下的 child name、installation、cleanup/plan、authority/process epoch、fence generation 与 grant sequence。driver 必须在 active grant 存在时拒绝普通 service stop/unload；强制 detach、dismount 或 driver crash 会毒化租约，不能把“不再能查询”解释为已释放。

拦截面至少覆盖 `IRP_MJ_CREATE` 的 create/open-if/overwrite-if、目录创建与 `FILE_DELETE_ON_CLOSE`，以及 `IRP_MJ_SET_INFORMATION` 的 `FileRenameInformation[Ex]`、`FileLinkInformation[Ex]`、`FileDispositionInformation[Ex]`。rename/hard-link 的 destination 与删除目标都必须按 Filter Manager 解析后的 parent FileId 和文件系统名称规则匹配；任一可能命中受管 scope 但无法可靠解析的操作都应拒绝并 poison 对应 fence。命中 active fence 的外部 mutation 首版直接拒绝，不无限 pending。用户态必须在 exact grant 后重新执行 pre-absence → native flush → post-absence，并让现有 `ensure_active` 门在 capability mint 与 Store 临界点 exact query 同一 grant generation；只有完整实现这条路径，才能替换当前不可构造字段和固定失败 stub。

目录 oplock 可以另作诊断监测器，验证瞬时 create/delete 确实触发 break；它必须使用独立 asynchronous handle，并与 durability fence 类型、证据 kind 和构造入口完全分离。这个监测器只能帮助测试威胁模型，不能解锁序号 4。

### 5.2 v1 wire 与特权组件供应链边界

checked-in wire descriptor 为 `windows-compute-namespace-fence-wire-v1.json`，其 RFC 8785 JCS SHA-256 固定为 `9557e4da4e5992ce604b2e102afd0d448d0a9fd23f5acbf49ad06a5eb17244d6`。它固定 `ELONFNC1`、little-endian、x86_64、64-byte header、1–8 request/reply kind、全部字段偏移、4 KiB 上限、低 16 位必需能力和 reserved-zero 规则。Rust codec 不使用 `repr(packed)`、裸指针、`usize` 或 `bool` 作为 wire 字段；名称为不带 NUL 的 1–255 UTF-16LE 单组件。任何未知 flag、非零 reserved、长度/offset 漂移、stale request ID 或错误 transport connection 都失败关闭。

Acquire 中的 parent handle 只是一龙唯一 NodeAgent 进程句柄表中的瞬时值；驱动必须按 `UserMode` 引用并验证实际目录对象，自行重建 volume instance/serial、parent `FILE_ID_128`、NTFS/ReFS 类型与目录大小写语义。Query/Release 不再携带 handle，而是精确重放 owner connection、boot/session、volume/grant generation、原始 UTF-16 名称、authority binding、fence ID 与非持久化 secret。恢复连接只进入 header 的 transport connection，不能冒充原 owner。reply 必须回显完整 snapshot 与本次 request；Release 还必须回显稳定 release nonce 和单调 release sequence。含 secret 的 receipt 不可 Clone，Debug 不输出 secret、卷序列号、FileId 或名称。

首方 `PrivilegedComponentManifest/InstallPlan` 使用独立 release/install-plan key purpose，不接受普通 compute-plugin Publisher 或 InstallPlan 授权。签名 envelope 必须声明与类型一致的 purpose，cross-binding 还拒绝 Manifest 与 InstallPlan 复用同一 key ID；该检查只是 metadata 约束，未来 resolver 仍必须按 purpose 解析并拒绝相同 public-key fingerprint。合同精确固定 component、service/filter/instance/port、协议 descriptor、driver build、`.sys/.inf/.cat` 摘要与长度、catalog publisher/certificate、微软 kernel trust、Node 兼容范围、rollback generation、显式用户同意、UAC 和“无活动 fence 才能升级”。当前只做 non-authorizing shape/cross-binding 验证；微软分配 altitude、Bootstrap 首方 key resolver、RFC 8785/Ed25519、WinVerifyTrust/catalog、下载、安装、SCM/FilterLoad 和驱动本体均未实现，安装 gate 因此固定失败。

## 6. 当前验证

当前 canonical 已通过 `elon-pc-node` 编译、32 项候选清理定向测试、9 项 minifilter wire 合同测试和 5 项特权组件合同测试：

1. 以 create-new 句柄删除精确文件，再删除目录；
2. 非空目录删除失败后保留原目录 custody，删除子文件后用同一 custody 重试；
3. 重新固定既有文件的 cleanup 句柄并完成文件与目录删除。
4. staging 子目录按深度从深到浅、同深度按规范逻辑路径稳定排序。
5. completion receipt JSON 往返后保持稳定 JCS 摘要；
6. completion receipt 拒绝未知字段；
7. completion 投影只删除 failed candidate，保留插件记录与 active slot；
8. completion 投影拒绝重复 candidate slot identity。
9. topology builder 对相同输入产生稳定摘要，并保持严格 child-first、candidate-root-final 顺序；
10. topology builder 拒绝 parent-before-child 输入；
11. topology builder 拒绝 candidate 根外父锚点 identity 漂移；
12. topology Store 原子写入后可 exact round-trip plan、全部 expected objects 与 seal；
13. topology Store 在对象列被篡改后拒绝 sealed adoption。
14. 首 delete-intent 对相同 plan/time 产生稳定摘要并绑定 plan digest；
15. 首 delete-intent 拒绝不晚于 plan seal 的可信时间；
16. intent Store 写入后可 exact round-trip 事件列、JSON 与摘要；
17. intent Store 在父 identity 列被篡改后拒绝 durable adoption。
18. 原文件句柄设置强 disposition 后只产生保留 exact parent 的能力，并需父句柄相对观测才能证明 name absence；
19. 子文件仍持有父目录句柄时，目录强 disposition 失败并保留原目录 custody，子文件 absence 能力释放后可用同一目录 custody 重试；
20. cleanup pending object 必须与 plan 对象的 logical path、对象/父 identity、内容摘要和大小精确匹配后才能设置强 disposition。
21. 首 disposition event 对相同 plan、intent、对象绑定和可信时间产生稳定摘要，并以前序 intent digest 建立 hash chain；
22. disposition event 拒绝不晚于 intent 的可信时间或对象种类绑定漂移；
23. disposition event Store 写入后可 exact round-trip 事件列、JSON 与摘要；
24. disposition event Store 在前序摘要列被篡改后拒绝 durable adoption。
25. parent-absence event 对相同 plan、intent、disposition、对象绑定和可信时间产生稳定摘要，并以前序 disposition digest 建立 hash chain；
26. parent-absence event 拒绝不晚于 disposition 的可信时间或对象绑定漂移；
27. parent-absence event Store 行写入后可 exact round-trip `observed_identity_digest = NULL`、事件列、JSON 与摘要；
28. parent-absence event Store 行在 observed identity 被篡改后拒绝 exact adoption；
29. namespace-durable event 对相同 sequence 3、对象绑定、primitive/filesystem kind 和可信时间产生稳定摘要，并以前序 absence digest 建立 hash chain；
30. namespace-durable event 拒绝不晚于 absence 的可信时间或不支持的 filesystem kind；
31. namespace-durable event Store 行写入后可 exact round-trip durability kind/evidence、事件列、JSON 与摘要；
32. namespace-durable event Store 行在 evidence digest 被篡改后拒绝 exact adoption。

前三项 Windows 测试只证明旧底层句柄删除语义，第四项只证明目录排序函数，第五至八项覆盖 completion，第九至十三项覆盖 topology，第十四至十七项覆盖首 intent 纯构建与 Store exact readback，第十八至二十项覆盖原句柄强 disposition、父句柄别名失败保管和 pending-to-plan 绑定，第二十一至二十四项覆盖 disposition event 构建、链接和 Store exact readback，第二十五至三十二项只覆盖 sequence 3/4 纯 builder 链和 SQLite exact-row 语义。另有 1 项 schema 定向测试证明 cleanup authorization/completion、四张 execution topology/journal 表及相关 trigger 可以安装并按相同指纹重开。9 项 wire 测试覆盖 descriptor 摘要、固定编码布局、完整 session/grant 回读及 unknown/reserved/stale/range/digest 拒绝；5 项特权组件测试只覆盖 shape、purpose/key ID 隔离、cross-binding 与 altitude 未分配时的固定失败门，不执行真实签名或 Windows 安装。现有验证仍没有运行 retained custody → cleanup authorization → topology → intent → physical disposition → disposition Store → parent observation → absence Store → namespace barrier → namespace Store 的完整事务夹具，也不覆盖真实 parent-relative OS observation、native directory flush、序号 3/4 authority/owner fresh/recovery 事务、minifilter transport、跨重启物理恢复或生产 Host 接线。

## 7. 下一批实现边界

下一批必须把本批协议和供应链 shape 变成真实受信路径：实现 WDK minifilter、ACL 限制的 Filter Manager port、唯一 NodeAgent process 绑定、Reserving → Draining → Active 原子 grant、exact Query/Release/tombstone，以及 Bootstrap 首方 Ed25519、Windows catalog/内核信任、下载/UAC 安装/回滚与版本门卫；随后才为当前线性租约增加唯一安全构造器。驱动必须在 grant 后的 pre-proof、barrier、post-proof、能力签发及后续 journal custody 窗口排除同权限 writer，且 port disconnect、driver/volume teardown 或结果不确定必须保留句柄并失败关闭。不得用目录 oplock、无限重复观察或 advisory lock 冒充 ABA 证明。完成后还要把 minifilter grant facts 纳入物理 evidence v2，并在序号 4 exact Store 后执行显式线性 Release，才能把 ordinal 0 的 custody 推进到 ordinal 1：证明前一对象 child/parent custody 已按 topology 安全释放，从 plan 的下一个 expected object 生成新 intent，并把四阶段 builder/Store 参数化为任意 ordinal，同时保持 fresh transaction、exact readback、可信时间单调和 outcome-uncertain 分类。当前 child-first retained 队列与 plan 同序，不需要先做一次高风险的全量句柄树重写，但多对象部分失败与跨重启恢复应逐步收口为单一 handle-tree/step-cursor 抽象。全部对象完成后才构造 `DurableCandidateCleanupTerminalJournal`；随后补齐完整事务夹具和 native barrier/fence 验证，最后接入生产 Host。不得先暴露“按 candidate token 删除目录”的管理接口，也不得把首对象 `namespace_durable` 冒充 terminal journal。
