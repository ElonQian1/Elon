---
title: 节点端点凭据与认证会话权威
status: current
reviewed_at: 2026-08-10
owners: backend, node, ai-economy
implementation_status: implementation_unwired
---

# 节点端点凭据与认证会话权威

## 1. 当前状态

服务端 schema v216 已形成一组默认空、生产不可达的节点端点凭据与认证会话账本，以及只接受 sealed 输入的 Store 内核。这里的“服务端 v216”属于云端 SQLite 迁移序列；它与节点本机插件 authority 的 v7 schema、以及形成该节点源码的历史 v216 实现批次不是同一个版本域。

本批只铺耐久 currentness：不从 `node_credentials` 或 `ELON_AGENT_SECRETS` 回填，不修改节点注册 HTTP、`/agent/ws`、`AgentManager`、`NodeRegistry`、NodeAgent、`homecli-proto` 或协议阈值，也没有 secure transport proof 的生产构造器。现有节点因此不会自动进入新账本，新表不会改变 legacy 登录、续约、连接或断开行为。

`store_migrations.rs` 已登记 v216，所以部署包含该源码的新二进制时会尝试创建空 schema；“dormant”不是“不运行 migration”，而是 migration 成功后没有 producer 或调用方。DDL 或 migration 失败仍可能让服务启动失败，本批未执行 migration，不能宣称磁盘兼容已验证；成功建表且 child row 仍为空时，新表不在 legacy 表上安装 trigger，也不参与 legacy auth 查询或写事务，不会选择性拒绝旧路径。

该源码尚未编译、执行迁移、测试或运行验证。它不表示节点在线，不表示某个 WebSocket 可用于算力控制，也不签发 ReadyCapability、Provider route、Start outbox、command、Lease 或派发权限。

## 2. 为什么需要独立权威

现有 WS 会话适合继续承载兼容开发节点功能，但不能直接提升为任务级算力端点权威：

- legacy secret 没有版本、撤销历史或耐久 currentness；
- Register 中的 owner、install、版本和 capability 都是节点自报事实；
- 进程内随机 `session_id` 已被封进不可序列化的 `AgentProcessSessionKey`，它只用于同进程连接替换，不能证明底层 credential 仍是当前版本；
- `AgentManager` 和 `NodeRegistry` 仍以 `agent_id` 索引，但当前源码已让 entry 保存 exact process key，并让旧 reader 的 touch、capability update、unregister 与 manager mutation 对 key 做比较；
- 当前公开默认地址仍包含 `ws://` / `http://`，服务器也没有可用于本权威的受信 TLS transport evidence producer；
- sharing、preparation 与 Planning Snapshot 的 ACK-derived 同步 Store 写已在 process-key read lease 内成组执行，replacement 不能穿越该本地窗口；这仍不能替代 ACK 落账事务中的耐久 credential/session currentness 重验。

因此端点凭据、认证 receipt、会话 current head 与在线 socket 必须分层。只有未来同批桥接完成后，安全传输、当前凭据和当前会话的交集才可能成为 compute endpoint authority。

## 3. v216 五表

| 表 | 保存的事实 | 明确不代表 |
|---|---|---|
| `node_endpoint_credentials` | stable credential ID、唯一 agent current head、当前 revision/digest 与生命周期状态 | bearer 正文、节点在线或 socket 可写 |
| `node_endpoint_credential_versions` | 每个 credential 的追加式精确版本、owner/install binding、verifier commitment、owner authorization basis kind/ID/digest 与前代链接 | 历史版本仍可认证 |
| `node_endpoint_credential_revocations` | 对精确 revision/digest 的追加式轮换、恢复或终态撤销事实及 owner authorization basis kind/ID/digest | 已经关闭某个内存连接 |
| `node_endpoint_session_authentication_receipts` | sealed WSS verifier evidence 下认证得到的 credential、owner/install、server instance、agent/proto version、capability digest、session generation 与前代 receipt 精确绑定 | 当前仍在线、Ready、route、command、Lease 或远端执行 |
| `node_endpoint_session_heads` | 每个 agent 当前认证会话的 exact credential、generation、receipt 和受控 active/terminal 投影 | 跨进程永久在线或网络送达 |

五表均使用稳定主键、必要的复合唯一约束与 `WITHOUT ROWID`，版本、撤销和认证 receipt 保持追加式。DDL 只约束新权威表，不在 legacy `node_credentials` 上安装触发器，也不扫描或升级旧行。新表以外键把 agent/owner 锚定到既有 `node_credentials`/`users`，但本批不写这些父表；未来 issuer 创建第一条 child row 前，必须先把 legacy duplicate merge/delete 改为 endpoint-aware，否则 `ON DELETE RESTRICT` 会按设计拒绝删除已成为身份锚点的节点行。

## 4. 凭据生命周期

凭据 current head 只允许以下转换：

```text
absent -> active revision 1
active revision N -> exact rotated/recovered revocation N + active revision N+1
active revision N -> exact owner/security revocation N + terminal revoked
terminal revoked revision N -> active recovery revision N+1 using that exact terminal revocation
```

rotate 与 recover 各自必须保留 owner authorization basis，并把 kind/ID/digest 写进 canonical version 与 revocation envelope；“知道 install_id”不能自动等价于持有旧端点凭据。active head 上推进新版本时必须同时看见旧 revision 的 `rotated`/`recovered` revocation、新 revision 以及不再 active 的旧 session head。若旧 head 已因 exact `owner_revoked`/`security_revoked` revocation 进入 terminal revoked，recovery 必须复用该终态撤销来签发 N+1，不能为同一 N 伪造第二条 `recovered` revocation。终态 revoke 必须绑定相同 revision/digest，不能用 later mutable head 改写历史。

credential version 只保存验证承诺，不向 API 或协议返回 secret hash。完整明文 secret 仍只能在未来安全签发响应中显示一次；当前 HTTP 注册路径没有接入这里。

## 5. 认证会话与 exact CAS

认证 receipt 必须由 Store 从字段私有、不可反序列化且无公共构造器的输入生成。未来生产 issuer 至少要在一个事务中完成：

1. 验证受信 WSS/TLS transport proof；不得把 URL scheme 或普通代理头当证明；
2. 重验 credential current revision/digest、未撤销状态及 owner/install exact binding；
3. 规范化并摘要 proto/capability set；
4. 绑定当前 server instance，生成 session ID；
5. 以 previous generation/receipt 为前提追加 authentication receipt；
6. 用 generation `+1` CAS 把 session head 推进为新的 active head；
7. exact readback 后提交。

当前 sealed domain 的 `NodeEndpointSessionBinding` 已精确携带 agent/credential、credential revision/digest、session ID/generation、receipt ID/digest 与 server instance。未来 `AgentEntry`、`NodeEntry` 应保存该 binding 或由它派生的字段私有 handle；替换、关闭、心跳、capability 更新、敏感 ACK 和断开清理都只能对这个 exact key 做 CAS，不能只传 `agent_id` 或裸 `session_id`。

当前新增的 `AgentProcessSessionKey` 是这一桥接前的窄前置层：`AgentEntry` 与 `NodeEntry` 只保存一份 key，安装连接时固定 `AgentManager.agents -> NodeRegistry.nodes` 锁序；旧 entry 先收到同步 shutdown，再在锁外清理 pending/ACK；旧 reader 的 touch/update/unregister 均为 no-op。`with_current_process_session` 只允许在 manager read guard 内运行无 `await` 的同步操作，三条插件观察链用它保护 ACK-derived Store 写与下一条 durable intent 的准备。该 key 没有 credential revision、receipt、server instance 或跨进程恢复能力，不能进入任何 compute authority digest。

authentication digest 覆盖固定 `bearer_sha256` authentication method、agent version、capability digest 及 sealed WSS verifier evidence；capability 原文只作为物理投影保存，不另进 canonical envelope。当前 sealed contract 把一次认证会话冻结为 15 分钟绝对有效期；后续若需要继续连接，必须形成下一代 receipt/head，不能延长或改写旧 receipt。

session head 的既有 active 只允许精确转为 `closed`、`stale`、`credential_rotated` 或 `credential_revoked`，或者被 generation `+1` 的新 active receipt 取代。服务重启后必须通过 server-instance binding 或启动收口让旧 active head 不再具有 currentness；receipt 只证明认证发生时的绑定，耐久 head 只证明事务检查时的 currentness，两者都不能冒充 live socket。

## 6. Legacy 禁线

以下任一事实，单独或任意组合，均不得成为 compute endpoint authority：

- `ELON_AGENT_SECRETS` 命中；
- legacy `node_credentials` 行、`secret_hash` 或 owner/install 展示字段；
- Register 自报的 owner、install、version、proto 或 capabilities；
- raw WebSocket `session_id`、Planning Snapshot 的 `cloud_session_id`；
- AgentManager map、`cmd_tx`、pending waiter 或 NodeRegistry online/TTL/touch；
- Ping/Pong、hardware、models、storage、dev runtime 或 lifecycle；
- public-development handshake、用户登录 token 或 install-id silent renew；
- `ws://` / `wss://` 字符串、`Forwarded` / `X-Forwarded-Proto` 等未经受信 verifier 认证的请求值；
- 当前 sharing、preparation 或 Planning Snapshot ACK；
- Provider endpoint ref、route credential/authorization 历史或 outbox 历史。

legacy WS 可继续服务既有开发节点能力，但必须在未来桥接时剥离 compute-sensitive effective capabilities。不能因为旧节点声明了新 capability 就发送或接受插件规划、Ready 或 Attempt 权威事实。

## 7. 未来必须同批完成的桥接

不得先接其中一半再开放 producer。一次可启用的桥接批必须同时覆盖：

1. `server/src/node_register_api.rs`：把新签发、持有旧凭据的 rotate、显式 recover 和 terminal revoke 分开；凭据签发 HTTP 本身也需要受信 HTTPS proof。credential 事务先使旧 session head 非 current，提交后才 best-effort 关闭 exact 内存会话。
2. `server/src/homecli_agent.rs::agent_ws_handler` 与 `server/src/homecli_agent/agent_session.rs`：用受信 transport verifier 和单一 Store 认证事务替代 env-first、hash 查询后再查 metadata 的两段判断。
3. `server/src/homecli_agent.rs::{AgentEntry,AgentManager}`：process-local exact key、替换与同步 fence 已铺；未来必须换成或联结耐久 `NodeEndpointSessionBinding`。现有 agent-id-only `close_agent_session` 仍是 legacy facade，不能用于安全撤销。
4. `server/src/node_registry.rs::{NodeEntry,NodeRegistry}`：process-local `register_exact`、`update_capabilities_exact`、`touch_exact` 与 `unregister_exact` 已铺；未来 online/candidate reader 仍须联结耐久 current binding，不能把本地 map 当认证事实。
5. `server/src/homecli_agent/compute_plugin_sharing.rs` 及其 `install_plan_preparation.rs`、`install_plan_planning_snapshot.rs` 子叶：dispatch 与 ACK 已携 process key，ACK-derived 同步 Store closure 已阻止本地 replacement 穿越；未来敏感 observation 仍必须在同一 Store 事务中重验耐久 session head。
6. `server/homecli-proto/src/lib.rs`、`compute_plugin_sharing.rs`、`compute_plugin_install_plan_preparation.rs` 与 `compute_plugin_install_plan_planning_snapshot.rs`：dispatch request 和 observed ACK 都冻结 exact endpoint binding，不能继续只依赖 cloud session UUID 或 authorization 自报。
7. `server/src/node_agent_registration.rs`、`node_agent_config.rs`、`node_agent_cloud_connection.rs`、`node_agent_session.rs` 与 `server/homecli-proto/src/lib.rs`：注册响应、持久凭据、WS bearer 和 Register 帧均携 credential ID/revision；process-local credential epoch 只防本机换证竞态，不能冒充云端 revision。
8. `server/src/node_register_api.rs`、`node_agent_config.rs` 与 `node_agent_admin_open.rs` 的生产地址及部署配置：公网凭据签发必须 HTTPS、节点通道必须 WSS；loopback 或显式开发期不安全通道只能进入 legacy/non-compute 分支。
9. legacy `node_credentials` 的 duplicate merge/delete：在 endpoint child row 可能存在前改为显式迁移或拒绝，不得继续直接删除新权威引用的父节点身份。
10. 完成上述闭包后才允许升级 protocol/capability，并且仍需单独完成 Planning Snapshot producer、Ready 构造器、route issuer 与 outbox worker。

## 8. 与 Ready、Route 和派发的关系

端点会话权威只回答“平台在一个安全传输上认证了哪个当前凭据版本，以及哪个 exact session generation 仍是 current”。它不回答：

- 节点插件 inventory、work-admission、Runtime 或 Sidecar 是否就绪；
- 硬件是否 verified，或 ReadyCapability 是否有效；
- Provider/Offer 是否 active，容量是否可预留；
- route credential、service actor 或 Lease authority 是否已签发；
- Start outbox 是否可发送、ACK 是否可信、Runner 是否运行；
- 用量、验收、结算或付款是否发生。

这些事实必须继续由各自的 sealed Plan、Ready、route、outbox、execution receipt 和 settlement 账本证明，不能从 endpoint session 推断。

## 9. 实现入口与验证边界

- migration：`server/src/node_compute_sharing_migration.rs` 的 `migration_v216` 与 `server/src/node_compute_sharing_migration/endpoint_authority*.rs`；
- domain：`server/src/node_compute_sharing/endpoint_authority.rs`；
- Store：`server/src/store/node_credentials/endpoint_authority.rs`，子叶为 `credentials.rs`、`credentials/{root,rows,write}.rs`、`secret.rs`、`sessions.rs` 与 `sessions/{head_rows,receipt_rows}.rs`；
- process-local fencing：`server/src/node_registry/session_key.rs`、`server/src/homecli_agent/session_fencing.rs`，以及 exact-key 接线后的 `agent_session.rs` 与三条插件 observation 子叶；
- migration registry：`server/src/store_migrations.rs`。

当前 Store surface 只有 crate-internal `issue_fresh_node_endpoint_credential`、`rotate_node_endpoint_credential`、`recover_node_endpoint_credential`、`revoke_node_endpoint_credential`、`authenticate_node_endpoint_session`、`close_node_endpoint_session`、`inspect_node_endpoint_session_currentness`、`restart_node_endpoint_sessions` 与 `recover_node_endpoint_session_heads`，且全部限制为 `pub(in crate::store)`。

输出 `NodeEndpointCredentialMutationReceipt` 与 `VerifiedCurrentNodeEndpointSession` 也保持相同可见性，没有 constructor、`Deserialize` 或 `Clone`；WS/HTTP 所在模块当前不可见。未来 HTTP/WS 桥接必须通过 Store-owned facade 注入 sealed authorization/transport 输入，不得直接公开 domain 构造器或把这些 kernel 提升为网络 API。

本批没有 HTTP、WS、AgentManager、NodeRegistry、NodeAgent 或协议调用点，没有 legacy backfill，也没有 secure transport proof 的生产构造器。只允许把它报告为 `implementation_unwired`；尚未编译、测试、执行内存或磁盘迁移，也未进行 TLS、网络、并发、崩溃恢复或真实节点验证。
