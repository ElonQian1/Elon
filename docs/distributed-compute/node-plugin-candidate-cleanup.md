# 节点插件失败候选清理

## 1. 状态

当前状态为 `partial_implementation_mixed`。Windows 受管文件系统已经具备同句柄删除原语，候选下载校验链也会保留可删除目录与文件 custody；私有 typed cleanup authorization Store、线性授权能力、进程内 outcome-uncertain recovery 和只消费固定句柄的物理执行器已经形成代码，远程 canonical 组合通过编译及 4 项定向测试。SQLite authority schema 又加入 expected-object topology、candidate-parent anchor、四阶段步骤 journal、completion receipt 及 `owned -> cleanup_pending -> cleaned` 门卫；本批 topology/journal schema 增量未重新编译、未运行 DDL 或测试。生产 Host、topology typed Store、受管目录及父级 identity、真实 namespace durability、既有执行器对 journal 的适配、completion Store 与跨重启恢复仍未接入，因此当前不会自动清理生产失败候选。

本文只维护失败候选清理边界。候选本机真源见 `node-plugin-local-authority.md`，健康失败与 quarantine 见 `node-ready-capability.md`，staging 物化见 `node-plugin-archive-extraction.md`。

## 2. 已实现的底层能力

1. 新建受管文件和目录在创建时取得 `DELETE` 权限，但不会因此自动删除。
2. 既有候选 downloads 目录只在最终目录分量上取得删除权，中间前缀继续使用普通 traverse 权限。
3. 原始候选文件以父句柄相对、数据只读、share-none 且携带 `DELETE` 权限的句柄打开。
4. 删除调用 Windows `FileDispositionInfoEx`，绑定已经固定的对象，不重新解析完整路径，也不调用 `remove_dir_all`。
5. 文件删除成功后消费文件句柄；失败则返回错误和同一文件 custody。
6. 目录删除成功后消费目录句柄；非空、共享冲突或权限错误时返回错误和同一目录 custody。
7. 非 Windows 平台继续失败关闭，尚未提供 portable beneath/no-follow 删除实现。

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

私有物理执行器在首次删除前会复验 retained staging 内容和 cleanup authorization，再从既有受管根预先固定 candidate 根与 staging 父目录。随后严格按 extracted 文件、staging seal、最深 extracted 目录、staging run、staging 父目录、download 文件、downloads 目录、candidate 根目录顺序消费句柄。每个成功步骤进入规范执行证据；任一系统删除失败都返回已完成步骤、失败对象的同一句柄、全部剩余句柄和根锁 lease。全树接受删除 disposition 后只产生 `PhysicallyExecutedCandidateCleanup`，该对象继续持有授权/隔离/staging 回执和根锁，等待未来 completion Store。

## 3. 当前能力不代表什么

底层句柄拥有删除权，不等于业务层已经授权删除。以下对象都不能单独触发清理：

- verification verified/rejected outcome；
- staging receipt；
- health receipt；
- quarantine receipt；
- `failed` slot phase；
- 调用方传入的布尔值、路径或 candidate token。

当前生产路径不会自动删除失败候选，也不会释放 candidate owner、清空 candidate pointer、创建新候选、恢复下载或允许重试。私有 Store 能签发 cleanup authorization，私有执行器也能消费该授权并形成物理执行证据，但二者都不等于 completion receipt；物理删除成功在完成事务落库前不能对外表现为 owner 已释放或候选可重建。

当前 `namespace_durable` 只是 schema 合同名称。受管文件系统尚未提供父目录 identity 读取、父句柄相对 absence 复验或 namespace durability 原语，因此现有 `delete_exact()` 成功不能生成该事件，更不能据此写 completion；typed Store 也尚未实现对 plan/object/event JSON、路径摘要和 JCS 摘要的重算读回。

现有物理执行器的 v1 `execution_evidence_digest` 只聚合进程内 delete-disposition 步骤，尚无不可变 evidence row，也未绑定 `execution_plan_digest` 与逐对象 `object_digest`。completion Store 之前必须持久化并重算该证据，或升级为包含这些绑定的 evidence v2；它与 `terminal_journal_digest` 是两条独立证据链，禁止重新合并为同一摘要。

SQLite trigger 只能证明前序 event 在当前事务视图中存在，不能证明它已由更早事务 durable commit。未来 typed Store 必须禁止调用方传入或复用裸 `Transaction`：`delete_intent` 要独立提交、exact readback 后返回非 Clone `DurableDeleteIntent`，物理删除只能消费该 capability；disposition、absence、durability 与 completion 各自使用后续 fresh transaction。不得提供一次事务批量写完四阶段或 completion 的接口。

## 4. 后续必须保持的事务顺序

完整清理必须拆成以下七个边界：

1. **Fresh authorization**：在 `BEGIN IMMEDIATE` 内重读 failed slot、quarantine receipt、candidate owner、inventory/state/authority/process fence 和可信时间，写入不可变 cleanup authorization，并把 owner 推进到 `cleanup_pending`。
2. **Topology Store**：从仍持有的句柄生成 expected-object topology，在一个事务中写 plan、全部对象和 seal；任何对象缺少 identity 或 parent 都不能进入执行。
3. **Durable intent**：每一步先提交绑定 exact object identity 的 `delete_intent`，再调用删除原语；没有 intent 不得把路径缺失解释为该步骤结果。
4. **Physical execution**：只消费授权对象与保留句柄，按文件、seal、最深目录、staging run、staging 父目录、downloads 目录、candidate 根目录的拓扑顺序执行。
5. **Outcome/partial custody**：同句柄 disposition、父目录相对 absence 与 namespace durability 必须分别追加事件；崩溃恢复只可在已有 intent 后记录受约束的 recovered absence。任何中途失败都返回 terminal journal 位置与剩余句柄，身份冲突永久失败关闭。
6. **Completion Store**：只有所有对象都到达 `namespace_durable` 且 terminal hash 与执行证据一致后，才能把 owner 从 `cleanup_pending` 推进到 `cleaned`、移除 failed slot、清空 candidate pointer，并精确推进 inventory/state/authority fence。
7. **Retry gate**：新候选或旧计划重试只能基于 durable completion outcome；cleanup authorization、quarantine、plan seal 或内存执行成功都不能代替完成回执。

如果 completion Store 结果不确定，调用方必须只凭 recovery key 查询 `NotCreated` 或 exact `Completed`，不能重复删除、重复释放 owner 或直接开始新候选。

## 5. 目录清理规则

- 清理过程必须持续持有 `ComputePluginRootLockLease`。
- 最终对象必须来自已固定父句柄的相对打开或原 create-new 句柄。
- 目录删除前必须已经消费全部子文件和子目录 custody。
- unexpected entry 会让目录删除失败关闭，不得递归跨越未知 reparse point。
- 路径字符串只用于规范逻辑名称、审计和错误信息，不参与授权性删除 lookup。
- 权限、共享冲突或非空错误不允许降级到普通路径 API。

## 6. 当前验证

`elon-pc-node` 已通过测试目标编译和 4 项定向测试：

1. 以 create-new 句柄删除精确文件，再删除目录；
2. 非空目录删除失败后保留原目录 custody，删除子文件后用同一 custody 重试；
3. 重新固定既有文件的 cleanup 句柄并完成文件与目录删除。
4. staging 子目录按深度从深到浅、同深度按规范逻辑路径稳定排序。

前三项 Windows 测试证明底层句柄语义，第四项证明既有执行器的目录顺序规则；远程 canonical 的 cancellation、exact chain、PlanApply/replay、schema fence 与物理执行器已通过 `elon-pc-node` 测试目标编译。既有 schema 定向测试还证明原 cleanup authorization/completion 两张表、六个不可变/写入 fence trigger、owner 两个转换门卫和活动候选唯一约束可以安装并按相同指纹重开；cleanup authorization 回执另有稳定摘要与拒绝未知字段测试。本批新增四张 execution topology/journal 表和相关 trigger 未重新编译、未运行 DDL 或定向测试；现有验证也不覆盖 topology typed Store、完整 authorization Store 事务夹具、journal 化执行、SQLite completion 事务、目录耐久 flush、跨重启恢复或生产 Host 接线。

## 7. 下一批实现边界

下一批应先实现只消费 `AuthorizedCandidateCleanup` 的 topology builder、typed Store、exact readback 与 sealed plan recovery；再把既有进程内执行器改造成逐对象消费 `DurableDeleteIntent`、以独立事务追加 disposition/absence/durability 事件并在部分失败时保留 terminal journal 位置与剩余句柄。随后补齐受管目录及父级 identity、真实 namespace durability、completion Store、结果不确定恢复和 owner/pointer/inventory 原子释放，最后接入生产 Host。不得先暴露“按 candidate token 删除目录”的管理接口，否则会绕过 quarantine、owner、topology 和 fence 合同。
