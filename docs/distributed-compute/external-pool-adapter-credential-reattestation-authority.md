---
title: 外部矿池 Adapter Provider-specific 凭据续签权威
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: historical_local_rust_sqlite_axum_verified_current_narrowing_source_review_only
---

# 外部矿池 Adapter Provider-specific 凭据续签权威

## 1. 目的与 V243 历史边界

V253 为一份 exact V249 Provider-specific installed-instance companion 建立可续签、可显式撤销的凭据认证签名链。V253 最初还定义了从 logical Adapter `registering` 到 logical Adapter `active` 的 currentness 桥；V274 已废止该 active 解释。pre-V275 的 V253 current authority 现在严格是 `registering-only`，旧 logical-active 结果只保留为 historical/superseded contract evidence，不能继续充当 current authority。

V253 使用新的 schema、表、API ABI 与 lineage。V243 receipt、V244 adoption、V247 installation 和 V249 companion 的历史语义保持不变；V253 不更新、延长、替换或把 V243 行重新解释为可续签证据。V243 只作为 V249 companion 已保存的 genesis lineage，不能作为 V253 current authority。

平台仍不解析或读取 credential，不调用 vault、KMS、gateway 或外部矿池。当前 `vault-ref:` / `gateway-ref:` 只是 onboarding 中服务端持有的非 bearer 查找引用。V253 从同一历史 onboarding root 重新计算域隔离 locator commitment，并验证外部 V241/V242 verifier 对精确声明的 RSA 签名；这不证明平台自身取得过 secret 或执行过网络认证。

V250 漏洞情报与 V252 六能力沙箱证据仍是 Provider-neutral release 级正交权威；V253 不复制、汇总或替代二者，也不因 credential current 就推定安全扫描或沙箱 current。

## 2. Provider-specific 稳定主体

每份 V253 binding 固定包含：

- exact V249 provider binding ID/digest/material digest、neutral release ID/digest、installation/adoption/application 根、installation content digest 与预留 route projection ID；
- Provider live credential subject：provider ID/kind、owner、Adapter ID/release、config revision/digest；
- settlement account 只作为签发时 observed historical root 保存；它不是签名 credential stable subject，也不是未来 activation compatibility 字段。旧 logical-active receipt 的 revision/digest 只能作为历史证据；future projected-active receipt 仍须精确绑定其 observed Provider pair，任何新 Provider 版本都要求在 V275 witness/root 门控路径重取证据；
- challenge 签发时观察到的 Provider status、policy revision 与 digest；
- onboarding 中定位符的 `vault_ref|gateway_ref` 类型与域隔离 SHA-256 commitment，不含原始 locator；
- V249 历史保存的 V243 receipt ID/digest，以及当前 V241 implementation / V242 RSA key 的精确坐标；
- V243 同形的 verifier report：最长运行 10 分钟、生成延迟 5 分钟、有效期 60 分钟、credential resolution 与 Provider authentication 均为 `passed`、上游响应证据摘要；
- durable challenge、随机 nonce、签名消息、sequence 和 exact predecessor。

V253 以 `provider_binding_id` 为唯一续签链 scope；Provider policy revision 或 digest 不能另起一条平行链绕过旧 head。pre-V275 签发 challenge 与 ordinary currentness 只允许当前 Provider 为 exact 源 `registering`，并要求 live Adapter ID 等于 `logical_adapter_id`；`active`、`draining`、`quarantined`、`disabled` 与其他状态全部失败关闭。future projected-active challenge/current 必须由 durable V275 activation witness 与 historical activation root 门控，不能复用该 registering 分支。

## 3. V274 supersession：pre-V275 registering-only

currentness 仍把“签发时观察版本”与“稳定 credential subject”分开，但 V274 冻结了双身份边界：

- 当前 challenge、receipt current view 与 Store-private current authority 只接受 exact registering revision/digest/status；registering live Adapter ID 必须等于 `logical_adapter_id`；
- 旧测试覆盖的 registering→logical-active 窄桥、logical-active challenge 与 later logical-active renewal 只证明旧合同曾实现，现均为 historical/superseded evidence，不是当前正向路径；
- future V275 genesis 使用单独的 non-authorizing transition-proof helper，消费 same-transaction current registering V253 authority与 planned adjacent projected-active target；它不要求预先存在 V274 durable row，也不能自行激活 Provider；
- future ordinary projected-active challenge/current 必须消费 durable V275 activation witness 与 exact historical activation root。active live Adapter ID 必须等于 `route_adapter_projection_id`，credential/release lineage仍锚定 `logical_adapter_id`，永远不得断言 logical ID 与 projection ID 相等；
- 跳 revision、缺失 witness/root、logical Adapter 冒充 active projection、owner/release/config 或其它 live credential subject 漂移均失败关闭。future active policy revision变化须经 successor refresh 重取 exact evidence，不能恢复旧 logical-active 例外。

因此 pre-V275 不存在可达的 active V253 currentness。V274 只提供 dormant structural target/transition seam；projected-active producer 与 ordinary active renewal 要等 V275 durable witness/closure 后由专门分支实现，并继续在同一 connection、同一 `checked_at` 重验。

## 4. Durable challenge、续签与撤销

challenge 使用服务器生成的 32-byte 随机 nonce 和五分钟有效窗，并追加保存完整 exact binding、draft、当前 head 与 predecessor。它只能消费一次，不能 update/delete/replace。两个 sibling challenge 可以观察同一 head，但最多一个能形成下一 sequence。

record 只接受 challenge ID、预期消息摘要、RSA signature、幂等键和显式确认。Store 在一个 `BEGIN IMMEDIATE` 内重新读取 challenge、V249 历史 companion、当前 release / Provider、V241/V242 key、当前 chain head 和时间窗，再验签并追加 receipt；pre-V275 的当前 Provider 必须仍为 exact registering pair。genesis 为 sequence 1；每个 successor 必须引用当前 head并递增一次。过期或撤销 head 仍可作为历史 predecessor 恢复新 head，但自身不会重新 current。future projected-active record 必须走 V275 witness/root 门控的专门分支，不能放宽现有路径。

revoke 追加唯一 terminal receipt，不改写 credential secret、V243 或 V249。fresh record/revoke 返回 `201`，exact replay 返回 `200`；challenge、head、actor scope、幂等、签名、root 或稳定主体漂移返回 `409`。

current authority 是 Store-private、不可 Clone/Serde 的 same-connection capability，保存统一规范 `checked_at`。HTTP GET、SQLite view、历史 DTO 或先前读取结果都不能替代未来 activation 事务内的 fresh currentness。

## 5. 管理 API 与脱敏

仅平台 `admin|owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-registry-provider-bindings/:provider_binding_id/credential-reattestations/challenge`
- `POST /api/admin/compute/external-pool-adapter-registry-provider-bindings/:provider_binding_id/credential-reattestations`
- `GET /api/admin/compute/external-pool-adapter-registry-provider-bindings/:provider_binding_id/credential-reattestations/currentness`
- `POST /api/admin/compute/external-pool-adapter-registry-provider-bindings/:provider_binding_id/credential-reattestations/:reattestation_receipt_id/revoke`

Malformed/unknown JSON 为 `422`；语义非法为 `400`；缺失 binding/challenge/receipt/current head 为 `404`；签名、currentness、lineage 或 exact-root 冲突为 `409`。challenge 与 fresh record/revoke 为 `201`，exact replay 和 currentness 为 `200`。

challenge 只暴露完成签名所需的 nonce/message 和脱敏安全根；结构化 binding 不单列 locator commitment，但授权签名方必然可从 domain-separated `signature_message_base64` 解码出包含 commitment 的完整 JCS binding，以验证签名确实承诺该根；任何响应都不含原始 locator。record/current/revoke 递归移除 locator/scheme/commitment、nonce/message/signature 及三者 digest、PEM、actor、幂等 scope/key、confirmation、原始 evidence 内容、receipt JSON 与本机路径；脱敏摘要仍可保留 observed Provider digest 与 legacy V243 receipt ID/digest 等安全根。正向效果只允许 `signed_provider_credential_reattestation_verified_current`；Adapter、Provider、route、execution、usage、settlement 和付款效果均为 `none`。

## 6. Activation 继续 NO-GO

V253 不接收 Prepared、不重开或重哈希安装树，不创建 service actor、credential resolver、v213 Adapter/credential/route/capability/seal/outbox，也不推进 Provider。未来 V275 activation 必须在同一原子事务消费 sealed V249 companion/Prepared、current V250/V252、current registering V253 transition proof、V274 planned logical-to-projection target、精确 owner-issued service actor与 Provider-specific v213 compatibility binding，并原子闭合 exact `registering → projected active` Provider version及 durable activation witness；任一步失败必须零效果。提交后的 ordinary projected-active currentness只能由该 witness/root 门控，不能把旧 V253 logical-active 证据重新解释为 authority。

生产可用还需要真实 credential resolver/KMS/gateway、verifier runtime、外部矿池认证、Sidecar IPC/transport、ACK/event、Runner、可信计量与结算。V253 的签名 `passed` 声明不能描述为 secret 已由平台读取、外部矿池在线、Adapter 已启动或 Provider 已可售卖。

V253 原 `8 passed` 仅证明旧合同与 registering 核心的 historical local evidence；本批 registering-only narrowing、HTTP fail-close expectation 与 projected transition seam均为 `source_review_only / implementation_uncompiled / implementation_unrun / passed=0 / failed=0`，未执行编译、测试、migration、SQLite或运行时。
