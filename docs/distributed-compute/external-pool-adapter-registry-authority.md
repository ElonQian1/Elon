---
title: 外部矿池 Adapter Provider-neutral Registry 与安装 Companion 权威
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter Provider-neutral Registry 与安装 Companion 权威

> 当前实现：V249 registry/companion 与 V251 兼容修复已通过本地定向 migration 和 HTTP 验收。V251 只把凭据验证器 JSON 从原始字符串相等改为四个签名字段逐项相等，并原子替换已应用 V249 的 trigger；不放宽摘要、能力、制品、installation 或 Provider 根约束。独立的 V250 漏洞情报 re-attestation 仍按其权威与验收页单独计量，不属于本页通过证据。

## 1. 目的与边界

V249 在 V247 sealed current installation authority 之上增加两类不可变事实：一份 Provider-neutral Adapter release registry receipt，以及一份把该 release 精确绑定到一个 V247 installation 的 Provider-specific companion receipt。它把“全局实现是什么”与“哪个 Provider 的哪次惰性安装采用了该实现”分开，消除 v213 v1 把全局 release 身份与 Provider service actor 直接耦合的问题。

V249 只登记已安装、仍惰性的字节身份。它不读取 credential，不启动 entrypoint 或 Sidecar，不连接外部矿池，不创建 service actor、credential、route、seal 或 outbox，不激活 Provider，也不生成 Offer、Job、Attempt、ACK、计量、结算或付款。Provider 继续保持 `registering`；v213 表保持零效果。

## 2. Provider-neutral release 与 companion

neutral release 只能从一份当前且精确的 V247 installation 派生，固定绑定：

- `adapter_id`、`release_version`、`route_kind=server_adapter` 与 `supported_provider_kinds=[external_pool]`；
- archive/implementation、manifest、文件 inventory、entrypoint 内容与完整 installation content 摘要；
- 六项规范 capability 及 capability set 摘要；
- V222 admission 中的 expected credential verifier 身份与摘要；
- 规范 receipt schema、JCS、SHA-256、登记时间与固定无副作用字段。

全局唯一键是 `adapter_id + release_version`。相同 release 的另一份当前 installation 只有在以上 Provider-neutral 材料逐项相等时才可复用同一 neutral receipt；任何实现、能力、verifier 或安装内容漂移均失败关闭。neutral receipt 不保存 Provider、owner、policy/config、adoption、installation、管理员或 service actor 身份。

每份 installation 另有唯一 companion。companion 绑定 neutral receipt ID/digest 与 V247 installation receipt ID/digest，同时保留该 installation 已经绑定的 Provider/adoption/package/source 精确根和认证管理员 actor；actor、confirmation 与幂等材料只保存在私有 canonical receipt，不进入公开 summary。不同 Provider 可共享同一 neutral release，但必须拥有各自 companion；一个 companion 的历史化不得改写 neutral receipt 或另一 Provider 的 companion。

## 3. sealed 文件树与原子提交

POST 不信任 installation ID/digest 本身。Service 先取得 Store 返回的安装审计目标，在 `spawn_blocking` 中重新打开并全量复算内容寻址安装树，得到 non-Clone/non-Serde `PreparedExternalPoolAdapterInstallation`。随后 Store 在单个 `BEGIN IMMEDIATE` 事务中：

1. 生成唯一规范 UTC 纳秒 `checked_at`；
2. fresh 请求消费同一份 Prepared 能力并重验 V247 current authority；
3. 对 existing neutral release 做 exact audit，或追加唯一 neutral receipt；
4. 对 exact idempotency replay 做历史 installation/binding material audit，或为 fresh 请求追加 installation companion；
5. exact readback 两份回执后提交。

fresh、exact replay 和“已有 neutral + 新 companion”都必须先重新审计安装树。fresh 与“已有 neutral + 新 companion”还必须重验当前 V247 根；exact replay 只返回已经存在的不可变历史结果，不会因其短时上游随后自然到期而伪造一次新登记。文件句柄与 pinned directories 必须活到数据库检查和提交结束；SQL 行、current view、HTTP GET 或先前检查结果不能重建 sealed authority。

## 4. 当前性与 route freshness 分层

创建时必须消费完整 V247 sealed current authority，因此创建瞬间仍要求 V244、V239、V243、V232、V227、Provider 与文件树全部 current/exact。创建后，registry currentness 证明的是“这份全局 release 和这个安装 companion 仍是被明确保留的精确登记事实”，不把短时验证报告的自然到期误写成 registry 撤销。

管理 GET 先重新审计 exact installation tree，再让 Store 在同一数据库检查内验证 neutral/companion 摘要与投影，并持续要求：V247 installation terminal 不存在、V244 adoption explicit terminal 不存在、neutral 所绑定的 release admission 与 package 仍 current、Provider 仍为同一精确 `registering` revision、文件树仍 exact。仅 V239 sandbox report 或 V243 credential verification report 随时间自然到期，不会让已经建立的 V249 companion 历史化；显式撤销、终态、root 漂移或文件漂移仍失败关闭。sealed currentness 只有在这些门卫全部成立时返回 `200 binding_current`；任一失败返回 `409`。SQL view 的 `historical_only` 只供数据库展示和审计，不能作为 HTTP consumer authority。

这个持续 currentness 绝不等于 route freshness。未来 route/activation 事务必须另外取得 fresh V243 credential verification，并消费可续签的 security re-attestation；V239 当前没有刷新路径，因此即使 V249 companion 仍 current，后续 fresh route security gate 也会被阻断。后续批次必须建立可续签 security re-attestation，不能修改到期时间、复用旧 GET 或以 registry receipt 绕过 freshness。

neutral receipt 是不可变的全局 release 历史，可在某个 companion 历史化后继续供其他精确、当前 companion 引用；它本身不等于某个 Provider 可路由。管理 currentness 是脱敏观察，不是 activation consumer authority。

## 5. 管理接口与脱敏

仅平台 `admin` 或 `owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-registry-bindings`
- `GET /api/admin/compute/external-pool-adapter-registry-bindings/:binding_id/currentness`

POST body 只接受 installation receipt ID/digest、幂等键和 `confirm_registry_binding=true`；actor 由认证会话注入。fresh 返回 `201`，exact replay 返回 `200`。GET 对不存在 binding 返回 `404`；上游、摘要或文件树漂移失败关闭。

公开响应只含 neutral 与 companion 的稳定身份、摘要、Provider-neutral release 元数据、Provider/installation 精确引用、登记时间、currentness 状态和固定效果。不得返回本机路径、entrypoint 原文、installed file 列表、candidate artifact locator/ref、credential locator/commitment、签名、公钥、receipt JSON、认证管理员 actor、幂等 scope/key 或 confirmation。

固定效果只允许表示 `provider_neutral_release_registered` 与 `installed_instance_companion_recorded`；credential、provider、route、execution、settlement 效果均为 `none`。

## 6. 后继硬门槛

V249 只修正 registry 身份域，不直接写入或兼容冒充 v213 v1 route Adapter。未来 activation 必须在同一原子事务消费一个仍 current 的 V249 companion sealed authority，并同时建立真实 Provider service actor、credential verification/currentness、Provider-specific route/credential/authorization、v213 compatibility binding 与 Provider activation；任一部分失败都不得让 Provider 进入可出售状态。

后续还需独立实现 credential resolver/KMS/gateway、Sidecar sandbox、transport、authenticated ACK/event、Runner、可信计量、市场交割和结算。V249 receipt、目录存在或管理 GET 均不能作为这些能力的证明。
