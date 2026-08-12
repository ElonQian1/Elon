---
title: 外部矿池 Adapter Registry Release 沙箱符合性续签权威
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
---

# 外部矿池 Adapter Registry Release 沙箱符合性续签权威

## 1. 目的与历史边界

V252 为一份 exact V249 Provider-neutral registry release 建立可重复续签、可显式撤销的沙箱符合性证明链。它解决 V239 只能绑定一次 V222 admission、最长有效 24 小时且不能刷新，因而不能长期供未来 route/activation 消费的问题。

V252 使用全新的 schema、表、API ABI 和 lineage。V239 收据、有效期、current view、V244 adoption 及 V249 companion 中的历史引用保持原样；V252 不更新、延长、替换或伪装成 V239 successor。V239 继续证明早期 adoption/installation/registry 形成时曾存在的历史证据，不能作为 V252 current authority。

本服务不会启动 Adapter、运行 VM/container、验证 sandbox runtime 或 transcript 的真实性，也不会证明系统调用、网络或文件系统观测来自真实隔离执行。平台只验证规范输入、服务器派生测试计划、精确上游根、当前 V237 RSA 验证者签名、时间边界、零策略违规、六项通过观察和追加式续签链。

## 2. Provider-neutral 精确绑定

每份 re-attestation 固定绑定：

- exact V249 registry release ID/digest/material digest，以及 admission、package/source、implementation/archive、manifest、entry inventory、entrypoint、installation content、credential-verifier intent 和六项 capability 根；
- 一份 current V250 vulnerability re-attestation head 的 ID/digest、sequence、情报 snapshot 与到期时间；
- 当前 active V237 sandbox verifier key record ID/digest/key ID、operator、product 和固定 RSA 算法；
- 固定 sandbox policy、runtime ID、runtime image digest、isolation profile、运行/报告窗口与资源观察；
- 从 V249 六项 capability 规范派生的有序 test plan，以及每项唯一 `passed` observation；
- durable challenge ID、随机 nonce、消息/签名摘要、规范 UTC 纳秒时间、sequence 和 exact predecessor。

V252 是 release 级事实，不保存 Provider、owner、V249 provider binding、route projection、credential、service actor、管理员幂等材料或 activation 状态。V252 current 只重审已经持久化的 Provider-neutral V249 identity（其中包括 `installation_content_digest`）；它不接收 Prepared capability，不重开或重哈希安装树，也不证明任何 installed instance 的实时文件 currentness。未来 activation 必须另外取得一个 exact V249 Provider-specific companion/sealed Prepared，并在同一事务中重开重哈希文件、验证它与 V252 指向同一 neutral release 和 installation content digest。

六项能力固定为 `authenticated_ack`、`authenticated_events`、`cancel_no_start`、`idempotent_commit`、`prepare`、`reconcile`。每项必须恰有一条顺序、revision、test-case ID 和 fixture digest 完全匹配的 `passed` observation；外网尝试、临时目录外写入、子进程尝试和总策略违规必须为零。资源与时间必须落在固定 policy 上限内。任何缺项、重复、乱序、失败、漂移、超限或管理员豁免都失败关闭。

## 3. Durable challenge、签名与续签

Challenge 以随机 32-byte nonce 和五分钟有效窗口追加保存，绑定全部 exact roots、draft、当前 head 与 predecessor。管理响应可以返回 nonce 和待签消息，供外部 verifier 完成 RSA-SHA256 签名；二者不是秘密，但 challenge 只能消费一次且不能替换。

record 只接受 challenge ID、预期消息摘要、签名、幂等键和显式确认。服务器从认证会话注入 actor，在同一 `BEGIN IMMEDIATE` 事务内重新读取 durable challenge、V249/V250/V237 current roots、唯一 head 和签名材料，再追加 receipt。receipt 的唯一 challenge ID 是单次消费事实；fresh 返回 `201`，同 actor、同幂等键、同不可变请求精确重放返回 `200`，challenge、签名、lineage 或 root 漂移返回 `409`。

同一 release 只有一个未被后继取代的 head。genesis 的 sequence 为 1 且没有 predecessor；每个后继必须精确引用当前 head 并递增一次。两个并发 challenge 可以观察同一 predecessor，但最多一个能形成 successor。过期或已撤销 head 仍可作为新 challenge 的历史 predecessor，使管理员恢复证明链；它自身不会重新变为 current。

## 4. Currentness 与撤销

currentness 使用服务器生成的单一规范 `checked_at`，重新审计 immutable receipt、durable challenge、标量投影、RSA 签名、唯一 head 和完整 lineage，并动态要求：

- V249 neutral release、admission、package/source 与持久化的全部 release identity 仍 exact/current；这不是 live-FS currentness；
- exact V250 head 仍为 `verified_current`，且 V252 的有效期不越过 V250 intelligence expiry；
- V237 verifier key 仍 active、未撤销且用途精确；
- V252 报告未到期、没有 successor、没有 revocation；
- 六项 test plan/observations、policy/resource/time 与全部 canonical digest exact。

满足时返回 `verified_current`。任一上游终态、V250 续签取代或撤销、verifier key 撤销、报告到期、后继、显式撤销或投影漂移都会使旧证据只能作为历史材料。数据库 view 和先前 GET 结果都不能铸造 consumer authority。

revoke 只允许精确当前 head，必须提交 receipt ID/digest、充分 reason、幂等键和显式确认。服务器追加唯一 revocation，不 update/delete/replace 原收据。fresh 返回 `201`，exact replay 返回 `200`；不存在返回 `404`，摘要、actor scope、幂等或 head 漂移返回 `409`。

## 5. 管理 API 与脱敏

仅平台 `admin|owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations/challenge`
- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations`
- `GET /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations/currentness`
- `POST /api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/sandbox-reattestations/:reattestation_receipt_id/revoke`

Malformed JSON、未知字段或错误类型返回 `422`；语义非法返回 `400`；缺失 release、challenge、receipt 或 current head 返回 `404`；签名、lineage、当前性、并发或 exact-root 冲突返回 `409`。challenge 为 `201`；fresh record/revoke 为 `201`，exact replay 为 `200`；currentness 仅在 current 时为 `200`。认证与角色分别返回 `401/403`。

challenge 仅暴露签名所需 nonce/message 与脱敏 binding。record/current/revoke 响应递归移除 nonce/message/signature 及其 digest、PEM、公钥、完整 test plan/observations/transcript、operator/product、认证 actor、幂等 scope/key、confirmation、receipt JSON 和本机路径。固定正向效果仅为 sandbox re-attestation current；credential、Provider、route、execution、usage、settlement 与付款效果全部为 `none`。

## 6. Provider activation 仍为 NO-GO

V252 只关闭 renewable sandbox freshness。它不创建 service actor、credential、route、seal、outbox 或 Provider 新版本。未来 activation 仍必须在一个原子事务内消费：sealed V249 Provider companion 与重开重哈希文件能力、current V250、current V252、fresh credential authority、owner-issued 精确 service actor、Provider-specific v213 compatibility binding、credential/route/capability/seal，并最后把 Provider 从 exact `registering` revision 推进为 `active`；任一步失败都必须零效果。

V243 只绑定 `registering` Provider 且最长有效 60 分钟，Provider 激活后无法形成可续签 active-Provider credential currentness。因此 V252 完成后仍不应直接激活；下一安全切片是新的可续签 Provider-specific credential re-attestation ABI，保留 V243 历史语义，再实现上述不可拆 activation。

生产可用还需真实 verifier/sandbox 运行、credential resolver/KMS/gateway、Sidecar IPC/transport、authenticated ACK/event、Runner、可信计量、市场交割与结算。V252 receipt 或六项 `passed` 不能表述为 Adapter 已真实运行、安全或可上线。
