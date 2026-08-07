# 节点插件失败候选清理

## 1. 状态

当前状态为 `partial_implementation_mixed`。Windows 受管文件系统、私有 cleanup authorization Store、旧式固定句柄执行器，以及原子 completion Store 内核和两阶段进程内 outcome-uncertain recovery 已形成代码；远程 canonical 最新组合通过 `elon-pc-node` 编译、8 项清理定向测试及 1 项包含 topology/journal 对象的 schema 建库与重开测试。completion 内核只接受同时绑定 sealed plan、物理证据与 terminal namespace-durability journal 的不透明能力，目前没有公开或私有构造入口。本批进一步形成对象/直接父级/原始名称 binding、拒绝全零 FileId 的身份门卫、保留父句柄的强 disposition capability 与父句柄相对 namespace 观测源码，但按当前架构铺设策略未编译或运行。topology/journal typed Store、单一 cleanup handle tree、真实 namespace durability、既有执行器对 durable intent/journal/强 disposition 的适配、跨重启恢复和生产 Host 仍未接入；现有测试也没有覆盖 authorization → executor prepare 的完整链路，因此当前不会自动清理生产失败候选。

本文只维护失败候选清理边界。候选本机真源见 `node-plugin-local-authority.md`，健康失败与 quarantine 见 `node-ready-capability.md`，staging 物化见 `node-plugin-archive-extraction.md`。

## 2. 已实现的底层能力

1. 新建受管文件和目录在创建时取得 `DELETE` 权限，但不会因此自动删除。
2. 既有候选 downloads 目录只在最终目录分量上取得删除权，中间前缀继续使用普通 traverse 权限。
3. 原始候选文件以父句柄相对、数据只读、share-none 且携带 `DELETE` 权限的句柄打开。
4. 删除调用 Windows `FileDispositionInfoEx`，绑定已经固定的对象，不重新解析完整路径，也不调用 `remove_dir_all`。
5. 旧 `delete_exact()` 在文件删除成功后消费文件句柄；失败则返回错误和同一文件 custody。它只服务尚未适配的 v1 执行器，返回值不是 completion 或 durability 证据。
6. 旧 `delete_exact()` 在目录删除成功后消费目录句柄；非空、共享冲突或权限错误时返回错误和同一目录 custody。它不会保留后续 namespace 观测所需的父句柄。
7. 非 Windows 平台继续失败关闭，尚未提供 portable beneath/no-follow 删除实现。
8. 每个非根受管文件或目录现在保存从同一对象句柄和同一直接父句柄派生的对象 identity digest、父 identity digest，以及父句柄相对打开时使用的原始单组件名称；可 Clone 的 `ManagedObjectBinding` 只作为 topology 输入，不授予删除权。
9. 独立强入口 `set_delete_disposition_exact()` 会运行时复验对象和父 identity；目录目标还要求 final `Arc<File>` 无别名。成功后明确关闭目标句柄，并返回不可 Clone、持续持有 exact parent handle chain 的 `ManagedDeleteDisposition`。现有执行器尚未调用该入口。
10. disposition 可继续线性观测 `Absent`、`ExpectedIdentityMatch` 或 `IdentityConflict`。Windows 只把父句柄相对 `NtCreateFile` 返回的 `STATUS_OBJECT_NAME_NOT_FOUND` / `STATUS_NO_SUCH_FILE` 当作 absence；delete-pending、共享冲突、权限失败和其他状态都保持不确定。历史 identity digest 相同还必须对象种类相同才会标记为 `ExpectedIdentityMatch`；该名称不宣称仍是原对象，因为目标 handle 已关闭后 FileId 仍可能被文件系统复用。两类 name-present 结果都不提供返回 disposition/absence 的降级通道，不能自动重试或删除该名称下的对象。
11. `ManagedParentRelativeAbsence::make_namespace_durable()` 当前固定返回 `NODE_MANAGED_NAMESPACE_DURABILITY_UNAVAILABLE` 并归还原 absence capability；预/后置 absence 复验、父目录 flush 与后置可信时间绑定完成前，代码不能产生 `ManagedNamespaceDurable`。

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

这些对象目前仍属于预生产 schema v3：缺少新对象或定义不一致的同版本库会失败关闭并要求重建，不提供旧 schema 原地迁移。execution plan 是 authorization v1 之后的第二个 fresh、不可变执行授权边界；在 typed Store 完成并返回 sealed capability 前，单独的 cleanup authorization 或手写 plan rows 都不得触发删除。

cleanup authorization Store 当前执行以下单一事务：fresh read failed slot、精确 quarantine/staging 回执、`owned` owner、inventory/state/authority/process fence 与可信时间；确认没有 prepared fetch/verification 后推进可信时间和 state/authority epoch，inventory 保持不变；随后写入 JCS+SHA-256 不可变授权回执，并把 owner 推进到 `cleanup_pending`。返回的 `AuthorizedCandidateCleanup` 继续持有原候选目录和文件句柄，不能序列化或克隆。提交结果不确定时，进程内 recovery key 只允许读取 `NotCreated` 或 exact `Authorized`；exact adoption 会重新读取权威状态并复验 retained staging 内容。

本批在远程 canonical API 上进一步加固组合语义：授权读与写事务都复验同一 process cancellation source；owner 对账绑定 plugin、slot、generation、release、owner plan 和 application inventory revision；恢复同时重放 quarantine→staging 的规范 JSON/JCS 链并拒绝已有 completion。PlanApply 投影继续把 `cleanup_pending` 计为占用 candidate pointer 的活动 owner，但拒绝任何直接修改该插件的 action；原计划历史重放接受 `cleanup_pending` 与未来 `cleaned`，只在 owner 仍为 `owned` 时返回可操作 candidate handle。当前 inventory invariant 也会让 `cleanup_pending` 期间的 sharing-off plan 失败关闭；“立即停止运行、后台继续保留清理 custody”的独立退出语义仍待设计。

旧式私有物理执行器会在首次删除前复验 retained staging 内容和 cleanup authorization，再尝试从既有受管根固定 candidate 根与 staging 父目录，并按 extracted 文件、staging seal、最深 extracted 目录、staging run、staging 父目录、download 文件、downloads 目录、candidate 根目录排序。它仍调用旧 `delete_exact()`，只聚合进程内 disposition evidence，没有消费 `DurableDeleteIntent`、保留父句柄或追加 journal。全树接受 disposition 后产生的 `PhysicallyExecutedCandidateCleanup` 已保留原始 staging recovery key、同一 cancellation guard、单调完成点、授权/隔离/staging 回执、执行证据和根锁，但这些 custody 不能替代缺失的 journal 中段。

该 prepare 路线还有已确认的 Windows 共享访问风险：普通 retained 目录句柄只声明 `FILE_SHARE_READ`，随后 late-open 同一 candidate/staging 目录却请求 `DELETE`，可能在真正执行前直接得到 sharing violation。不能通过全局追加 `FILE_SHARE_DELETE` 绕过，因为那会允许钉住期间 rename/delete 并削弱 namespace custody。后续应从原始 custody 派生或一开始构造唯一 cleanup handle tree；在此修复和完整链测试前，`PhysicallyExecutedCandidateCleanup` 只代表旧式源码形状，不代表生产可执行或耐久清理。

远程 completion Store 内核会在一个 `BEGIN IMMEDIATE` 中消费不可伪造的 `DurableCandidateCleanupTerminalJournal`，重验 sealed plan、全部对象的 `namespace_durable` 计数与末端摘要、物理证据链、原授权、`cleanup_pending` owner、failed slot、inventory/state/authority/process fence、可信时间和无 prepared 工作；随后只移除精确 failed slot、清空 candidate pointer但保留插件记录与 active slot，推进全部 fence，写入同时绑定 plan/evidence/journal 的不可变 completion receipt，并把 owner 原子推进到 `cleaned`。事务后重读与 recovery 只允许 exact `NotCreated` 或 `Completed`。终态 journal capability 仍无构造入口，所以旧执行器不能绕过缺失的中段直接调用该内核。

## 3. 当前能力不代表什么

底层句柄拥有删除权，不等于业务层已经授权删除。以下对象都不能单独触发清理：

- verification verified/rejected outcome；
- staging receipt；
- health receipt；
- quarantine receipt；
- `failed` slot phase；
- 调用方传入的布尔值、路径或 candidate token。

当前生产路径不会自动删除失败候选，也不会创建新候选、恢复下载或允许重试。私有代码能够签发 cleanup authorization、形成旧式物理执行证据，并具备终态 journal 之后的原子 completion 内核；但 topology/journal 持久化和 namespace durability 中段尚未实现，完整流水线不可达且未由 Host 调用。物理删除成功在 completion Store 落库前不能对外表现为 owner 已释放或候选可重建。

当前 `namespace_durable` 仍只是 schema 合同名称。受管文件系统已形成未编译的对象/父 identity 与父句柄相对 absence/冲突观测源码，但 durability seam 明确不可用；现有旧 `delete_exact()` 和旧执行器均未接入强 disposition，成功不能生成 absence 或 durability 事件，更不能据此写 completion。typed Store 也尚未实现对 plan/object/event JSON、路径摘要和 JCS 摘要的重算读回。

现有物理执行器的 v1 `execution_evidence_digest` 只聚合进程内 delete-disposition 步骤，尚无不可变 evidence row，也未绑定 `execution_plan_digest` 与逐对象 `object_digest`。completion Store 之前必须持久化并重算该证据，或升级为包含这些绑定的 evidence v2；它与 `terminal_journal_digest` 是两条独立证据链，禁止重新合并为同一摘要。

SQLite trigger 只能证明前序 event 在当前事务视图中存在，不能证明它已由更早事务 durable commit。未来 typed Store 必须禁止调用方传入或复用裸 `Transaction`：`delete_intent` 要独立提交、exact readback 后返回非 Clone `DurableDeleteIntent`，物理删除只能消费该 capability；disposition、absence、durability 与 completion 各自使用后续 fresh transaction。不得提供一次事务批量写完四阶段或 completion 的接口。

## 4. 已固定且后续必须保持的事务顺序

完整清理必须拆成以下七个边界：

1. **Fresh authorization**：在 `BEGIN IMMEDIATE` 内重读 failed slot、quarantine receipt、candidate owner、inventory/state/authority/process fence 和可信时间，写入不可变 cleanup authorization，并把 owner 推进到 `cleanup_pending`。
2. **Topology Store**：从仍持有的句柄生成 expected-object topology，在一个事务中写 plan、全部对象和 seal；任何对象缺少 identity 或 parent 都不能进入执行。
3. **Durable intent**：每一步先提交绑定 exact object identity 的 `delete_intent`，再调用删除原语；没有 intent 不得把路径缺失解释为该步骤结果。
4. **Physical execution**：只消费授权对象与保留句柄，按文件、seal、最深目录、staging run、staging 父目录、downloads 目录、candidate 根目录的拓扑顺序执行。
5. **Outcome/partial custody**：同句柄 disposition、父目录相对 absence 与 namespace durability 必须分别追加事件；崩溃恢复只可在已有 intent 后记录受约束的 recovered absence。任何中途失败都返回 terminal journal 位置与剩余句柄，身份冲突永久失败关闭。
6. **Completion Store**：只有所有对象都到达 `namespace_durable` 且 terminal hash 与执行证据一致后，才能把 owner 从 `cleanup_pending` 推进到 `cleaned`、移除 failed slot、清空 candidate pointer，并精确推进 inventory/state/authority fence。
7. **Retry gate**：新候选或旧计划重试只能基于 durable completion outcome；cleanup authorization、quarantine、plan seal 或内存执行成功都不能代替完成回执。

上述 authorization、旧物理执行和 completion 三端已有私有类型或 Store，topology、durable intent、逐步 journal 与 namespace durability 中段仍缺。如果 completion Store 结果不确定，调用方必须只凭 recovery key 查询 `NotCreated` 或 exact `Completed`，不能重复删除、重复释放 owner 或直接开始新候选。

## 5. 目录清理规则

- 清理过程必须持续持有 `ComputePluginRootLockLease`。
- 最终对象必须来自已固定父句柄的相对打开或原 create-new 句柄。
- 目录删除前必须已经消费全部子文件和子目录 custody。
- unexpected entry 会让目录删除失败关闭，不得递归跨越未知 reparse point。
- 路径字符串只用于规范逻辑名称、审计和错误信息，不参与授权性删除 lookup。
- 权限、共享冲突或非空错误不允许降级到普通路径 API。

## 6. 当前验证

远程 canonical 最新组合已通过 `elon-pc-node` 编译和 8 项清理定向测试：

1. 以 create-new 句柄删除精确文件，再删除目录；
2. 非空目录删除失败后保留原目录 custody，删除子文件后用同一 custody 重试；
3. 重新固定既有文件的 cleanup 句柄并完成文件与目录删除。
4. staging 子目录按深度从深到浅、同深度按规范逻辑路径稳定排序。
5. completion receipt JSON 往返后保持稳定 JCS 摘要；
6. completion receipt 拒绝未知字段；
7. completion 投影只删除 failed candidate，保留插件记录与 active slot；
8. completion 投影拒绝重复 candidate slot identity。

前三项 Windows 测试只证明旧底层句柄删除语义，第四项只证明目录排序函数，后四项覆盖 completion 凭证和关键库存投影；它们没有运行 retained custody → cleanup authorization → executor prepare，也没有暴露 late `DELETE` open 的 share-access 冲突。另有 1 项 schema 定向测试证明 cleanup authorization/completion、四张 execution topology/journal 表及相关 trigger 可以安装并按相同指纹重开。本批身份 binding、强 disposition 与 namespace 观测源码未编译或运行；现有验证仍不覆盖 topology typed Store、完整 authorization→journal→completion 事务夹具、journal 化执行、目录 durability、跨重启恢复或生产 Host 接线。

## 7. 下一批实现边界

下一批应先实现只消费 `AuthorizedCandidateCleanup` 与当前 handle-derived binding 的 topology builder、typed Store、exact readback 与 sealed plan recovery；同时把 retained 原始 custody 改造成单一 cleanup handle tree，消除 late `DELETE` reopen 的 share-access 冲突。然后再把既有进程内执行器改造成逐对象消费 `DurableDeleteIntent` 和强 disposition capability、以独立事务追加 disposition/absence/durability 事件，并在全部对象 durable 后构造 completion 内核所需的 `DurableCandidateCleanupTerminalJournal`。随后补齐受约束的 Windows parent namespace flush、严格晚于 flush 的 authenticated trusted-time binding、部分失败/跨重启恢复和完整 authorization→journal→completion 事务夹具，最后接入生产 Host。不得先暴露“按 candidate token 删除目录”的管理接口，否则会绕过 quarantine、owner、topology 和 fence 合同。
