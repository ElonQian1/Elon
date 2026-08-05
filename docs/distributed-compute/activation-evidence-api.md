---
title: 分布式算力激活证据申请控制面
status: current
reviewed_at: 2026-08-05
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力激活证据申请控制面

## 1. 当前状态

激活证据申请的 v177 状态机、v178 过期批准废止审计、v179 不可变激活计划、v180 原子应用回执、v181 紧急隔离回执、v203 第二人复核回执、v204 隔离恢复控制面、本人 HTTP/MCP 控制面和管理员 HTTP 审核队列已写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。

这套控制面记录“供给者提交了哪些证据摘要、审核人作出了什么决定、激活应写入哪一个精确 Provider 合同、谁独立复核了该计划，以及该计划是否被受控应用”。`approved` 只表示证据包通过人工审核，`prepared` 只表示不可变候选合同已生成，第二人复核只固定复核事实；三者的 `activation_effect` 均为 `none`。只有不同于计划准备人的第二名 `admin/owner` 已按精确 `plan_digest` 留下 v203 回执后，应用入口才可能在一个事务内把 Provider 下一版本和 CapacityPool 改为 active，并保存不可变回执。该内部状态变化不连接节点、不读取凭据正文、不发布 Offer、不开放预留、不派发任务，也不移动资金。

PC `/compute-supply` 已写入供给者本人申请、历史状态、审核说明、预检阻断项和 submitted 取消源码；提交对话框只接受节点绑定引用和三个 64 位 SHA-256 摘要，并明确拒绝把凭据、密钥或原始硬件报告放入引用字段。仅平台 `admin/owner` 可见的 `/compute-activation` 已写入按状态审核队列、申请预检、批准/退回/拒绝、approved 废止、计划准备、第二人复核、计划二次预检、精确摘要应用、应用回执、紧急隔离和三段式恢复源码。当前账号若是计划准备人或恢复计划准备人，PC 不提供对应复核按钮；服务端仍是最终强制边界。管理员能力没有下放到本人页面或 MCP。两个页面均尚未构建、运行或发布。

## 2. 申请流程

1. 用户先登记本人 `registering/self_declared` Provider、`registering` CapacityPool、Bucket 和所需 self-declared Supply。
2. 用户提交节点绑定引用、短期 ReadyCapability 摘要、路由证明摘要和硬件观测摘要，并显式确认申请。
3. 服务端确认 Provider/Pool 归属和 `registering` 状态，重新审计当前 Pool epoch，并锁定 Provider/Pool 精确版本及不含检查时间的稳定账本审计摘要。
4. 管理员只能通过登录后的平台 HTTP 队列审核。批准前，服务端再次核对所有权、状态、精确版本和稳定账本审计摘要。
5. 审核通过后，管理员可显式准备一个 v179 激活计划。服务端在同一事务内再次核对申请、Provider、Pool 和稳定账本审计摘要，并生成下一 Provider revision 的不可变目标合同。
6. 不同于计划准备人的第二名管理员以当前 `plan_digest`、稳定幂等键和 `confirm_review=true` 独立复核计划，生成 v203 追加式回执，但不执行激活。
7. 管理员以当前 `plan_digest`、稳定幂等键和 `confirm_apply=true` 应用计划；Store 在同一 `BEGIN IMMEDIATE` 事务内重新核对第二人复核和全部业务依赖，写入 Provider 下一版本、Pool 生命周期事件、申请/计划终态和 v180 不可变应用回执。
8. Provider/Pool 的内部激活与市场发布分离。Offer 已有后续 v182 管理员原子发布流程；节点真实连接、Price Snapshot、可预留容量、任务派发和资金结算仍是独立边界。
9. 若已应用结果需要紧急停止，管理员可按当前 `application_digest` 和明确原因执行 v181 隔离；Provider 当前 active 版本和 Pool 当前 active epoch 在一个事务内转为 quarantined，并保存不可变回执。
10. 修复证据和路由后，管理员可按 v204 准备恢复计划，由不同管理员复核；旧 active Offer 全部退场且恢复预检无阻断时，应用事务追加 active Provider 下一版本和 Pool `quarantined -> active` 事件。恢复不会重发旧 Offer。

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
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/preflight` | 只读复核 prepared 计划当前是否仍具备应用条件 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/review` | 不同于准备人的管理员按精确计划摘要生成追加式复核回执 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/review` | 读取并重新审计该计划的第二人复核回执 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/application` | 以精确计划摘要和显式确认原子应用计划 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/application` | 读取并审计该计划的不可变应用回执 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/application/quarantine` | 以精确应用摘要、原因和显式确认紧急隔离 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/application/quarantine` | 读取并审计不可变隔离回执 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan` | 读取并审计隔离恢复计划 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan` | 显式准备恢复计划，不解除隔离 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/preflight` | 读取恢复阻断项和 active Offer 数量 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/review` | 读取并审计恢复计划第二人复核 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/review` | 不同于准备人的管理员复核恢复计划 |
| GET | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/application` | 读取并审计恢复应用回执 |
| POST | `/api/admin/compute/activation-evidence-requests/:request_id/activation-recovery-plan/application` | 以精确摘要和显式确认原子应用恢复 |

决定只支持 `approved`、`changes_requested` 或 `rejected`。退回和拒绝必须填写说明；只有 `submitted` 可以审核。批准时如果 Provider/Pool 所有权、状态、版本或账本审计发生变化，服务端失败关闭，要求供给者重新提交。

PC `/compute-activation` 复用上述管理员 HTTP 合同，不另建前端状态真源。工作区按状态筛选申请，先显示证据引用和申请预检，再把审核、计划准备、第二人复核、计划应用、过期批准废止、已激活资源隔离和隔离恢复拆成独立确认动作。激活和恢复表单只接收路由、Gateway、配置和凭据引用，不接收凭据正文；第二人复核展示准备人与复核人以及不可变复核摘要，应用按钮只有在对应预检 `ready_for_apply=true` 时可用。服务端写事务仍会重新审计，不能把前端快照当成授权或锁。恢复细节由 `docs/distributed-compute/activation-recovery-api.md` 维护。

## 6. 激活就绪预检

本人 HTTP/MCP 和管理员 HTTP 可生成 `compute_federation.activation_preflight.v1` 只读报告。报告逐项检查申请已批准、Provider 所有权和精确版本未变化、Provider 仍为 registering、存在路由、存在 verified 硬件摘要及验证时间、信任层不再是 `self_declared`、服务区域非空、Pool 归属/版本/状态未变化，以及当前账本审计健康且稳定摘要一致。

失败项以稳定阻断码返回，例如 `request_not_approved`、`provider_routing_missing`、`verified_hardware_missing`、`provider_trust_tier_self_declared`、`pool_version_changed` 或 `ledger_audit_changed`。只有没有阻断项时 `ready_for_activation=true`；该值仍只是当前快照，不是授权、锁、SLA 或激活执行。预检的 `activation_effect` 同样固定为 `none`。

## 7. 不可变激活计划

管理员准备计划时必须提交稳定幂等键、当前 `request_digest`、目标 Endpoint 引用、verified 硬件摘要、目标信任层、验证时间和 `confirm_prepare=true`。Endpoint 只允许保存 `endpoint_id`、transport、gateway、address hint 和 `credential_ref` 等引用；数据库不保存凭据正文。

计划固定申请摘要、Provider/Pool 精确版本、稳定账本审计依赖、下一 Provider revision、目标 Provider 规范 JSON、目标 Provider 摘要、Endpoint 摘要、准备人和计划摘要。相同申请只能绑定同一份规范计划；相同幂等键不能改写目标合同。首次写入前，Store 在同一 `BEGIN IMMEDIATE` 事务内复核全部依赖，避免预检后到写入前发生版本漂移。

计划初始状态为 `prepared`。它是后续受控激活的输入，不是当前 Provider 事实；准备和读取均返回 `activation_effect=none`。当对应 approved 申请被废止时，仍为 prepared 的计划在同一事务内转为 `superseded`。只有 v180 应用入口可以把精确摘要匹配且已完成 v203 第二人复核的 prepared 计划改为 `applied`。

### 7.1 第二人复核回执

复核请求必须由平台 `admin/owner` 发起，并提交稳定 `idempotency_key`、当前 64 位小写 `plan_digest`、可选复核说明和 `confirm_review=true`。服务端要求计划仍为 `prepared`，精确摘要匹配，且复核账号与 `prepared_by_user_id` 不同。同一计划最多生成一份回执；相同幂等请求只能重放完全相同的复核材料。

v203 回执绑定 `plan_id`、`request_id`、Provider/Pool、计划摘要、准备人、复核人、说明、请求摘要和服务端时间，并通过数据库触发器禁止更新或删除。读取和应用时都会重新计算摘要并核对参与者分离。该规则只要求“准备人与复核人不同”，没有实现第三名独立应用人、组织级审批流、硬件证据密码学验证或现实身份背调。

## 8. 计划应用预检

管理员可读取 `compute_federation.activation_plan_preflight.v2` 报告。服务端重新核对计划仍为 prepared、申请仍为 approved 且摘要和依赖绑定一致、当前 Provider 仍为原 registering 版本、目标 Provider 身份和下一 revision 有效、目标合同具备路由与 verified 事实、Pool 仍为原 registering 版本、当前账本审计健康且稳定摘要一致，以及存在精确绑定且参与者分离有效的 v203 第二人复核回执。

失败项使用稳定阻断码，例如 `plan_not_prepared`、`request_digest_changed`、`provider_version_changed`、`target_provider_not_ready`、`pool_version_changed`、`ledger_audit_changed`、`plan_review_missing`、`plan_review_digest_changed` 或 `plan_review_separation_invalid`。只有没有阻断项时 `ready_for_apply=true`；该值只是读取时快照，不是授权、锁、SLA 或执行结果，`activation_effect` 固定为 `none`。

## 9. 受控应用与不可变回执

应用请求必须由平台 `admin/owner` 发起，并提交稳定 `idempotency_key`、当前 64 位小写 `plan_digest` 和 `confirm_apply=true`。应用入口不信任此前的预检快照，而是在写事务内重新审计 v203 第二人复核回执、申请、当前 Provider、目标 Provider 合同、Pool 精确版本和稳定账本摘要。

首次成功应用在同一事务内完成五项变化：登记计划锁定的 active Provider 下一版本；追加 `registering -> active` Pool 生命周期事件；把申请改为 `activated`；把计划改为 `applied`；写入 v180 追加式应用回执。任一步失败均不提交。相同申请和计划只能产生一份应用回执；相同幂等请求只能重放相同计划摘要。

回执返回 `activation_effect=provider_and_pool_active` 与 `offer_effect=none`。回执审计绑定当时的不可变 Provider 历史版本和 Pool 生命周期事件，不要求 Provider/Pool 永远保持 active，因此后续合法升级、draining 或 retired 不会使历史回执失效。该回执只证明平台内部状态迁移已原子提交，不证明节点公网可达、硬件证据经过密码学验证、Offer 已发布、容量已可交易或资金已结算。

紧急隔离请求必须提交当前 `application_digest`、稳定幂等键、非空原因和 `confirm_quarantine=true`。Store 在写事务内重新审计应用回执并读取 Provider/Pool 当前状态；只有二者仍为 active 且归属一致时，才登记 quarantined Provider 下一版本、追加 Pool `active -> quarantined` 生命周期事件并写入 v181 追加式回执。任一步失败均不提交。

隔离回执绑定应用摘要、隔离前后的不可变 Provider 版本、当前 Pool epoch、生命周期事件、原因、执行人和时间，返回 `provider_effect=quarantined`、`pool_effect=quarantined`、`offer_effect=none_direct`。它不删除或改写原应用、计划、申请或 Offer；现有候选发现会因当前 Provider 不再 active 而排除新选择，但隔离不等于撤销既有业务合同、退款或节点关机命令。v204 已用独立、追加式、第二人复核的流程实现最窄恢复，详见 `docs/distributed-compute/activation-recovery-api.md`。

## 10. 状态与并发边界

- 首次状态固定为 `submitted`；同一 Provider/Pool 同时只允许一份 `submitted` 或 `approved` 申请。
- 本人只能把 `submitted` 改为 `canceled`，并必须提供当前 `request_digest`。
- 管理员审核使用 `request_digest` 比较交换，申请内容或状态并发变化时拒绝覆盖。
- `changes_requested`、`rejected` 和 `canceled` 结束当前申请；用户可使用新幂等键重新提交。
- 当 `approved` 因 Provider/Pool 版本或其他依赖变化而不再可用时，平台 `admin/owner` 可显式执行 `approved -> superseded`。操作要求当前 `request_digest`、非空原因和 `confirm_supersede=true`，保留原审核字段，并另存废止时间、执行人和原因；相同执行人和原因可幂等重放，对应 prepared 计划同时转为 `superseded`。
- `superseded` 会释放同一 Provider/Pool 的活跃申请唯一约束，使用户可以基于当前版本重新提交；它只适用于尚未应用的 approved 申请，不撤销已发生的激活。
- 受控应用以一个写事务执行 `approved -> activated` 和 `prepared -> applied`；应用回执、Provider 版本与 Pool 生命周期事件均必须匹配，否则失败关闭。
- prepared 计划只能由不同于准备人的第二名管理员复核；复核回执追加后不可修改或删除，后续废止只改变计划状态而不抹除复核历史。
- 每份激活应用最多产生一份 v181 隔离回执；相同幂等键或相同应用只能重放相同应用摘要和原因。隔离保留申请 `activated`、计划 `applied` 和原应用回执，避免把历史事实伪装成未发生。
- 每份 v181 隔离回执同时只允许一份 prepared v204 恢复计划；恢复前 Provider 下 active Offer 必须全部退场。恢复追加新 Provider 版本和 Pool 生命周期事件，不删除隔离历史，也不自动重新发布旧 Offer。

## 11. 尚未实现

- Cargo/TypeScript 编译、v177-v181/v203/v204 迁移执行、并发、HTTP/MCP 和 PC 真实调用验证；
- 节点绑定引用、ReadyCapability、路由证明和硬件观测的真实采集与密码学验证；
- 审核员查看原始证据工件、签名链和挑战任务的界面；
- 第三名独立应用人、组织级多级审批、prepared 恢复计划废止/替换、重复隔离恢复周期和通用回滚控制面；
- verified 硬件事实、路由凭据轮换、Price Snapshot、任务派发和真实结算；Offer 原子发布已由 `docs/distributed-compute/offer-api.md` 独立维护。
