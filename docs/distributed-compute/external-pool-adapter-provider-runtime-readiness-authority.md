---
title: 外部矿池 Adapter Provider-specific post-cleanup runtime readiness 权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
verification_status: targeted_local_verified
---

# 外部矿池 Adapter Provider-specific post-cleanup runtime readiness 权威

## 1. 唯一语义：短时生产观察，不是 activation

V270 为 exact Provider binding 增加一条 durable、可撤销、极短时的 runtime readiness
verification。它组合 current V249/V250/V252/V253/V254/V255/V258/V259、exact current V268
signed runtime compatibility receipt，以及一次实际消费 V256 operator-mounted config/credential、连接
V258 target 并由 V265 child 接受 no-work response 的 Provider-specific observation。只有 authenticated
shutdown、pidfd reap、cgroup 与 scratch cleanup 全部成功后，Store 才可追加 readiness receipt。

receipt 只证明同一进程 custody、同一短时 evidence 窗口内，exact Provider-specific runtime 链完成过
一次无任务探测。它不保持 child、session、socket 或 Secret，不声明 upstream 有任务或容量，也不创建
route、service actor、Provider activation、market admission、usage 或 settlement authority。Provider 必须
保持 exact `registering`，`activation_ready=false`，V254 的 18 个 temporary absolute deny 逐字保留。

V269 的管理员 courier 已可形成 V268 signed receipt；无人值守 signer transport、私钥托管和自动续签
影响运维可用性，但不是 V270 正确消费一个 exact current V268 receipt 的前置。V270 不增加 signer、
worker、outbox、job、lease 或 retry。

## 2. 独立、默认关闭的启动 custody

V270 不复用 V269 fixture-only signing-handoff 的 enable authority。生产 Secret 与 upstream network 使用
独立三项环境合同：

- `ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_ENABLED`；
- `ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_CGROUP_PARENT_PATH`；
- `ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_BUNDLE_ROOT_PATH`。

enabled 只接受 exact ASCII `true|false`。未设置等于 `false`；disabled 时任一路径仍出现、enabled 时任一
路径缺失、空、非绝对或无法以 no-follow directory semantics 打开，都令 server 启动失败。enabled 仅
支持 Linux x86-64；delegated cgroup-v2 parent 必须具备 cpu、memory、pids controller，bundle root 必须
是 V256 固定 content-addressed layout 的本地安全目录。两份 directory authority 均由 server 私有托管，
请求不得传 path、FD、mount、cgroup、bundle root 或 fallback。

启动时还生成 locked、zeroize-on-drop 的 process HMAC key 与随机 custody epoch。key、epoch 原值、路径和
FD 不持久化、不序列化、不进日志或响应；receipt 只绑定 epoch digest。server 重启后无法恢复旧 key/epoch，
旧 receipt 即使墙钟尚未到期也必须成为 historical，不得用新进程重新解释。

固定路由始终注册。管理员 trigger 在完成认证与角色检查后，若 feature disabled 或 custody unavailable，
返回 `503 Service Unavailable`；不得退化为 caller path、宿主 root cgroup、普通文件读取或 V269 fixture
custody。currentness 与 revocation 仍可安全读取历史或追加终止记录，不把 unavailable 解释为 current。

## 3. 六份 late-bound Prepared 与唯一执行次序

V270 只允许 server 根据 path 与 caller-expected digest 查找私有 installation audit target。exact sealed tree
必须按用途晚绑定地独立重开、重哈希六次：V264 broker preflight、V264 broker postflight、V263 delivery
bundle、V263 delivery session、post-cleanup bundle reproof、post-cleanup session reproof。Prepared 不可 Clone、
Debug 或 Serde；不得预先囤积六份 handle，也不得让 SQLite transaction、connection 或 Prepared installation
handle 跨 DNS/TCP/TLS、child execution 或 application exchange await。

成功顺序固定为：

1. 初始 V270 preflight `BEGIN IMMEDIATE` 复核 exact Provider/binding/candidate/profile/target、caller expected
   roots、structural predecessor、actor-bound idempotency 与 trigger admission，随后提交；V250/V252/V253/
   V268 在进入网络前的 V265 preflight 复核，exact companion 在 delivery 与 final reproof 各消费一次，
   process custody epoch 只由最终 observation seal 绑定，不冒充初始事务输入；
2. 事务外按 V256 固定 layout 解析 locked bundle，按 V264 policy 完成 fresh DNS、public-unicast gate、TCP、
   TLS 1.3、WebPKI hostname/time/chain 与 exact leaf SPKI pin；
3. 启动 V267 current V2 launch image，完成 authenticated bootstrap、V256 Secret delivery 与 V265 bounded
   ELNW request/response；child 必须是验证 response 语义并返回 root-bound receipt 的同一个 session；
4. 先完成 authenticated shutdown，再由 pidfd bounded wait 完成 reap，并显式确认 cgroup leaf 与 scratch
   cleanup；任一失败都没有 readiness callback 或 durable write；
5. cleanup 后才重新取得 final bundle/session Prepared，在新的 `BEGIN IMMEDIATE` 和同一个 fresh
   `checked_at` 中重验全部 Provider roots、current dynamic evidence、bundle identity、custody epoch 与 exact
   V268 receipt，并原子追加唯一 readiness receipt；
6. transaction commit 后才形成公开 safe summary。响应或连接取消不能取消已经开始的 terminal cleanup。

V268 必须在 final insert 的同一 connection、同一 `checked_at` 通过 Store-private current authority 精确加入，
并与 Provider chain 的 registry release、installation content、source/launch image、Profile V2 与 V259 current
policy roots 逐项相等。公开 V268 currentness JSON、V269 signer payload 或先前缓存的 preflight 均不能作为
authority。

## 4. Durable receipt、私有 commitments 与 TTL

V270 只新增 append-only readiness receipt、append-only revocation 和 derived currentness view。没有 durable
`running`、`failed` 或 mutable head row；launch、network、protocol、cleanup 或 postflight 失败均零 readiness
写入。receipt 使用独立 domain-separated RFC 8785 JCS/SHA-256 schema、exact scalar projection、receipt
integrity、no-update/no-delete 与 no-replace guard。

V256 禁止把 config/credential content hash、bundle generation、delivery root 或 locator 落库。V270 因此只在
私有 receipt 中保存两个 keyed commitment：

- `runtime_bundle_identity_commitment`：process HMAC key 对本次 transient V256 bundle identity 的
  domain-separated commitment，可在同进程 fresh resolve 后重算比较；
- `post_cleanup_observation_commitment`：对 bundle commitment、V265 nonce/root、target observation、
  authenticated shutdown、reap、cgroup/scratch cleanup 与 custody epoch 的 domain-separated commitment。

两项 commitment、request/response digest、raw selected address、endpoint/SNI/SPKI、Secret hash、bundle
generation 与内部 receipt JSON 都不得进入公开 projection。数据库副本不能用 commitment 作为离线猜测
oracle；没有 process HMAC key也不能把伪造 row恢复成 Store-private current authority。

V270 不授予新的时间窗口。`expires_at` 必须是以下最早值：

- post-cleanup fresh `checked_at + current V255/V259 probe_timeout_ms`，且 timeout 硬上限为 15,000 ms；
- current V250 vulnerability intelligence expiry；
- current V252 sandbox report expiry；
- current V253 credential report expiry；
- exact V268 signed receipt expiry。

final insert 开始和提交前都必须满足 `now < expires_at`；否则零写。durable row 过期后只保留审计历史，
不得把原 V265 process-private observation 放大为 60 秒、5 分钟或任意续期 capability。

## 5. Lineage、currentness 与 revocation

readiness 按 `provider_binding_id` 全局单线：唯一 genesis、唯一 `(binding, sequence)`、每个 predecessor 最多
一个 successor。fresh successor 必须 exact 引用 structural latest receipt ID/digest；latest 即使已过期或已
撤销，也可作为新 probe 的 predecessor。相同 authenticated actor、idempotency key 与请求 material 的
exact replay 返回已有 durable row，不再次 physical run；material 漂移固定冲突。

并发或进程在 observation commit 前崩溃可能重复一次只读 no-work physical probe；最终 UNIQUE/CAS 只允许
一条 durable successor，竞争者只能恢复首个 exact row。不得宣称 physical exactly-once。新 idempotency key
和 latest predecessor 才能申请新的 readiness 窗口。

Store-private `CurrentExternalPoolAdapterProviderRuntimeReadinessAuthority` 必须 non-Clone、non-Debug、
non-Serde，并在同一 connection/checked_at 重验 structural head、revocation、expiry、process epoch、fresh
bundle HMAC commitment、V249/V250/V252/V253/V254/V255/V258/V259 与 exact V268 authority。公开 GET 是
diagnostic safe summary，不能缓存后带进 activation transaction。

fresh revocation 只要求 historical exact receipt、structural latest、未撤销、authenticated owner 或 platform
admin、actor-bound idempotency 与 confirmation；不要求 runtime custody、filesystem、upstream、policy 或动态
evidence仍 current。revocation 追加唯一终态，不修改 receipt，不触发 child、network、Provider 或 market。

## 6. Exact 五条 HTTP 路由

共同完整路径为：

`/api/{admin|me}/compute/external-pool-provider-bindings/:provider_binding_id/activation-candidates/:candidate_id/runtime-launch-profiles/:profile_id/upstream-transport-targets/:target_id/supervisor-session-policy-companions/:companion_id/provider-runtime-readiness-receipts`

exact surface 只有五条：

- admin `POST <base>`：唯一 production trigger，只允许 platform `admin|owner`；
- admin `GET <base>/:readiness_receipt_id/currentness`；
- admin `POST <base>/:readiness_receipt_id/revocation`；
- owner `GET <base>/:readiness_receipt_id/currentness`；
- owner `POST <base>/:readiness_receipt_id/revocation`。

owner GET/revocation 必须先证明 authenticated user 是 exact binding owner；owner 没有 trigger 权限。create body
使用 `deny_unknown_fields`，只接受 expected binding/installation/candidate/profile/target/companion digest、exact
V268 verification receipt ID/digest、optional predecessor pair、idempotency key 与
`confirm_provider_runtime_readiness=true`。Store 自选 current V250/V252/V253；caller 不得提交 actor、scope、
checked_at、expiry、policy、nonce、observation、readiness、endpoint、Secret、path、FD、cgroup 或 result。

revocation body 只含 expected receipt digest、reason、idempotency key 与 confirmation。fresh create/revoke 返回
`201`，exact replay 返回 `200`；认证、角色、shape、语义、缺失、root/currentness/idempotency、unavailable
与内部错误分别沿既有 `401/403/422/400/404/409/503/500` 边界。未授权错误不得泄露对象存在性。

## 7. Readiness、effects 与 activation 后置

durable receipt 的 `observed_readiness` 只允许六项 true：

- `process_spawn_ready`；
- `ipc_session_ready`；
- `secret_delivery_ready`；
- `broker_connect_ready`；
- `upstream_probe_observed`；
- `runtime_launch_ready`。

`activation_ready` 始终 false。derived `current_readiness` 仅在 receipt 为 exact current head、未撤销、未过期、
process custody 与所有 roots current 时复制前六项；任何 historical reason 都令七项 current readiness 全部
false。receipt 可使用纯描述性 `post_cleanup_runtime_readiness_recorded_no_activation_authority`，但 credential、
adapter、provider、route、activation、execution、usage、market、settlement 九项业务 effect 全为 `none`。

V270 不构造 v213 Adapter/version、route credential、service actor authorization、route authorization/
capability/seal 或 Start outbox，不推进 Provider `registering -> active`，也不创建 Pool、Offer、Job、Reservation、
Attempt、usage、settlement 或 Sui authority。atomic Provider activation 必须在后续独立批次同事务消费 V270
Store-private current authority并原子创建完整 route/runtime closure。CapacityPool/Offer market admission 还要
独立 capacity/price/admission roots 与 18 项 direct-SQL replacement guard，不能由 readiness 顺带授予。

## 8. 当前实现与验证现实

V270 已随完整 Windows `elon-server` 产品目标与 WSL2 `elon-server` test target 编译；Windows 受管
`provider_runtime_readiness` 17/17 与 process custody 3/3 通过。前者包含 15 项静态合同和 2 项真实
SQLite fresh/repeat/reopen/integrity 用例，后者覆盖提交晋升、exact material、身份漂移、进程 epoch 隔离和
15 秒上限。指纹见 acceptance。当前严格为
`implementation_partially_verified / targeted_local_verified`；HTTP/Linux production fixture、真实 child、
Secret/upstream、并发与故障矩阵仍未运行。

当前证据不能证明 startup controller、locked-memory/zeroize、六份 late-bound production audit、生产
DNS/TLS、Secret delivery、post-cleanup fault ordering、SQLite 并发 guard 或 TTL race 已动态验收。验收边界
见 [`external-pool-adapter-provider-runtime-readiness-acceptance.md`](external-pool-adapter-provider-runtime-readiness-acceptance.md)。
