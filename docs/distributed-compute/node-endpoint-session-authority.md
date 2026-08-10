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

v216 migration 本身不从 `node_credentials` 或 `ELON_AGENT_SECRETS` 回填，也不在 legacy 表上安装反向 trigger。后续增量已经显式接入 secure owner/bootstrap API、Windows NodeAgent issue/recover client，以及 legacy 注册、WS、Manager/Registry 和 waiter 的 no-downgrade 栅栏。当前增量再增加独立 v13 compute-inert endpoint-session DTO 与默认关闭的 WSS seam；它不复用 legacy Register、AgentEntry、NodeRegistry 或任务协议。direct-TLS-only listener 把同一 rustls TLS 1.3 连接的一次性中性 evidence消费为精确 audience proof；plain listener、legacy `/agent/ws`、URL/Host、`Forwarded`/`X-Forwarded-Proto` 和静态配置标记均拿不到 proof。

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

因此端点凭据、认证 receipt、会话 current head 与在线 socket必须分层。当前 WSS seam只把安全传输、当前凭据和 exact session generation 合成为 compute-inert endpoint authentication；敏感 ACK 仍须在同一 Store 事务重验该 head，才能在未来形成 compute endpoint authority。

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

认证 receipt 由 Store 从字段私有、不可反序列化且无公共构造器的输入生成。issuer 在一个事务中完成：

1. 验证受信 WSS/TLS transport proof；不得把 URL scheme 或普通代理头当证明；
2. 重验 credential current revision/digest、未撤销状态及 owner/install exact binding；
3. 精确核对 Register 声明的 owner/install/credential triple 与会话 profile：v13 只能是 `compute_inert + []`，v14 只能是 `planning_snapshot_bootstrap_only + [node_endpoint_planning_snapshot_bootstrap_v1]`；
4. 绑定当前 server instance，生成 session ID；
5. 以 previous generation/receipt 为前提追加 authentication receipt；
6. 用 generation `+1` CAS 把 session head 推进为新的 active head；
7. exact readback 后提交。

sealed domain 的 `NodeEndpointSessionBinding` 精确携带 agent/credential、credential revision/digest、session ID/generation、receipt ID/digest 与 server instance；Store permit 另封存不可混用的 v13/v14 profile。它只安装进独立 endpoint socket supervisor，替换、关闭、currentness 检查和断开清理只能对 exact binding 做 CAS。v14 supervisor只能运行固定 Planning bootstrap driver；两种 profile都不会进入 legacy `AgentEntry`、`NodeEntry` 或 capability map。

当前新增的 `AgentProcessSessionKey` 是这一桥接前的窄前置层：`AgentEntry` 与 `NodeEntry` 只保存一份 key，安装连接时固定 `AgentManager.agents -> NodeRegistry.nodes` 锁序；旧 entry 先收到同步 shutdown，再在锁外清理 pending/ACK；旧 reader 的 touch/update/unregister 均为 no-op。`with_current_process_session` 只允许在 manager read guard 内运行无 `await` 的同步操作，三条插件观察链用它保护 ACK-derived Store 写与下一条 durable intent 的准备。该 key 没有 credential revision、receipt、server instance 或跨进程恢复能力，不能进入任何 compute authority digest。

authentication digest 覆盖固定 `bearer_sha256` authentication method、agent version、capability digest 及 sealed WSS verifier evidence；capability 原文只作为物理投影保存，不另进 canonical envelope。当前 sealed contract 把一次认证会话冻结为 15 分钟绝对有效期；后续若需要继续连接，必须形成下一代 receipt/head，不能延长或改写旧 receipt。

session head 的既有 active 只允许精确转为 `closed`、`stale`、`credential_rotated` 或 `credential_revoked`，或者被 generation `+1` 的新 active receipt 取代。服务重启后必须通过 server-instance binding 或启动收口让旧 active head 不再具有 currentness；receipt 只证明认证发生时的绑定，耐久 head 只证明事务检查时的 currentness，两者都不能冒充 live socket。

### Direct TLS verifier seam

direct TLS 通过独立、默认关闭的 listener 提供唯一当前可构造的 secure transport proof。只有 `NODE_ENDPOINT_DIRECT_TLS_ENABLED=true` 才启用，并要求 `NODE_ENDPOINT_DIRECT_TLS_LISTEN_ADDR`、`NODE_ENDPOINT_DIRECT_TLS_CERT_CHAIN_PATH`、`NODE_ENDPOINT_DIRECT_TLS_PRIVATE_KEY_PATH` 与 `NODE_ENDPOINT_DIRECT_TLS_VERIFIER_REVISION` 作为完整组出现；owner credential routes 还要求独立的 `NODE_ENDPOINT_OWNER_CREDENTIAL_API_ENABLED=true`。一旦显式启用，证书、私钥、verifier revision、监听地址或 bind 任一无效都会让启动失败，不能静默回退为“已验证”。该 listener 固定 TLS 1.3 与 HTTP/1.1，握手后从 rustls 的同一 `ServerConnection` 读取 negotiated protocol、cipher 和 ALPN，并把 boot-scoped server instance、leaf certificate digest、verifier policy digest、连接 evidence ID 与握手时间纳入 canonical evidence digest。它不读取也不信任请求头、URI scheme、SNI、代理声明或 plain listener 状态。

每条 TLS 连接只得到一个 30 秒内可取走一次的中性 proof slot；重复、过期或 poisoned take 均失败。secure Router 在 credential gate 下挂四个 owner credential POST；只有再显式启用 `NODE_ENDPOINT_SESSION_API_ENABLED=true` 才允许 `/agent/ws` upgrade，而 bootstrap gate 还要求 credential 与 session 两个 gate同时开启，避免签发后没有安全登录通道。owner 路径把 evidence、POST、精确路径和 canonical mutation digest绑定为 owner transport + response permit；WSS 路径则把同一 evidence一次性绑定为 node-endpoint audience，10 秒内读取独立 v13/V1 或 v14/V2 Register并让 Store消费 proof。proof绑定当前 boot的 `server_instance_id`，OpenRequest必须与它精确一致且在30秒内认证；Hyper的30秒限制只覆盖握手与upgrade前阶段，不截断已升级socket。

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

## 7. v14 Planning bootstrap 截止线

v13/V1 保持 auth-only、空 capability 与 Ping/Pong/Close 语义，不原地扩义。v14/V2 增加的仍不是通用计算控制面：

1. v14 固定 `session_mode=planning_snapshot_bootstrap_only`、`compute_authority=false` 与唯一 capability `node_endpoint_planning_snapshot_bootstrap_v1`；V2 Register/Accepted 使用私有 wire 字段和全量校验，不能转换成 V1。
2. Accepted 后只允许六条独立消息：sharing request/observed、preparation request/observed、Planning request/observed。每条封存 exact session binding、bootstrap ID、1–6 sequence、delivery ID、前序消息摘要与自身 JCS/SHA-256 摘要；第一条以前述 authentication digest 为前驱。
3. 每个 exact session 最多一个 chain、一个 in-flight request。sharing disabled/rejected可在第二条结束，preparation rejected可在第四条结束，Planning始终在第六条结束；重复只接受同位置同摘要，冲突永久失败关闭。
4. v219 append-only provenance把三段来源与 exact v216 authentication receipt/head/current credential绑定。每段 observation 与下一 intent必须在同一 `BEGIN IMMEDIATE` 中完成 transaction-local currentness重验、写入、exact readback与提交；发送前 supervisor 检查不能代替该门卫。
5. Windows NodeAgent 在 Accepted V2 后建立 non-Clone typed session witness，并只复用本机纯 Bootstrap 状态机。credential replacement重绑 account并撤销旧 controller/witness；同 credential普通 socket replacement只换 session provenance。每阶段执行前后与发送 observation 前都重验 endpoint epoch/currentness。
6. Planning observation可 `accepted=true`，但当前必须保持 `snapshot_ready=false`、`snapshot=None`、`phase=blocked` 及全部副作用标志为 false。生产 handle-bound SQLite VFS、可信时间、rollback、policy/revocation、inventory/profile、catalog/keyring、installed/work-admission同快照 projector齐备前，不得生成 signed Plan或调用既有 generation/signer路径。
7. endpoint supervisor不创建 `AgentEntry`、不注册 `NodeRegistry`、不提供通用 `cmd_tx`。非法/超时消息、replacement、credential mutation、断开、重启或15分钟到期只终结 exact socket/chain；Ping不续期，节点 `connected=false`。

严格后续顺序仍是：生产 VFS 与 honest Planning Snapshot→Control-signed reauthorization及本机v8 work-admission/Host enforcement→Sidecar与active health→Ready V2服务端验真→route/ArtifactAccess→outbox/ACK/Lease。NodeAgent rotate/revoke管理面及任何Google reauth另行闭合，不得恢复silent install renew。

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
- endpoint WSS：`server/homecli-proto/src/node_endpoint_session*.rs`、`node_endpoint_planning_bootstrap/*`、`server/src/node_endpoint_transport/endpoint_session/*`、v216 session Store与v219 endpoint Planning provenance、独立endpoint supervisor；
- Windows NodeAgent：`server/src/node_agent_endpoint_credentials.rs` 与其 `admin/bootstrap/login/owner_api/persistence/secure_store/startup/types/session` 子叶，以及独立 `node_agent_endpoint_session.rs`；
- migration registry：`server/src/store_migrations.rs`。

Store-owned `mutate_node_endpoint_credential_as_owner` 是四个owner路径唯一生产facade；底层`record_node_endpoint_owner_reauthentication`与issue/rotate/recover/revoke fixed-time kernel仅供该事务组合或受限内部恢复，网络层不得分事务串联。session侧只由direct-TLS handler调用窄facade；raw receipt、currentness或close kernel不成为网络能力。

credential Store commit 不直接暴露明文；只有消费与实际 Store transport 成对的 `OwnerApiResponsePermit` 后才形成 response delivery。底层 `NodeEndpointCredentialMutationReceipt` 与 `VerifiedCurrentNodeEndpointSession` 没有网络构造器、`Deserialize` 或 `Clone`。未来 WS 桥接必须继续通过 Store-owned facade 注入 sealed transport/session 输入。

当前已有默认关闭、direct-TLS-only 的 owner credential HTTP producer、Windows NodeAgent issue/recover bootstrap及源码级 v13 auth-only WSS；本批再加入 v14 三阶段 Planning bootstrap与v219事务 provenance，但仍没有 honest snapshot producer、AgentEntry/NodeRegistry compute binding或任何通用敏感 ACK。整体仍只能报告 `implementation_unwired`：取得 endpoint credential、认证 socket或 blocked Planning observation都不等于compute online、Ready或派发权威。本增量尚未编译、测试或运行，也未执行内存/磁盘迁移或真实TLS、网络、并发、崩溃恢复与节点验证。
