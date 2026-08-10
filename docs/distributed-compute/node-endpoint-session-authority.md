---
title: 节点端点凭据与认证会话权威
status: current
reviewed_at: 2026-08-10
owners: backend, node, ai-economy
implementation_status: implementation_unwired
---

# 节点端点凭据与认证会话权威

## 1. 当前状态

服务端 schema v216 已形成节点端点凭据与认证会话账本及 sealed Store 内核；v217 追加目标绑定的所有者近期重认证回执，v218 再追加每次 mutation 只能消费一次的回执账本。这里的“服务端 v216-v218”属于云端 SQLite 迁移序列；它与节点本机插件 authority 的 v7/v8 schema、以及形成节点源码的历史实现批次不是同一个版本域。

v216 migration 本身不从 `node_credentials` 或 `ELON_AGENT_SECRETS` 回填，也不在 legacy 表上安装反向 trigger。后续增量已经显式接入 secure owner/bootstrap API、Windows NodeAgent issue/recover client，以及 legacy 注册、WS、Manager/Registry 和 waiter 的 no-downgrade 栅栏，但仍未修改 `homecli-proto` 或协议阈值。默认关闭的 direct-TLS-only listener 把同一 rustls TLS 1.3 连接的一次性中性 evidence 消费为 request-bound owner API proof；plain listener、legacy `/agent/ws`、URL/Host、`Forwarded`/`X-Forwarded-Proto` 和静态配置标记均拿不到 proof。只有 owner 明确启用安全配置并完成 mutation，节点才会进入新凭据账本。

`store_migrations.rs` 已连续登记 v216-v218，所以部署包含该源码的新二进制时会尝试创建这些 schema。DDL 或 migration 失败仍可能让服务启动失败；本批未执行 migration，不能宣称磁盘兼容已验证。新表不在 legacy 表上安装反向 trigger；只有 owner 明确调用 secure API 后才会创建 endpoint child row，随后 legacy duplicate delete 会被外键按设计拒绝。

该源码尚未编译、执行迁移、测试或运行验证。它不表示节点在线，不表示某个 WebSocket 可用于算力控制，也不签发 ReadyCapability、Provider route、Start outbox、command、Lease 或派发权限。

## 2. 为什么需要独立权威

现有 WS 会话适合继续承载兼容开发节点功能，但不能直接提升为任务级算力端点权威：

- legacy secret 没有版本、撤销历史或耐久 currentness；
- Register 中的 owner、install、版本和 capability 都是节点自报事实；
- 进程内随机 `session_id` 已被封进不可序列化的 `AgentProcessSessionKey`，它只用于同进程连接替换，不能证明底层 credential 仍是当前版本；
- `AgentManager` 和 `NodeRegistry` 仍以 `agent_id` 索引，但当前源码已让 entry 保存 exact process key，并让旧 reader 的 touch、capability update、unregister 与 manager mutation 对 key 做比较；
- 当前公开默认地址仍包含 `ws://` / `http://`；只有独立 direct-TLS listener 上的精确 owner 路径能把中性 connection evidence 绑定为 owner HTTPS proof，已绑定的 endpoint/owner audience 不能互转；
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

credential version 只保存验证承诺，不向 API 或协议返回 secret hash。新 secret 由服务端 CSPRNG 生成，只在同一 direct-TLS response permit 的首次成功响应显示一次；响应丢失后 exact replay 只返回元数据与 `NODE_ENDPOINT_SECRET_NOT_REPLAYABLE`，必须发起新的 recovery request。

### v217 所有者近期重认证回执

v217 新增 `node_endpoint_owner_reauthentication_receipts`，保存一条不可变、目标绑定的近期重认证事实。回执同时封存 account session 与账号状态摘要、认证方法及 factor/evidence 摘要、endpoint mutation action/request/target、预期 credential head，以及独立的 secure owner-API transport evidence；fresh registration 要求 endpoint root 尚不存在，rotate/recover/revoke 则要求预期 revision/digest 与当前 root 精确一致。回执固定五分钟绝对有效期，secure transport evidence 到重认证最多相隔 30 秒，使用 RFC 8785 JCS 与 SHA-256，按 owner + issuance request 精确重放。表为 `WITHOUT ROWID`，拒绝所有唯一键 replacement、UPDATE 与 DELETE，并以外键和触发器锚定当前 owner/session/node target。

v217 回执不提供 standalone 网络入口。唯一生产路径位于 v218 复合 Store facade：同一事务从 bearer 对应的当前 `sessions JOIN users` 行、当前密码 factor、exact target 与 request-bound owner TLS proof 派生 sealed 输入并立即写入。`trusted_device`、`last_seen_at`、`OWNER_TOKEN`、普通登录结果、Google login/bind challenge、恢复码或已经绑定为 endpoint WSS audience 的 proof 都不能升级为该回执。Google 以后仍须使用专用 reauth challenge；恢复码只恢复密码并撤销 session，不能产生近期重认证。

### v218 单次消费与复合 mutation

v218 新增 `node_endpoint_owner_reauthentication_consumptions`。它以 24 个物理列封存 exact v217 receipt、owner/action/request/target 与 mutation result；表为 `WITHOUT ROWID`、append-only、no-replace，并以 deferred FK 联结同事务稍后写入的 credential version/revocation/root。source guard 区分 initial 无 root、active rotate/revoke、active 或 terminal-revoked recovery；reverse guard 要求新 version/revocation 的 owner authorization basis 精确指回本次消费。

四个 direct-TLS-only `POST /api/me/node-endpoint-credentials/...` 路径共用一个 `BEGIN IMMEDIATE`：重验真实 DB bearer session、active user、当前 password factor、legacy owner/install anchor 与 expected endpoint head；派生并 readback v217；先写 v218 consumption；再执行 v216 issue/rotate/recover/revoke CAS、exact readback并提交。rotate 额外从不进入 JSON/Debug 的专用敏感头接收旧 endpoint secret并固定时序校验。请求与响应分别消费同一 TLS evidence 派生的 transport/response permit，Store commit 不提供绕过 permit 的明文 getter。`NodeEndpointOwnerAuthorizationBasis` 虽可序列化，credential mutation 仍只接受由 exact v217/v218 闭包派生的 sealed authorization。

API 默认不可达；除完整配置并显式启用 direct TLS listener 外，还必须独立设置 `NODE_ENDPOINT_OWNER_CREDENTIAL_API_ENABLED=true`，避免既有 verifier-only 配置自动扩大权限。它拒绝 query credential、`OWNER_TOKEN` 与 plain HTTP，且任何 proxy header 都不参与 transport、peer 或 owner 证明；密码 step-up 先按 direct listener 实际 peer IP、再按 bearer 摘要走两级 process-local 限流，容量压力下拒绝新桶而不驱逐已有桶，所有响应 `no-store`。Google、trusted-device 与可信代理 producer尚未实现。

### Windows NodeAgent bootstrap 与 no-downgrade

`NODE_ENDPOINT_HTTPS_ORIGIN` 是 NodeAgent 唯一安全端点真源：必须是无 userinfo、path、query 或 fragment 的 `https` origin，owner API 与未来同 authority 的 WSS 地址只能从它派生，不能再持久化一条可独立漂移的 WSS URL。配置值与已持久化 origin 不一致时失败关闭；非 Windows 构建在注册或 mutation 前拒绝，因为当前 secret custody 只实现 Windows DPAPI CurrentUser。

secure 管理端登录只接受账号+密码并重新取得短期 DB bearer；token-only、无账号的 token+password、`NODE_USER_TOKEN` 和旧 `node.json` 凭据都不能进入该分支。NodeAgent 先把 `endpoint_required=true` tombstone 原子写入独立 `%APPDATA%\elon-node-agent\node-endpoint-credential.v1.json`，再清除内存与 `node.json` 的 legacy secret/token，随后才调用 gated secure register。即使旧状态文件清理失败，进程内 legacy credential、credential epoch、watch 与插件 bootstrap 仍立即失效，且不会继续 register。

secure register 的成功响应严格只有 `agent_id + owner_user_id`；服务端生成的 legacy anchor secret 不返回也不广告 `ws://`。若 owner/install 已有 endpoint root，409 返回 exact credential binding，NodeAgent 只能进入 recovery。issue/recover 的 authorization 与 mutation request ID 在发网前持久化；exact replay 只恢复元数据、不重返明文，并触发新的 recovery request。新 secret 只在首回响应中出现，随即进入独立 DPAPI 文件；本地类型不实现 `Clone`、`Debug` 或序列化，响应缓冲在解析后清零。endpoint tombstone、pending request、current binding 或加密 secret 损坏时均失败关闭，不回退 legacy。

服务端 legacy 注册现在在一个 `BEGIN IMMEDIATE` 中重验 bearer、agent/owner/install endpoint root 和最终 currentness；endpoint root 命中返回 409，不再 silent renew。legacy WS 以 DB row 为先，只有无 DB row 时才允许 `ELON_AGENT_SECRETS`，并在 side effect 前和 Manager 安装前两次重验 root 与 DB secret。root mutation 在 Manager write fence 中先完成目标 owner preauthorization，再取消安全地摘除旧 Registry/Manager/pending；Codex Vault legacy hash proof也在同一类 root gate后才核 secret。ToolApproval、task/req、CLI 与三条插件 dispatch/ACK 继续只由 exact `AgentProcessSessionKey` 线性化；该 key 仍不是耐久 endpoint authority。

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

### Direct TLS verifier seam

direct TLS 通过独立、默认关闭的 listener 提供唯一当前可构造的 secure transport proof。只有 `NODE_ENDPOINT_DIRECT_TLS_ENABLED=true` 才启用，并要求 `NODE_ENDPOINT_DIRECT_TLS_LISTEN_ADDR`、`NODE_ENDPOINT_DIRECT_TLS_CERT_CHAIN_PATH`、`NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_PATH` 与 `NODE_ENDPOINT_DIRECT_TLS_VERIFIER_REVISION` 作为完整组出现；owner credential routes 还要求独立的 `NODE_ENDPOINT_OWNER_CREDENTIAL_API_ENABLED=true`。一旦显式启用，证书、私钥、verifier revision、监听地址或 bind 任一无效都会让启动失败，不能静默回退为“已验证”。该 listener 固定 TLS 1.3 与 HTTP/1.1，握手后从 rustls 的同一 `ServerConnection` 读取 negotiated protocol、cipher 和 ALPN，并把 boot-scoped server instance、leaf certificate digest、verifier policy digest、连接 evidence ID 与握手时间纳入 canonical evidence digest。它不读取也不信任请求头、URI scheme、SNI、代理声明或 plain listener 状态。

每条 TLS 连接只得到一个 30 秒内可取走一次的中性 proof slot；重复、过期或 poisoned take 均失败。secure Router 在 credential gate 下挂四个 owner credential POST；只有再显式启用 `NODE_ENDPOINT_OWNER_BOOTSTRAP_API_ENABLED=true` 才增加 secure login/register 两个 POST。owner 路径把 evidence、POST、精确路径和 canonical mutation digest 绑定为 owner transport + response permit；`/agent/ws` 仍取走 proof 后固定返回 `503 NODE_ENDPOINT_CREDENTIAL_BRIDGE_UNWIRED`，不会 WebSocket upgrade、调用 legacy handler或写入 v216 session Store。proof 绑定当前 boot 的 `server_instance_id`，未来 `NodeEndpointSessionOpenRequest` 仍必须与它精确一致且在 30 秒内认证。

因此该 seam 证明的是“此请求来自本进程直接终止的特定 TLS 握手”，不是 bearer、owner、节点、在线状态或计算能力。可信反向代理模式仍不存在；未来若采用代理，必须先有仓库管理的受信 hop 与不可伪造 TLS evidence（例如受控 UDS/mTLS 或经验证的 PROXYv2 SSL TLV），普通转发头永远不够。

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

1. NodeAgent 已有 Windows DPAPI issue/recover bootstrap；未来补 rotate/revoke 管理面时仍必须携旧 secret或显式 recovery确认，并保留相同 pending/replay/no-downgrade 语义，不能恢复 silent install renew。
2. `server/src/homecli_agent.rs::agent_ws_handler` 与 `server/src/homecli_agent/agent_session.rs`：用受信 transport verifier 和单一 Store 认证事务替代 env-first、hash 查询后再查 metadata 的两段判断。
3. `server/src/homecli_agent.rs::{AgentEntry,AgentManager}`：process-local exact key、替换与同步 fence 已铺；未来必须换成或联结耐久 `NodeEndpointSessionBinding`。现有 agent-id-only `close_agent_session` 仍是 legacy facade，不能用于安全撤销。
4. `server/src/node_registry.rs::{NodeEntry,NodeRegistry}`：process-local `register_exact`、`update_capabilities_exact`、`touch_exact` 与 `unregister_exact` 已铺；未来 online/candidate reader 仍须联结耐久 current binding，不能把本地 map 当认证事实。
5. `server/src/homecli_agent/compute_plugin_sharing.rs` 及其 `install_plan_preparation.rs`、`install_plan_planning_snapshot.rs` 子叶：dispatch 与 ACK 已携 process key，ACK-derived 同步 Store closure 已阻止本地 replacement 穿越；未来敏感 observation 仍必须在同一 Store 事务中重验耐久 session head。
6. `server/homecli-proto/src/lib.rs`、`compute_plugin_sharing.rs`、`compute_plugin_install_plan_preparation.rs` 与 `compute_plugin_install_plan_planning_snapshot.rs`：dispatch request 和 observed ACK 都冻结 exact endpoint binding，不能继续只依赖 cloud session UUID 或 authorization 自报。
7. `server/src/node_agent_registration.rs`、`node_agent_config.rs`、`node_agent_cloud_connection.rs`、`node_agent_session.rs` 与 `server/homecli-proto/src/lib.rs`：注册响应、持久凭据、WS bearer 和 Register 帧均携 credential ID/revision；process-local credential epoch 只防本机换证竞态，不能冒充云端 revision。
8. `node_agent_config.rs`、`node_agent_admin_open.rs` 与部署配置：公网凭据 mutation 必须使用本批 direct-TLS owner API，节点通道必须 WSS；loopback 或显式开发期不安全通道只能进入 legacy/non-compute 分支。
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

- migration：`server/src/node_compute_sharing_migration.rs` 的 `migration_v216`/`migration_v217`/`migration_v218` 与 `server/src/node_compute_sharing_migration/endpoint_authority*.rs`；
- domain：`server/src/node_compute_sharing/endpoint_authority.rs`；
- Store：`server/src/store/node_credentials/endpoint_authority.rs`，子叶包括 `credentials/mutations/*`、`owner_reauthentication/{currentness,rows,consumption_rows}*` 与 `owner_credential_mutation/{current_account,current_target,authorization,execute,replay,secret,transaction}.rs`；
- process-local fencing：`server/src/node_registry/session_key.rs`、`server/src/homecli_agent/session_fencing.rs`，以及 exact-key 接线后的 `agent_session.rs` 与三条插件 observation 子叶；
- legacy no-downgrade：`server/src/store/node_credentials/legacy_registration.rs`、`endpoint_authority/legacy_currentness.rs`、`server/src/homecli_agent/{legacy_session_authority,endpoint_credential_fencing,legacy_message_fencing}.rs`；
- direct TLS verifier/API：`server/src/node_endpoint_transport.rs`、`server/src/node_endpoint_transport/{config,direct_tls,evidence_slot,secure_router,owner_api}.rs`、`owner_api/{contracts,handlers,ingress,response,bootstrap}.rs`，以及 endpoint authority 的 `session/direct_tls.rs` 与 `owner_reauthentication/direct_tls.rs`；
- Windows NodeAgent：`server/src/node_agent_endpoint_credentials.rs` 与其 `admin/bootstrap/login/owner_api/persistence/secure_store/startup/types` 子叶；
- migration registry：`server/src/store_migrations.rs`。

Store-owned `mutate_node_endpoint_credential_as_owner` 是四个 owner 路径唯一生产 facade；底层 `record_node_endpoint_owner_reauthentication` 与 issue/rotate/recover/revoke fixed-time kernel 仅供该事务组合或受限内部恢复，网络层不得分事务串联。session 侧 `authenticate_node_endpoint_session`、close/currentness/restart/recovery 内核仍无 WS caller。

credential Store commit 不直接暴露明文；只有消费与实际 Store transport 成对的 `OwnerApiResponsePermit` 后才形成 response delivery。底层 `NodeEndpointCredentialMutationReceipt` 与 `VerifiedCurrentNodeEndpointSession` 没有网络构造器、`Deserialize` 或 `Clone`。未来 WS 桥接必须继续通过 Store-owned facade 注入 sealed transport/session 输入。

当前已有默认关闭、direct-TLS-only 的 owner credential HTTP producer和 Windows NodeAgent issue/recover bootstrap；legacy no-downgrade fence也已接入，但仍没有 secure WebSocket upgrade、v216 session Store caller、AgentManager/NodeRegistry 耐久 binding或协议 credential triple，也没有 legacy backfill。整体仍只能报告为 `implementation_unwired`：取得 endpoint credential 不等于在线，更不等于 compute authority。本增量尚未编译、测试或运行，也未执行内存/磁盘迁移或真实 TLS、网络、并发、崩溃恢复与节点验证。
