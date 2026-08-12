---
title: 外部矿池 Adapter 沙箱符合性续签验收边界
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter 沙箱符合性续签验收边界

## 本批状态

V252 的领域合同、migration、Store、管理员 Service/HTTP 与源码测试已经写入，但按架构铺设阶段约束未编译、未执行 migration、未运行测试或服务，也未连接真实 sandbox verifier。实际执行结果固定为 `passed=0`，不能继承 V239、V243 或 V244 的历史通过数字。

本批允许的证据仅为源码静态检查、rustfmt、diff hygiene、文件规模和文档一致性。以下用例是后续运行验收必须执行的源码合同，不是当前已通过结果。

## 后续必须运行的正向矩阵

- fresh database、V251→V252 升级、migration 重放和两次重开；
- exact V249 neutral release（含持久化的 `installation_content_digest`）+ current V250 + active V237 key 的 durable challenge、RSA 签名 genesis、currentness；
- 同一 release 的 successor 续签、旧 head historical、sequence/predecessor 连续；
- 过期或撤销 head 作为历史 predecessor 恢复新 head；
- fresh record/revoke `201`、exact replay `200`、currentness `200`；
- 六项 capability 的规范 test plan 和恰一条有序 `passed` observation；
- challenge 只暴露签名所需材料，record/current/revoke 递归脱敏；
- Provider 保持 exact `registering`，v213 Adapter/credential/service actor/route/seal/outbox 及 Offer/Job/Attempt 保持零新增；V252 challenge/current 不接收 Prepared、不重开重哈希文件且不声称 installed instance current。

## 后续必须运行的失败关闭矩阵

- `401/403`，malformed/unknown JSON `422`，语义非法 `400`，缺失对象 `404`，root/signature/currentness/lineage 冲突 `409`；
- nonce、message、signature、challenge ID、actor scope 或幂等材料漂移；challenge 到期、重复消费与响应丢失重试；
- 并发 sibling challenge 只能一个形成 successor；重复 genesis、断裂 sequence、错误 predecessor 和非 head 撤销；
- V249 持久化 release/admission/package/source/manifest/inventory/content digest 漂移或终态；Provider-specific installation/live-FS 漂移由未来 activation 消费 V249 companion/sealed Prepared 时另行失败关闭，不属于 V252 current；
- V250 被 successor 取代、撤销、到期、scanner key 撤销或投影漂移；
- V237 key 撤销、key ID/digest/operator/product 漂移；
- test plan 缺失、额外、乱序、revision/fixture 漂移，observation 重复、失败或非 `passed`；
- 外网、临时目录外写入、子进程或其他策略违规非零，以及时间/CPU/内存超限；
- SQL update/delete/replace、canonical JSON 与全部物化列投影漂移；
- record/current/revoke 响应出现 nonce/message/signature 及 digest、PEM、test plan、observations、transcript、actor、幂等、confirmation、receipt JSON 或本机路径。

## 零效果验收

V252 的每个成功入口都必须证明以下对象不变：

- Provider 状态、policy revision 与 digest；
- `compute_route_adapters`、route Adapter version、credential、service actor、route authorization/capability/seal 与 Start outbox；
- CapacityPool、Supply、Offer、Price Snapshot、Job、Reservation、Attempt、Lease、ACK/event；
- usage、Verification、Execution Receipt、settlement、Provider 收益与付款。

源码中存在固定 `none` 字段不单独构成零效果证明；运行验收必须在同一数据库夹具中比较写前写后权威表计数与 exact Provider current version。

## 仍未验收

- Rust 编译、全量 Store migration、SQLite fresh/upgrade/reopen/concurrency/crash 行为；
- 进程内或真实 TCP HTTP、真实 RSA verifier、真实 sandbox runtime、VM/container 隔离与恶意制品；
- transcript、系统调用、网络、文件系统、CPU/内存观测的真实性；
- 生产数据库升级、备份恢复、MCP/PC 管理面、部署与告警；
- 可续签 credential v2、Provider activation、service actor、v213 compatibility、route/worker/ACK、派发、计量与结算。

因此交付状态只能写为 `implementation_uncompiled / implementation_unrun / passed=0`。后续即使 V252 专项通过，也只证明服务器能验证一份签名声明链，不证明 Adapter 已在可信沙箱真实执行。
