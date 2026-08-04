---
title: 分布式算力激活证据申请控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力激活证据申请控制面

## 1. 当前状态

激活证据申请的 v177 状态机、v178 过期批准废止审计、v179 不可变激活计划、本人 HTTP/MCP 控制面和管理员 HTTP 审核队列已写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。

这套控制面记录“供给者提交了哪些证据摘要、审核人作出了什么决定、后续激活应写入哪一个精确 Provider 合同”。`approved` 只表示当前证据包通过人工审核，`prepared` 只表示不可变候选合同已生成；两者的 `activation_effect` 均为 `none`。当前代码不会把 Provider 或 CapacityPool 改为 active，不会发布 Offer、开放预留或移动资金。

## 2. 申请流程

1. 用户先登记本人 `registering/self_declared` Provider、`registering` CapacityPool、Bucket 和所需 self-declared Supply。
2. 用户提交节点绑定引用、短期 ReadyCapability 摘要、路由证明摘要和硬件观测摘要，并显式确认申请。
3. 服务端确认 Provider/Pool 归属和 `registering` 状态，重新审计当前 Pool epoch，并锁定 Provider/Pool 精确版本及不含检查时间的稳定账本审计摘要。
4. 管理员只能通过登录后的平台 HTTP 队列审核。批准前，服务端再次核对所有权、状态、精确版本和稳定账本审计摘要。
5. 审核通过后，管理员可显式准备一个 v179 激活计划。服务端在同一事务内再次核对申请、Provider、Pool 和稳定账本审计摘要，并生成下一 Provider revision 的不可变目标合同。
6. 后续真正应用计划、激活 Provider/Pool 和发布 Offer 仍需独立、尚未实现的受控状态迁移。

提交的引用和摘要是待审核材料，不是平台已验证事实。服务端不在这张表内保存节点端点、访问凭据、原始硬件报告或完整路由证明。

## 3. 本人 HTTP 接口

全部本人接口要求一龙用户 Bearer 会话，且路径中的 Provider、Pool 和申请必须属于当前用户。

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests` | 显式确认后提交一份证据申请 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests?limit=20` | 列出该 Pool 的本人申请历史 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id` | 读取一份本人申请 |
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id/cancel` | 以当前申请摘要和显式确认取消 submitted 申请 |
| GET | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id/preflight` | 只读检查后续激活条件并返回阻断码 |

提交请求必须带稳定 `idempotency_key`。幂等范围按用户、Provider 和 Pool 隔离；相同键只重放相同证据材料。已成功写入的同一请求可在 Provider/Pool 后续变化后继续重放，但新申请必须重新满足当前依赖。

## 4. 本人 MCP 工具

以下工具加入现有项目级开放商业 MCP，但所有权仍按当前登录用户判断：

| 工具 | 类型 | 作用 |
|---|---|---|
| `compute_submit_my_activation_evidence_request` | 显式确认、幂等写入 | 提交本人 Provider/Pool 证据摘要 |
| `compute_get_my_activation_evidence_request` | 只读 | 读取一份本人申请 |
| `compute_list_my_activation_evidence_requests` | 只读 | 列出本人 Pool 的申请历史 |
| `compute_cancel_my_activation_evidence_request` | 显式确认、CAS 写入 | 取消仍为 submitted 的本人申请 |
| `compute_preflight_my_activation_evidence_request` | 只读 | 检查本人申请的后续激活条件并返回阻断码 |

管理员审核不向 MCP 开放，避免普通 AI 代理获得信任升级审批能力。

## 5. 管理员 HTTP 审核

| 方法 | 路径 | 作用 |
|---|---|---|
| GET | `/api/admin/compute/activation-evidence-requests?status=submitted&limit=20` | `admin/owner` 按状态读取审核队列 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/review` | 以当前申请摘要、决定和显式确认执行审核 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/preflight` | `admin/owner` 只读检查后续激活条件 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/supersede` | 以当前摘要、原因和显式确认废止过期 approved 申请 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan` | 读取该申请已准备的不可变激活计划 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan` | 显式确认后准备下一 Provider revision，不执行激活 |

决定只支持 `approved`、`changes_requested` 或 `rejected`。退回和拒绝必须填写说明；只有 `submitted` 可以审核。批准时如果 Provider/Pool 所有权、状态、版本或账本审计发生变化，服务端失败关闭，要求供给者重新提交。

## 6. 激活就绪预检

本人 HTTP/MCP 和管理员 HTTP 可生成 `compute_federation.activation_preflight.v1` 只读报告。报告逐项检查申请已批准、Provider 所有权和精确版本未变化、Provider 仍为 registering、存在路由、存在 verified 硬件摘要及验证时间、信任层不再是 `self_declared`、服务区域非空、Pool 归属/版本/状态未变化，以及当前账本审计健康且稳定摘要一致。

失败项以稳定阻断码返回，例如 `request_not_approved`、`provider_routing_missing`、`verified_hardware_missing`、`provider_trust_tier_self_declared`、`pool_version_changed` 或 `ledger_audit_changed`。只有没有阻断项时 `ready_for_activation=true`；该值仍只是当前快照，不是授权、锁、SLA 或激活执行。预检的 `activation_effect` 同样固定为 `none`。

## 7. 不可变激活计划

管理员准备计划时必须提交稳定幂等键、当前 `request_digest`、目标 Endpoint 引用、verified 硬件摘要、目标信任层、验证时间和 `confirm_prepare=true`。Endpoint 只允许保存 `endpoint_id`、transport、gateway、address hint 和 `credential_ref` 等引用；数据库不保存凭据正文。

计划固定申请摘要、Provider/Pool 精确版本、稳定账本审计依赖、下一 Provider revision、目标 Provider 规范 JSON、目标 Provider 摘要、Endpoint 摘要、准备人和计划摘要。相同申请只能绑定同一份规范计划；相同幂等键不能改写目标合同。首次写入前，Store 在同一 `BEGIN IMMEDIATE` 事务内复核全部依赖，避免预检后到写入前发生版本漂移。

计划初始状态为 `prepared`。它是后续受控激活的输入，不是当前 Provider 事实；GET/POST 都返回 `activation_effect=none`。当对应 approved 申请被废止时，仍为 prepared 的计划在同一事务内转为 `superseded`。当前没有把计划改为 `applied` 的入口。

## 8. 状态与并发边界

- 首次状态固定为 `submitted`；同一 Provider/Pool 同时只允许一份 `submitted` 或 `approved` 申请。
- 本人只能把 `submitted` 改为 `canceled`，并必须提供当前 `request_digest`。
- 管理员审核使用 `request_digest` 比较交换，申请内容或状态并发变化时拒绝覆盖。
- `changes_requested`、`rejected` 和 `canceled` 结束当前申请；用户可使用新幂等键重新提交。
- 当 `approved` 因 Provider/Pool 版本或其他依赖变化而不再可用时，平台 `admin/owner` 可显式执行 `approved -> superseded`。操作要求当前 `request_digest`、非空原因和 `confirm_supersede=true`，保留原审核字段，并另存废止时间、执行人和原因；相同执行人和原因可幂等重放，对应 prepared 计划同时转为 `superseded`。
- `superseded` 会释放同一 Provider/Pool 的活跃申请唯一约束，使用户可以基于当前版本重新提交；它不撤销任何已发生的激活，因为当前没有激活执行入口。
- `activated` 仍只为后续生命周期保留，当前控制面没有进入该状态的入口。

## 9. 尚未实现

- Cargo 编译、v177-v179 迁移执行、并发和 HTTP/MCP 真实调用验证；
- 节点绑定引用、ReadyCapability、路由证明和硬件观测的真实采集与密码学验证；
- 审核员查看原始证据工件、签名链和挑战任务的界面；
- prepared 计划的独立复核、受控应用、回滚和 Provider/Pool 激活状态迁移；
- verified 硬件事实、路由凭据轮换、Offer 发布、任务派发和真实结算。
