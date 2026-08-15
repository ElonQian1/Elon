---
title: 外部矿池 Adapter Provider-specific 凭据续签验收边界
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: historical_local_rust_sqlite_axum_verified_current_narrowing_source_review_only
---

# 外部矿池 Adapter Provider-specific 凭据续签验收边界

## 本批状态

V253 的 2026-08-13 historical 领域合同、migration、Store、管理员 Service/HTTP 已完成本地专项验证：migration/源码合同 `5 passed`，进程内 Axum HTTP `3 passed`，共 `8 passed`。验证覆盖重复 migration、append-only 对象、Store/DDL 列顺序、canonical projection、隔离边界、genesis/replay/successor、撤销、脱敏、`registering -> 紧邻 active` 窄桥、later active 续签及局部零效果检查。正式证据指纹为 `8c9c1fe9f6c0df7c5e56cc280c171ff06ea3dfd2498b27e4a1ba6a791f71c4c5`，receipt 为 `fa0cdd61ad97a5fccd8d839ac5d8f75b7db6a12f092d01762b2dcb9ca26e2184`。其中 active 两类结果现只作为 historical/superseded contract evidence；V274 已把 pre-V275 current semantics 收窄为 `registering-only`，不能把旧通过数解释为当前 active authority。本批 registering-only narrowing、HTTP fail-close expectation 与 projected transition seam未编译、未运行、未测试。

本轮发现并修正一处冻结测试期望偏差：receipt canonical projection 的真实合同是 `77` 项而非 `79` 项。challenge 签发/过期时间不在 72 列 receipt 表中重复存储，而是通过 receipt binding 与 durable challenge binding 的完整 JSON 等式校验，因此不应虚构两个额外投影列。该修正没有扩大 V253 的数据库或业务权限。

本地通过不等于生产可用。V253 仍未连接真实 credential resolver、KMS、gateway、verifier runtime 或外部矿池；以下未覆盖矩阵仍是后续验收要求。

## 已覆盖与后续正向矩阵

- fresh database、V252→V253 升级、重复 migration、两次重开和历史 V243/V249 保持逐字兼容；
- exact V249 Provider binding + current neutral V249 release + current V241/V242 key 签发 durable challenge，验证 RSA genesis、currentness、exact replay 与 successor；
- challenge `201`，fresh record/revoke `201`，exact replay `200`，currentness `200`；
- 同一 binding sequence/predecessor 连续，旧 head historical；过期/撤销 head 可作为历史 predecessor 恢复；
- pre-V275 只有 registering receipt 在 exact 原 revision/digest/status current，且 live Adapter ID 必须等于 `logical_adapter_id`；
- 旧 registering→logical-active 窄桥、active challenge 与 later logical-active successor 的通过结果仅为 historical/superseded evidence，不再列为当前正向验收；
- future V275 genesis 应由单独的 non-authorizing transition proof 消费 current registering V253 与 planned adjacent projected-active target；future ordinary projected-active challenge/current 还必须消费 durable V275 activation witness 与 exact historical activation root；这些路径本批未运行、未验收；
- future active live Adapter ID 必须等于 `route_adapter_projection_id`，而 release/credential lineage继续使用 `logical_adapter_id`；验收不得断言二者相等；
- V243 历史 receipt、V249 locator commitment 与 onboarding 私有 locator 重算一致；结构化 DTO 不单列 locator/commitment，而授权签名方收到的 domain-separated 完整签名消息会承诺 commitment，但永不包含 raw locator；
- V250 漏洞情报与 V252 六能力沙箱 currentness 保持正交，V253 不把它们的缺失或成功投影为 credential 结果；
- credential 正向效果准确，其余 Adapter/Provider/route/execution/usage/settlement 效果全部为 `none`。

## 后续失败关闭矩阵

- `401/403`、malformed/unknown JSON `422`、语义非法 `400`、缺失对象 `404`、签名/root/currentness/lineage 冲突 `409`；
- nonce/message/signature、challenge、report、actor scope、幂等材料漂移，challenge 过期/重复消费和响应丢失重试；
- sibling challenge 只允许一个 successor，重复 genesis、错误 sequence/predecessor、stale head 与非 head revoke；
- V241 verifier 或 V242 key 撤销、digest/key ID/implementation 漂移；
- V249 binding/release/installation/adoption/application/V243 lineage 丢失、terminal、摘要或 projection 漂移；
- onboarding 原始 locator 的 scheme/commitment 与 V249 companion 不一致；
- report 非 `passed`、运行超过 10 分钟、生成延迟超过 5 分钟、有效期超过 60 分钟、已过期或未来时间；
- pre-V275 任意 active Provider、registering revision/digest 漂移或 registering live Adapter 不等于 logical Adapter 均失败关闭；
- future registering→projected-active 跳号、缺失 V275 witness/historical activation root、active live Adapter 不等于 route projection、把 logical ID 与 projection ID 强等、owner/release/config/live credential subject 漂移均失败关闭；settlement 不得被当作 credential stable subject 或 activation compatibility 字段；
- future active receipt 遇到任意 revision/digest/status 变化时须经 witness/root 门控 successor 重取证据；`draining`、`quarantined`、`disabled` 均不能签发或保持 current；
- receipt/head 显式撤销、到期、successor、SQL update/delete/replace、canonical JSON 或物化列漂移；
- response 出现 locator/commitment、nonce/message/signature及 digest、PEM、actor、幂等、confirmation、evidence 原文、receipt JSON、bearer/token/secret 或本机路径。

## 零效果与仍未验收

当前 HTTP 测试已确认 route adapter/credential/capability、service actor、attempt start outbox、Offer 和 Job 七类表未被 V253 写入，并确认 receipt/revocation 的 Adapter/Provider/route/execution/usage/settlement 效果为 `none`。完整运行验收仍须比较写前写后 Provider current version、v213 seal、Capacity/Reservation/Attempt/Lease/ACK/event、usage/Receipt/settlement/付款表；当前局部检查不能替代完整零效果证明。

尚未验收 SQLite V252→V253 文件库升级、两次重开、并发 sibling challenge、崩溃恢复、真实 TCP HTTP、真实 secret custody、外部认证、生产数据库/部署/MCP/PC、Provider activation、projected-active challenge/current、Sidecar/worker/ACK、派发、计量与结算。旧 `8 passed` 维持 historical local evidence；本批 registering-only narrowing、HTTP fail-close expectation 与 projected transition seam严格为 `source_review_only / implementation_uncompiled / implementation_unrun / passed=0 / failed=0`。V253 总体仍是 `implementation_partially_verified`，不能把历史验证状态升级为 latest narrowing或生产可用。
