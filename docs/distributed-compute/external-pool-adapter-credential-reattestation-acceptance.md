---
title: 外部矿池 Adapter Provider-specific 凭据续签验收边界
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter Provider-specific 凭据续签验收边界

## 本批状态

V253 的领域合同、migration、Store、管理员 Service/HTTP 与源码测试已写入，但按架构铺设阶段约束未编译、未执行 migration、未运行测试或服务，也未连接真实 credential resolver、KMS、gateway、verifier runtime 或外部矿池。实际执行结果固定为 `passed=0`；以下矩阵是后续必须运行的源码合同，不是通过证据。

## 后续正向矩阵

- fresh database、V252→V253 升级、重复 migration、两次重开和历史 V243/V249 保持逐字兼容；
- exact V249 Provider binding + current neutral V249 release + current V241/V242 key 签发 durable challenge，验证 RSA genesis、currentness、exact replay 与 successor；
- challenge `201`，fresh record/revoke `201`，exact replay `200`，currentness `200`；
- 同一 binding sequence/predecessor 连续，旧 head historical；过期/撤销 head 可作为历史 predecessor 恢复；
- registering receipt 在 exact 原 revision current；仅跨稳定主体完全相同的紧邻 `revision + 1 active` 后仍 current；
- active 状态可签发 successor，active receipt 必须 exact 匹配签入 revision/digest；
- 稳定主体未漂移时，高于 source binding `+1` 的 later active 仍可新签 successor；该 active receipt 随即只认精确签入版本，而旧 registering receipt 的延续桥仍只限紧邻 `+1 active`；
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
- registering→active 跳号、高于 `revision + 1`、owner/Adapter/config/live credential subject 漂移；settlement 不得被当作 credential stable subject 或 activation compatibility 字段，但仅 settlement 变更亦会产生新 active revision/digest，因而旧 active receipt 必须 historical 并通过 successor 续签恢复；
- active receipt 遇到任意 revision/digest/status 变化；draining/quarantined/disabled 均不能签发或保持 current；
- receipt/head 显式撤销、到期、successor、SQL update/delete/replace、canonical JSON 或物化列漂移；
- response 出现 locator/commitment、nonce/message/signature及 digest、PEM、actor、幂等、confirmation、evidence 原文、receipt JSON、bearer/token/secret 或本机路径。

## 零效果与仍未验收

运行验收必须比较写前写后 Provider current version、v213 Adapter/credential/service actor/route/capability/seal/outbox、Capacity/Offer/Job/Attempt/Lease/ACK/event、usage/Receipt/settlement/付款表，证明 V253 无旁路效果。源码中的固定 `none` 字段不单独构成证明。

尚未验收 Rust 编译、SQLite fresh/upgrade/reopen/concurrency/crash、进程内或真实 TCP HTTP、真实 secret custody、外部认证、生产数据库/部署/MCP/PC、Provider activation、Sidecar/worker/ACK、派发、计量与结算。因此状态只能是 `implementation_uncompiled / implementation_unrun / passed=0`。
