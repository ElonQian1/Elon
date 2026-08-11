---
title: 外部矿池 Adapter Release Staged Admission 权威
status: current
reviewed_at: 2026-08-11
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Release Staged Admission 权威

## 1. 权威范围与当前结论

本文冻结 `external_pool` Adapter release 的第一段平台来源：一名平台管理员提交精确 release 声明，另一名平台管理员独立复核，Store 再按已批准摘要执行 immutable stage apply。最大效果只是形成一份 `staged` admission source，供未来 Adapter registry producer 重新验证后引用。

`staged` 只表示“平台允许继续准备这份精确候选”，不表示 artifact 已下载、字节摘要已重算、签名或供应链已验证、实现已加载、协议能力已验证、credential verifier 已存在或 route 已获授权。当前已形成领域 DTO/规范摘要校验、Store submit/review/apply/exact readback、v222 三账本/trigger，以及仅限 `admin/owner` 的 Service/HTTP submit→独立 review→stage 入口；服务端编译、完整临时文件 Store 迁移/重开、Store 状态机和进程内 HTTP 接口均已执行。生产部署、MCP/PC 写入口、artifact、verifier 与外部网络仍未运行。

本权威与 Provider onboarding 分开：onboarding application 批准某个 Provider 使用指定 Adapter release/config；release admission 则冻结平台愿意继续审查的 Adapter 候选。任一来源都不能替代另一来源。

## 2. 不可旁路的三段流程

### 2.1 平台管理员提交

当前 HTTP submit 入口只允许已登录的 `admin/owner`，不接受 Provider owner 以本人身份自助登记 release。Service 从认证会话派生 `submitted_by_admin_user_id`，外部 JSON 拒绝该字段和其他未知字段；进程内接口测试已覆盖未登录、普通用户越权及 actor 注入。请求必须采用拒绝未知字段的 DTO、RFC 8785 JCS、SHA-256 摘要和稳定幂等键，并只携带：

- `adapter_id` 与 `release_version`；
- `route_kind=server_adapter`；
- `supported_provider_kinds=[external_pool]`；
- `candidate_artifact_ref`，固定为不含凭据的 `artifact-ref:<opaque-id>`；
- `declared_implementation_sha256`，固定为 64 位小写十六进制的候选字节摘要声明；
- 六项按固定顺序排列的 capability revision：`authenticated_ack`、`authenticated_events`、`cancel_no_start`、`idempotent_commit`、`prepare`、`reconcile`；
- 服务端根据该固定 capability 数组规范计算的 `capability_set_digest`；
- `expected_credential_verifier` 的 `verification_kind/verifier_id/verifier_revision/verifier_digest`；
- 明确确认语、有界说明、`submitted_by_admin_user_id`、提交时间和幂等键。

`candidate_artifact_ref` 不是 URL、下载授权或 bearer secret。`declared_implementation_sha256` 只是预期值；Store 保存它不等于读取过 artifact。`capability_set_digest` 只证明规范 capability 字节一致，不证明实现通过 conformance。`expected_credential_verifier` 只是未来绑定意图；即使字段完整，也不证明 verifier 已注册、当前有效或执行过认证。

首版不接受额外 Provider kind、endpoint route、可执行路径、动态库句柄、网络地址、Token、密码、API key、Cookie、签名 URL、私钥、KMS 明文或 Provider-specific credential/config 正文。

### 2.2 独立管理员复核

reviewer 必须是另一名当前 `admin/owner`，且 `reviewed_by_admin_user_id != submitted_by_admin_user_id`。`approved`、`changes_requested` 与 `rejected` 都生成绑定 exact `request_id/request_digest` 的不可变 review receipt；退回或拒绝必须给出有界原因。

复核只确认元数据形状、候选来源和继续 staging 的平台决定。reviewer 不能把 submitted digest 升级为 recomputed digest，不能声明 artifact 安全、Adapter conformance、六能力可用、verifier current 或外部矿池可连接。

### 2.3 Immutable stage apply

apply 只消费仍为 `approved` 的 exact request/review、稳定幂等键、预期摘要和固定确认语，不接受任何新的 release 字段。当前 Store-private 事务源码重新核对四眼、摘要、时间、投影、`adapter_id/release_version` 冲突和幂等重放，然后只完成：

1. 把 request 从 `approved` 推进到 `staged`；
2. 写入一份不可变 staged admission；
3. exact readback request、review 与 admission 摘要。

任一步失败必须零提交。`applied_by_admin_user_id` 可以是已批准 reviewer 或另一名管理员，但只能机械消费既有决定，不能在 apply 时更改 artifact、capability 或 verifier intent。

## 3. v222 账本源码合同

当前 v222 源码仅增加：

- `compute_external_pool_adapter_release_requests`：规范请求、精确字段投影、`submitted/approved/changes_requested/rejected/staged` 状态与幂等；
- `compute_external_pool_adapter_release_reviews`：一份 request 对应一份不可变四眼复核；
- `compute_external_pool_adapter_release_admissions`：一份 approved request/review 对应一份不可变 staged admission，并对 `adapter_id/release_version` 保持唯一。

三类账本必须有 JSON 投影、追加式历史、禁止覆盖/删除、单调时间和状态来源 trigger。v222 不新增或修改 v213 Adapter/credential/route/seal 行，不替换 v213 source trigger，也不创建 Adapter current root、verifier registry、service actor 或 credential verification receipt。

若未来需要 withdrawal/revocation，必须新增追加式生命周期事实；不得更新或删除既有 admission 来改写历史。没有该 lifecycle 前，后续 consumer 必须把 admission 当作候选来源重新复核，而不是永久有效的执行授权。

## 4. Future registry consumer 的硬门槛

未来 Adapter registry producer 即使读到 staged admission，仍须在独立权威中同时证明：

1. purpose-bound artifact resolver 实际解析 exact `candidate_artifact_ref`，重新计算字节 SHA-256，并与声明值一致；v227 bytes source 只能证明管理员 raw body 的重开摘要与声明相等，不能单独满足“解析 exact ref”或 provenance；
2. 供应链/签名/conformance 证据达到另行冻结的要求；
3. exact verifier version 已由独立 verifier registry 登记、未撤销且适用于 `external_pool + server_adapter`；
4. Provider owner 已向真实平台 service actor 签发仍有效的最小授权；
5. Adapter capability revision 与 verifier binding 经规范重放后仍与 admission 一致。

只有这些事实闭合后，后续事务才可考虑登记 v213 Adapter version，并必须另存 exact admission binding。当前 v213 schema 尚无 admission source companion；不得通过字段相似或同名摘要推断已绑定。

## 5. P0 禁线

- 禁止让 Provider onboarding request/review/application 代替 Adapter release admission，反之亦然。
- 禁止在 staged apply 中读取 artifact、联网、加载代码、调用 verifier/KMS/gateway 或探测外部矿池。
- 禁止把声明的 SHA-256 称为 recomputed、verified、trusted 或 attested。
- 禁止把 expected verifier binding 称为 verifier registry/currentness 或 credential proof。
- 禁止从 request/review/admission 写入 `compute_route_adapters`、`compute_route_adapter_versions`、credential、route authorization、capability 或 seal 行。
- 禁止改变 Provider 状态，或创建 CapacityPool、Supply、Offer、Price Snapshot、Job、Reservation、Execution Plan、outbox、Lease、Receipt 与 settlement。
- HTTP 写入口只允许 `admin/owner`，actor 必须来自认证会话，响应只返回摘要回执；禁止在没有独立权限批次的情况下复用为 MCP/PC 写入口。
- 禁止把 staged admission 描述为 Adapter 可执行、外部矿池在线或商业可用。

## 6. 与后续批次的截止线

当前形成 release staging 权威、已验证 Store 状态机和进程内管理员 Service/HTTP；生产部署与真实环境仍未接线。v227 已另行冻结 server-owned quarantine raw bytes source，但其源码和运行证据尚未形成，且它不解析 candidate ref。credential resolver/verifier、verifier registry、service actor issuance、Adapter registry activation、route issuance、Provider activation、transport 与 authenticated ACK/event 均属于后续独立批次。

后续 route producer 只有在 Adapter registry、sealed credential verification、TTL/revocation、service actor、六能力 currentness 和 onboarding source 在同一 Store 权威中闭合后，才允许写 v213 credential/route/seal rows。release admission 自身永远不跨越这条截止线。

## 7. 当前验收边界

当前 2 项定向 Rust/SQLite Store 测试已执行完整临时文件迁移、submit、精确重放、独立复核、stage、重开读回及失败关闭路径；另 2 项进程内 HTTP 测试覆盖 `401/403`、actor 注入、四眼复核、确认门卫、摘要冲突、脱敏回执与 immutable replay。证据见 [`external-pool-adapter-release-acceptance.md`](external-pool-adapter-release-acceptance.md) 与 [`external-pool-adapter-release-api-acceptance.md`](external-pool-adapter-release-api-acceptance.md)。

状态为 `implementation_partially_verified`。Store 状态机已运行不等于生产数据库已经升级、角色权限已经验证、入口已经开放或任何生产 Adapter 能力已经存在；并发、异常断电、HTTP、artifact、verifier、route 与网络仍未验证。
