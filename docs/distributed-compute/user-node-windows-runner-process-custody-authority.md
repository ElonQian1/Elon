---
title: UserNode Windows Runner 进程监管前置 V1 权威草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-process-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner 进程监管前置 V1 权威草案

## 1. 本批结论

本批只冻结并写入 **Windows suspended child + atomic Job custody prerequisite**。它不是 Host start、runtime
transition、完整 sandbox、active health 或 Ready authority。当前源码既没有 retained share-none Runner→locked
loader load-set 的 owned producer，也没有 restricted/AppContainer launch-security producer，因此 Win32 backend 仍是
default-blocked/unreachable；源码中存在 `CreateProcessAsUserW` 不等于真实进程已经创建。

状态固定为：

- design: `draft_frozen`；
- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- code acceptance: `passed=0/failed=0`；
- persistence: `migration/table/writer=none/none/none`；
- feature registration: `unregistered_feature_workflow_unavailable`。

## 2. 为什么先落进程监管前置

现有 `ComputePluginHost` 只运行 legacy in-process LLM；`lifecycle.rs` 只有 DTO 与纯 transition predicate；
`runner_events.rs` 只有 Runner-originated payload，没有认证 IPC。直接增加 `RuntimeStartReceipt` 会让调用方用
scalar/DTO 冒充进程事实。正确首片必须至少持有：

1. 按值消费的 `DurableWorkAdmittedPluginSlot<'root>`；
2. 同一 Runner 文件身份、全量重哈希、loader-compatible 且锁住非系统依赖闭包和 path namespace 的 load-set custody；
3. 从精确 grant 生成、回读并封存的 restricted/AppContainer primary token，以及 process/thread empty-DACL SD；
4. 匿名、不继承、禁止 breakaway 的 Job Object；
5. 由 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 将 Job 原子附加到新 child 的 aligned attribute-list；
6. `CreateProcessAsUserW(CREATE_SUSPENDED)` 返回的 distinct process/primary-thread owned handles，以及从句柄回读的
   PID、TID 与 creation `FILETIME`；
7. 失败时可终止并确认、无法确认时继续保留全部已返回句柄和 source authority 的线性 custody。

本批写入第 3-7 项的 sealed 边界与私有 Windows backend；第 2、3 项 producer 均明确保留为缺口，因此没有可达
production call path。

## 3. Owner graph

私有 owner graph 固定为：

`DurableWorkAdmittedPluginSlot + SealedComputePluginRunnerImage + SealedWindowsRunnerLaunchSecurity`
→ `ValidatedWindowsRunnerProcessPreparation`
→ `CreateJobObjectW + Set/QueryInformationJobObject + PROC_THREAD_ATTRIBUTE_JOB_LIST`
→ `CreateProcessAsUserW(CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT)`
→ `IsProcessInJob + process/thread identity query-back`
→ `PreparedComputePluginRunnerProcess`。

最终 custody 不实现 `Clone`/Serde，继续携带 `'root`，并拥有 admission、load-set/工作目录文件句柄、launch token/SD、
Job、process、primary thread 和 process identity。没有 `ResumeThread` 入口；Drop 先终止并关闭 Job/process/thread，再
释放 launch、image 与 admission authority。

## 4. Sealed load-set 的 owned transition 缺口

现有 `PinnedManagedFile` 在 Windows 以 read/write/delete、share=0 打开。share mode 不可原地修改；原 handle 存活时
无法再开 loader-compatible data handle，关闭原 handle 后短借用 API又不能表达 reopen/identity/hash 任一点失败的
outcome uncertainty。因此未来 bridge 不能是 path/raw-handle getter，也不能是 `&mut` 短借用 transition。

安全方向只能是 owned linear transition：

- 按值消费 exact admitted/archive Runner custody，不能接收 caller path、digest、index；
- 用 parent-relative metadata identity anchor 跨越 share-none handle 关闭窗口；
- reopen 后重证 volume/FileId/type/reparse/link/size/digest，并锁住 executable、cwd、非系统依赖闭包与 loader path
  namespace；
- 成功返回 replacement managed custody 与 opaque sealed load-set；
- 失败区分 `NotTransitioned(original custody)` 与 `OutcomeUncertain(anchor/candidate/rest-of-admission custody)`，后者只进
  recovery/quarantine，绝不返还 scalar retry permit。

`SealedComputePluginRunnerImage` 当前故意没有 producer。owned share transition、DLL/import closure 和 namespace lock
没有实现及动态故障证据前，backend 不可调用，不能称为 Runner 已准备。

## 5. Launch security 前置

受限 token 不能在 child 创建后补装。默认 process/thread DACL 还可能允许同权限 sibling 按 PID/TID reopen 并恢复或
注入 suspended child。因此本批把 launch security 提升为 **create prerequisite**，不再把它列为 resume blocker：

- `SealedWindowsRunnerLaunchSecurity` 私有字段、无 constructor，不暴露可调 token handle；
- primary token 必须是 restricted 或 AppContainer primary token，且 future producer 必须关闭所有 adjust handles，只保留
  `CreateProcessAsUserW` 必需的 least-rights unique handle；
- future producer 必须 query-back integrity、restricted/AppContainer SID、privilege、capability 与 token type，并封存
  canonical profile digest；
- process/thread SD 使用显式对齐的 immutable self-relative buffer；调用紧前重验有效性、精确长度、
  `SE_SELF_RELATIVE`、DACL present/non-NULL/non-defaulted 且 ACE count 为零；
- `SECURITY_ATTRIBUTES.bInheritHandle=FALSE`，`CreateProcessAsUserW.bInheritHandles=FALSE`。

本批源码只回读 primary/restricted/AppContainer 基线和 sealed digest/SD，不实现 token/SD producer，也未动态证明 Windows
ACL 行为。管理员、SeDebug、Host 自身 process-handle 安全或独立 service SID/account 属后续更强隔离问题，不能把此边界
称为完整 sandbox。

## 6. Atomic Job 与 suspended process 顺序

源码顺序固定为：

1. 在 OS 副作用前完成 absolute path、Windows argv quoting、空 allowlist environment、数值转换及 launch-security
   query-back；
2. 创建匿名 Job，设置并回读 `KILL_ON_JOB_CLOSE`、signed `max_processes` active-process limit 和 signed
   `max_memory_bytes` job-memory limit；回读不得出现 breakaway flags；
3. 用 aligned RAII buffer 两阶段初始化 attribute-list，把唯一 Job handle 以 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 写入；
4. 以 exact application path、mutable argv、显式空 Unicode environment、handle-derived cwd、restricted primary token、
   empty-DACL process/thread attributes 调用 `CreateProcessAsUserW`；
5. flags 固定为 `CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW |
   EXTENDED_STARTUPINFO_PRESENT`，不得存在 post-create `AssignProcessToJobObject` fallback；
6. success 返回即必须已在 Job；每个 distinct non-NULL raw handle 各一次进入 `OwnedHandle`，alias/partial contract anomaly
   进入 fail-closed rollback；
7. suspended 状态下以 `IsProcessInJob` 回读，并从 process/thread handles 分别回读 PID/TID、creation `FILETIME` 与 live
   状态；
8. 返回 inert custody，绝不 resume。

显式空 environment 只避免继承 Node 进程秘密，不是未来运行环境。认证 IPC/bootstrap 必须另行定义排序 allowlist，才
可能替换此占位边界。

## 7. Enforcement 边界

本批只形成 restricted/AppContainer token sealed prerequisite、empty-DACL child objects、kill-on-close、process-count 与
job-memory 的源码基线。以下仍固定为 resume blocker：

- authenticated IPC bootstrap；
- CPU millicore、VRAM、disk、network 与 Sidecar uptime enforcement；
- durable stopped→starting Store transaction 与 commit-unknown recovery。

由于 primary thread 永不 resume，本批不能把 Job/token 称为完整 sandbox 或 signed grant enforcement receipt。未来任一
blocker 未关闭时，都必须保持 process suspended 或终止，不能降级启动。

## 8. 失败与 outcome uncertainty

`CreateProcessAsUserW` 前失败只关闭无 child 的匿名 Job，并保留整份 preparation。Job-list 令 successful create 返回的
child 在出现任何 post-create anomaly 前就属于 kill-on-close Job；不存在未 assigned child 分支，也不得 fallback。

post-create 失败先 `TerminateJobObject`，再以 retained process handle 执行 `TerminateProcess` 兜底并 bounded wait，
从而覆盖 membership query error/false。handle contract anomaly 会先在 raw 层识别 NULL/alias，只对每个
distinct returned handle 建立一次 owner；只有 `WAIT_OBJECT_0` 表示 rollback confirmed。若 terminate/wait 不能确认，
failure 继续持有 preparation、Job、process 与 primary-thread handles，并在 Drop 再次执行 fail-safe；Job handle 最先关闭，
由 kill-on-close 保证 child 不脱离 Job。failure 不提供 retry extractor。

PID、path、receipt、candidate health、CLI sidecar record 或 `ComputePluginFetchProcessFence` 都不能恢复 start 权限。

## 9. 不变式与零效果

start material 只绑定 work-admission source/receipt、installation/plugin/slot/release、Plan/grant、Runner path/digest/size/
FileId、loader dependency/namespace digests、entrypoint argv、launch token/SD digests、resource/permission ceiling、runtime
generation before 与 authority/process/clock fences。它同时把以下效果固定为 `none`：

- runtime phase、runtime generation、health、Ready；
- Provider、route、Offer、Capacity、Execution、Attempt、Lease；
- usage、settlement、money。

本批不改 `lifecycle.rs`、local-authority schema、migration、writer、Ready builder、v14/v15、HTTP/MCP/Wire 或控制
WebSocket。source-lineage 的 local currentness、runtime transition、Host runtime 与 v15 session 四项 gap 全部保持
`missing`。

## 10. 后续顺序

1. 实现 share-none admitted Runner→locked loader load-set 的 owned transition，并做 Windows TOCTOU/share/DLL/path
   namespace 故障矩阵；
2. 冻结并实现 launch-security producer，完整 query-back token/integrity/SID/privilege/capability/ACL，并做 Windows
   reopen/resume/injection 负向矩阵；
3. 冻结 authenticated IPC/bootstrap，完成 CPU/VRAM/disk/network/uptime enforcement；
4. 在同一 fresh local-authority owner 下形成 durable `stopped -> starting` exact-successor transaction 与恢复；
5. 只有 Store、同一 process custody、完整 enforcement query-back 和 authenticated IPC 均成功，才新增受控 resume；
6. 再实现 active health、Ready source currentness、v15 session 与服务端 verifier。

任何后续步骤都不能把本批 source draft 的存在描述为 Host runtime gap 已关闭。
