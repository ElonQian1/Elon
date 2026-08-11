---
title: 外部算力池 Adapter Onboarding 来源权威
status: current
reviewed_at: 2026-08-11
owners: backend, ai-economy, security
implementation_status: implementation_partially_verified
---

# 外部算力池 Adapter Onboarding 来源权威

## 1. 权威范围与当前结论

本文冻结 `external_pool` 的第一段服务端接入权威：Provider owner 发起 request，独立平台管理员复核，Store 再按精确摘要执行 immutable apply。该段只建立未来 Adapter route 可以引用的 owner-approved、admin-reviewed 来源；它不实现 concrete Adapter、credential verifier、route resolver、网络或任务执行。

当前 owner request 领域 DTO、Store-private submit/review/apply 与 v221 三账本源码已经写入并可编译，v221 已随完整临时文件 Store 执行迁移、关闭和重开，onboarding request→独立 review→immutable apply 也已执行，同时把通用 Provider 注册对 `external_pool` 封死；没有 HTTP/MCP/PC 入口。apply 的最大效果固定为：

- 登记一份 `provider_kind=external_pool`、`status=registering`、`trust_tier=self_declared` 的 Provider 当前版本与不可变历史；
- 登记一份专用 `external_pool_onboarding` application source；
- 不写 v213 Adapter、credential、route authorization、capability 或 seal 行；
- 不创建 CapacityPool、Bucket、Supply、Offer、Price Snapshot、Job、Reservation、Execution Plan、outbox、Lease、Receipt 或任何结算记录。

因此 onboarding apply 不等于 Provider active、route authorized、credential verified、capacity available、Offer published、任务可派发或收入可结算。

## 2. 不可旁路的三段流程

### 2.1 Owner request

request 必须由未来 Provider 的一龙 owner 账号在已认证会话中显式提交。服务端固定 `provider_kind=external_pool`、`owner_account_id` 和首期 `settlement_account_id`，调用方不能指定其他 owner、冒充平台 service actor 或直接登记 Provider。

request 只允许携带：

- 稳定 Provider ID、显示名称、区域和规范化能力包络；
- 拟采用的 Adapter ID、release version、config revision 与 opaque config digest；
- 可选 `non_bearer_credential_ref`、脱敏 hint、外部证据引用及其 SHA-256；
- 稳定幂等键、明确确认语和有界说明。

`non_bearer_credential_ref` 只能是服务端保险箱或专用网关中的查找引用，不能包含 Token、密码、API key、Cookie、签名 URL、私钥或可直接认证的 bearer 内容。request 只是 owner 声明，不验证该引用存在、可解密或可调用外部矿池。

### 2.2 独立管理员复核

只有平台 `admin/owner` 可以复核 submitted request。最低四眼要求固定为：request owner 与 reviewer 必须是不同账号；reviewer 不能同时充当 Provider owner。`approved`、`changes_requested` 和 `rejected` 都生成绑定当前 `request_digest` 的不可变复核回执，退回或拒绝必须给出有界原因。

复核只能确认“平台允许按这份精确声明准备 registering Provider 来源”。它不能把声明硬件升级为 verified hardware，不能把非 bearer 引用升级为 authenticated credential，也不能证明 Adapter 实现、协议能力、外部主体或结算账户真实有效。

### 2.3 Immutable apply

apply 只接受仍为 approved 的 exact request、不可变 review receipt、稳定幂等键、当前摘要和显式确认。当前 Store-private 源码在一个 `BEGIN IMMEDIATE` 内重新验证 owner、reviewer 分离、Provider ID 未占用、request/review 摘要、目标 Provider 规范 JSON 与全部时间/上限，再原子完成：

1. 以 policy revision 1 登记 `external_pool/registering/self_declared` Provider；
2. Provider `endpoint=None`，`adapter=Some(exact approved ref)`，observed/verified evidence 继续为空；
3. 把 request 推进到 applied；
4. 写入不可变 onboarding application，并 exact readback Provider revision/digest 与 application digest。

任一步失败必须零提交。相同幂等键只允许精确重放；Provider、owner、settlement、Adapter ref、request/review digest 或目标 Provider JSON 任一变化都必须冲突失败，不能覆盖历史。

apply 不接收新业务字段，不能在最后一步改写已审批内容。它可以由复核管理员触发，但只消费既有 exact review；不会因此获得新增裁量权。

## 3. v221 来源绑定

当前 v221 源码增加 `compute_external_pool_onboarding_requests`、`compute_external_pool_onboarding_reviews`、`compute_external_pool_onboarding_applications` 三类账本及其幂等、投影、不可变和状态门卫；整批 DDL 与来源 trigger 替换位于同一 `BEGIN IMMEDIATE` 迁移事务。

v221 同时替换 v213 的 `trg_compute_route_authorization_exact_source`：

- `provider_activation_application` 继续精确引用既有 endpoint activation application；
- `provider_recovery_application` 继续精确引用既有 recovery application；
- `external_pool_onboarding` 精确引用专用 onboarding application 的 `application_id/application_digest/provider_id`，`approved_by_user_id` 继续绑定 Provider owner，并另行闭合独立 reviewer 与 approved review；
- 禁止继续借用 endpoint-only `compute_activation_applications` 伪装外部矿池来源。

v221 目前已编译并在完整临时文件 Store 中完成迁移/重开，Store onboarding request/review/apply、Provider 原子登记与 trigger 已通过 2 项专项；但没有生产调用入口，因此生产环境不会触发该 route source 分支。即使 application 已存在，也只说明 route 获得了一个可审核来源；没有后续 sealed credential verification 与 route currentness custody 时，仍不得写或采用 v213 route rows。

## 4. Generic registration 必须封死

本人 Provider HTTP/MCP 继续只接受 `user_node` 与 `managed_cluster`；通用 `Store::register_compute_provider` 现也直接拒绝 `external_pool`，不会只依赖 HTTP service 的字符串检查。

唯一允许写入首版 external-pool Provider 的入口是 onboarding apply 事务内部的 store-private registration kernel。不得新增 generic admin create、直接表写、通用 Store DTO、MCP 工具或测试 constructor 作为旁路。低层历史登记 kernel可以供已有受控事务复用，但不能独立公开成 external-pool 授权。

## 5. v213 Route Authority 后置门槛

专用 application source 形成后，仍须另批依次实现并验证：

1. Adapter release 先走独立平台管理员四眼复核与 immutable staged admission，再由 purpose-bound resolver 重算 artifact 摘要并形成 Adapter registry/version 的平台来源、实现摘要和 lifecycle；staging 权威见 [`external-pool-adapter-release-authority.md`](external-pool-adapter-release-authority.md)；
2. credential verifier/KMS 或专用网关，以及只返回 sealed verification custody 的真实 producer；
3. credential TTL、revocation 与 cleanup-only horizon；
4. exact 六能力 revision、route authorization/seal、currentness 与历史重放；
5. resolver、transport、prepare/commit/reconcile/cancel、authenticated ACK/event ingress 和 crash recovery。

只有第 1 至 4 项在同一 Store 权威中闭合后，未来 route producer 才能引用 onboarding application 写入 v213 rows。管理员审批、Provider adapter ref 或数据库中存在 non-bearer ref 都不能替代 credential proof。

Adapter release admission 与 Provider onboarding application 是两条正交来源：前者只允许平台继续准备一份候选 release，后者只允许登记一份 exact `external_pool/registering` Provider。两者的服务端源码现可编译，v221/v222 已随完整临时文件 Store 迁移和重开，且两条 Store 状态机各通过 2 项专项；Adapter release 管理员 Service/HTTP 另通过 2 项进程内接口验收，onboarding service/API 与 release 生产部署仍未运行。任何一条都不能单独写 v213，声明的 artifact SHA-256 和 expected verifier binding 也不得描述为已重算或已验证。

## 6. 明确禁线

- 禁止在 onboarding apply 中创建或激活 CapacityPool、Supply、Offer、Price Snapshot 或可预留容量。
- 禁止创建 Execution Plan、Start command/outbox、send authority、ACK、Lease、Runner event、usage、Receipt 或 settlement。
- 禁止联网、解析 concrete Provider 字段、读取 bearer secret、调用 KMS、发送探测或把外部响应记为已认证事实。
- 禁止把申请、复核或 application receipt 描述成主体认证、硬件验证、Adapter conformance、credential verification、外部矿池在线或商业可用。
- 禁止把 Provider-specific 字段放入 Provider/Offer/Job 核心合同；其 opaque config 继续留在 Adapter 边界。
- 禁止让 endpoint-only activation/recovery、本人 Provider API 或 generic Store registration 旁路专用来源。

## 7. 与节点 VFS 的正交边界

本来源权威完全位于服务端控制面，不依赖节点插件 A1/A2。它也不能用来绕过 A2：逐 case Windows SHM、联合关闭、route/registration 与多 Connection 动态证据仍是 production process owner/VFS/register/open 的硬门槛。production VFS、A1 producer、v15、Runtime、Ready 与用户节点派发继续不可达。

外部矿池未来只走 `server_adapter + adapter_execution`，不能借本来源伪装成 user-node endpoint、ReadyCapability 或 Planning Snapshot。

## 8. 未来最小 HTTP 形状

首批代码只需 owner 与管理员 HTTP，不向 MCP 或 PC 下放信任升级写入口：

| 方法 | 路径 | 固定作用 |
|---|---|---|
| POST | `/api/me/compute/external-pool-onboarding-requests` | owner 显式提交声明与 non-bearer 引用 |
| GET | `/api/me/compute/external-pool-onboarding-requests?limit=20` | owner 读取本人历史 |
| GET | `/api/me/compute/external-pool-onboarding-requests/:request_id` | owner 读取一份 request/review/application 投影 |
| POST | `/api/me/compute/external-pool-onboarding-requests/:request_id/cancel` | owner 取消仍为 submitted 的 request |
| GET | `/api/admin/compute/external-pool-onboarding-requests?status=submitted&limit=20` | 管理员读取队列 |
| POST | `/api/admin/compute/external-pool-onboarding-requests/:request_id/review` | 独立管理员按 exact digest 复核 |
| GET | `/api/admin/compute/external-pool-onboarding-requests/:request_id/preflight` | 只读重审 owner、review 与 Provider 冲突 |
| POST | `/api/admin/compute/external-pool-onboarding-requests/:request_id/application` | exact immutable apply，不产生 route |

管理员入口不接受 bearer secret，不返回完整 non-bearer ref；响应只给出 presence、hint、摘要、状态、阻断码和不可变 receipt 身份。

## 9. 本批静态验收边界

当前已用 2 项定向 Rust/SQLite 测试执行完整临时文件迁移、owner submit、独立 approved review、Provider 原子登记、三段幂等重放、关闭重开，以及 owner 自审、非批准 apply、错误确认语与改变历史重放等失败关闭路径。证据见 [`external-pool-onboarding-acceptance.md`](external-pool-onboarding-acceptance.md)。角色权限、并发、HTTP 与生产数据库仍未验证，状态为 `implementation_partially_verified`。

真实 credential verifier、v213 producer、Adapter、网络与 worker 仍未实现；不得以本源码、v221 名称或未来 API 表宣称 onboarding 已可用。
