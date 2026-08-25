---
title: UserNode Windows Runner 进程监管前置 V1 验收草案
status: draft
reviewed_at: 2026-08-26
owners: node, compute, windows
proposed_feature_id: compute-user-node-windows-runner-process-custody-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# UserNode Windows Runner 进程监管前置 V1 验收草案

## 1. 本批证据等级

本批只有 source-written 证据。用户要求架构铺设阶段不编译、不运行、不执行 migration 或真实验证，因此新增 Rust
source-contract guard 也未执行：

- implementation: `source_written/source_review_only/implementation_uncompiled`；
- runtime: `implementation_unrun`；
- passed/failed: `0/0`；
- migration/table/writer: `none/none/none`；
- dynamic Windows evidence: `0`。

格式化、文本、diff、体积与模块化检查只属于交付卫生，不提高运行成熟度。

## 2. 文件责任

| Owner | 文件 | 责任 |
|---|---|---|
| private facade | `runtime_process_custody.rs` | 路由 sealed model、policy、encoding、launch security、Job 与 Windows backend；声明无 resume/Store/Ready |
| linear model | `runtime_process_custody/model.rs` | work-admission、load-set、launch security、Job/process/thread 与 PID/TID/creation-time custody |
| signed policy | `runtime_process_custody/policy.rs` | 精确来源 binding、creation prerequisites、resume blockers 与零效果 |
| Windows encoding | `runtime_process_custody/encoding.rs` | absolute UTF-16 path、argv quoting、显式空 environment |
| launch security | `runtime_process_custody/launch_security.rs` | 无 producer 的 restricted/AppContainer token owner、aligned empty-DACL SD 与 query-back |
| Job owner | `runtime_process_custody/windows_job.rs` | Job set/query-back、aligned `PROC_THREAD_ATTRIBUTE_JOB_LIST` RAII |
| Windows backend | `runtime_process_custody/windows.rs` | atomic-Job `CreateProcessAsUserW`、membership/identity query、linear custody/rollback |
| source review | `runtime_process_custody_source_contract_tests.rs` | 未运行 guard，固定 owner、调用顺序、负边界和证据状态 |

## 3. 静态源码审阅目标

源码应满足：

1. sealed load-set 与 sealed launch security 均没有 producer；prepare 与 backend 各只有定义、无调用方或 facade re-export；
2. load-set 持有 executable/cwd、非系统 dependency files 与 namespace directory custody，不暴露 path/raw/files/clone
   getter；
3. preparation 与最终 process custody 按值拥有 `DurableWorkAdmittedPluginSlot<'root>`、load-set 与 launch security；
4. launch token 必须 query-back primary 且 restricted/AppContainer profile 不漂移，token handle 不可继承；
5. process/thread SD 必须 aligned、self-relative、长度/digest 精确，DACL present/non-NULL/non-defaulted 且 ACE count=0；
6. Job 在 process 前创建并 set/query-back kill-on-close/process-count/job-memory，拒绝 breakaway flags；
7. attribute-list 两阶段 aligned 初始化，Job handle value 保持稳定，Drop 删除 opaque list；
8. 只直调 `CreateProcessAsUserW`，使用 exact token/SD/path/argv/environment/cwd 与 false handle inheritance；
9. creation flags 必须包含 suspended/Unicode/no-window/extended-startup，且不得存在 post-create Assign fallback；
10. success 后必须 `IsProcessInJob`；raw NULL/alias 先识别，每个 distinct handle 只进入一次 `OwnedHandle`；
11. PID/TID 必须分别从 process/thread handles 回读，creation time 只从 process handle读取；
12. source slice 不存在 `ResumeThread` 或通用 spawn；
13. post-create 失败必须 terminate Job、以 retained process handle 兜底 terminate、再 bounded wait，未确认时保留全部
    OS/source custody；最终 custody Drop 先终止 Job；
14. 无 runtime/health/Ready/Provider/route/Offer/Capacity/Execution/Attempt/Lease/usage/settlement/money 写入。

这些目标目前只作为未运行 Rust guard 与人工 source review 的目标，不能记为 passed。

## 4. 明确未验收矩阵

| 轴 | passed | failed | unrun | 当前结论 |
|---|---:|---:|---:|---|
| Rust 编译 / Windows 链接 | 0 | 0 | 1 | 未编译，Win32 签名与 feature 未由 compiler 证明 |
| source-contract Rust test | 0 | 0 | 1 | guard 已写但未运行 |
| share-none→locked loader load-set | 0 | 0 | 1 | owned producer 不存在，backend 不可达 |
| restricted/AppContainer token + SD producer | 0 | 0 | 1 | sealed type 无 constructor |
| `CreateProcessAsUserW` + atomic Job-list | 0 | 0 | 1 | 未在 Windows 运行 |
| nested Job / no-breakaway | 0 | 0 | 1 | 未运行 |
| rollback / leak / orphan | 0 | 0 | 1 | 未注错、未观察 handles |
| argv / Unicode / environment | 0 | 0 | 1 | 仅源码，未对真实 Runner 验证 |
| complete token/ACL isolation / sandbox | 0 | 0 | 1 | producer、完整 query-back 与动态攻击矩阵不存在 |
| CPU/VRAM/disk/network/uptime | 0 | 0 | 1 | resume blocker |
| authenticated IPC / health | 0 | 0 | 1 | 不存在 |
| runtime Store / recovery | 0 | 0 | 1 | 无 schema/table/writer |
| Ready / v15 / server verifier | 0 | 0 | 1 | 不存在 |
| Provider / market / money | 0 | 0 | 1 | effect=none |

`failed=0` 只表示没有执行失败项，不表示通过。

## 5. 未来动态故障矩阵

解除架构阶段禁令后，至少验证：

- share-none owned transition 的 NotTransitioned/OutcomeUncertain、same path/different FileId、rename/swap/reparse/hardlink、
  share-mode 冲突、重哈希漂移、DLL/import closure 与 namespace mutation；
- restricted/AppContainer token type、integrity/SID/privilege/capability 漂移、adjust-handle 残留、default/NULL/wide DACL、
  sibling reopen/ResumeThread/CreateRemoteThread 与 admin/SeDebug 边界；
- 空格、反斜杠、引号、Unicode、NUL、超长 argv/cwd 及 environment secret non-inheritance；
- Job create/set/query/attribute-list/create/membership 各点失败；
- parent 已在兼容/不兼容 Job、旧 Windows 不支持 Job-list、breakaway 拒绝、process/memory query-back 漂移；
- NULL/aliased process/thread handles、PID/TID mismatch、creation time 失败、suspended child 早退；
- terminate 失败、wait timeout/failed、already-exited、Drop 二次 fail-safe、Job kill-on-close 与 orphan scan；
- 多次并发准备、custody drop、Node crash/restart 后残留检查；
- 后续 IPC/enforcement/Store 形成后，resume 前任一 blocker 失败都保持 suspended 或终止。

## 6. 负向验收

以下任一声明均为失败：

- 声称本批已创建、启动或运行真实 Runner；
- 把 suspended process custody 称为 Host runtime、完整 sandbox、transition receipt、health 或 Ready；
- 把 Job/token/empty DACL 称为 CPU/VRAM/disk/network/uptime 或完整 grant enforcement；
- 用 path、PID、caller digest、CLI sidecar 或 Runner `Started` 事件替代 owner custody；
- 声称 source-lineage 四项 gap 任一已经关闭；
- 声称 Provider active、route、Offer、Capacity、Execution、Attempt、Lease、计量、结算或资金效果；
- 声称编译、测试、Windows 动态矩阵、migration 或生产验收已经完成。
