---
title: 外部矿池 Adapter 沙箱符合性续签验收边界
status: current
reviewed_at: 2026-08-13
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: local_migration_and_in_process_http_verified
---

# 外部矿池 Adapter 沙箱符合性续签验收边界

## 本批状态

V252 已随 `elon-server` 测试目标成功编译，并通过 9 项专用本地验收：7 项 migration 合同测试和 2 项进程内 Axum HTTP 行为测试。正式验证指纹为 `3dc01f491da5a4be4f111ff5c2de29f556710eaed60fc61093da10e968492fa6`，状态提升为 `implementation_partially_verified`。

首次运行发现 V252 测试夹具复用了 V249 已创建 sandbox verifier 的幂等命名空间，却提交了另一把随机公钥；生产不可变历史校验正确拒绝了材料漂移。改用 V252 独立命名空间后，夹具又错误读取 V250 公共摘要中不存在的字段；最终改为消费正式脱敏摘要的 `intelligence_expires_at`。两次修复都只调整测试支持代码，没有放宽生产签名、幂等、脱敏或 currentness 规则。

本批实际运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Force -Domain v252-sandbox-reattestation -- test --manifest-path server/Cargo.toml --bin elon-server sandbox_reattestation --no-fail-fast
```

## 已覆盖的正向合同

- migration 注册、规范投影、追加式表/view、root/time guard 与 Store 插入列顺序；
- exact V249 neutral release（含持久化的 `installation_content_digest`）+ current V250 + active V237 key 的 durable challenge、RSA 签名 genesis、currentness；
- 同一 release 的 successor 续签、sequence/predecessor 连续、stale sibling 拒绝；
- fresh record/revoke `201`、exact replay `200`、currentness `200`；
- 六项 capability 的规范 observation 形状及错误 observation 拒绝；
- challenge 只暴露签名所需材料，record/current/revoke 递归脱敏；
- Provider 保持 exact `registering`，v213 Adapter/credential/service actor/route/seal/outbox 及 Offer/Job/Attempt 保持零新增；V252 challenge/current 不接收 Prepared、不重开重哈希文件且不声称 installed instance current。

## 后续扩大验收

- `401/403`，malformed/unknown JSON `422`，语义非法 `400`，缺失对象 `404`，root/signature/currentness/lineage 冲突 `409`；
- nonce、message、signature、challenge ID、actor scope 或幂等材料漂移；challenge 到期、重复消费与响应丢失重试；
- 真正并发 sibling challenge 只能一个形成 successor；重复 genesis、断裂 sequence、错误 predecessor 和非 head 撤销；
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

- 生产形态 V251→V252 原位升级、两次磁盘重开、真正并发与 crash 行为；
- 进程内或真实 TCP HTTP、真实 RSA verifier、真实 sandbox runtime、VM/container 隔离与恶意制品；
- transcript、系统调用、网络、文件系统、CPU/内存观测的真实性；
- 生产数据库升级、备份恢复、MCP/PC 管理面、部署与告警；
- 可续签 credential v2、Provider activation、service actor、v213 compatibility、route/worker/ACK、派发、计量与结算。

因此当前只能表述为“V252 migration 合同与核心管理 HTTP 行为已通过本地专用验收”。它只证明服务器能验证一份签名声明链，不证明 Adapter 已在可信沙箱真实执行。
