---
title: 外部矿池 Adapter 运行时兼容性 Profile 权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: frozen_v1_static_profile_and_unsigned_candidate_validation
---

# 外部矿池 Adapter 运行时兼容性 Profile 权威

## 1. 唯一语义：发布可复现的互操作合同

V266 把 V255、V258、V259 与 V265 已存在的 Linux runtime、server-owned upstream
transport、authenticated supervisor/session 和 ELNW no-work 边界组合为一份 server-owned、
版本化、机器可读的兼容性 Profile。第三方 Adapter 作者和独立验证工具可以据此生成相同的
challenge 与候选报告，而不需要复制服务端源码中的常量。

机器真源是
[`external-pool-adapter-runtime-compatibility-profile-v1.json`](external-pool-adapter-runtime-compatibility-profile-v1.json)。
代码从 V255/V258 current catalog 与显式冻结的 supervisor/session V1 catalog 重建 Profile，
并使用 RFC 8785/I-JSON 规范化和 domain-separated SHA-256 计算摘要；checked-in JSON 与该
V1 合同逐字段不一致时测试失败。后续 current catalog 升级不能静默改写 Profile V1。

## 2. Profile 的 exact 绑定

Profile 固定 Linux x86_64，绑定六项 revision 1 release capability、V255 runtime launch
policy、V258 upstream transport policy、冻结的 V259 supervisor/session policy V1，以及
ELSP/ELNW 与 Broker exact exchange 的固定协议参数。三个 policy digest 来自对应版本化
catalog；V266 不另建第二套运行时、网络、凭据或沙箱政策。

ELNW request 为 1..16 KiB，response 为 1..64 KiB，单项 probe 最多 15 秒。root 使用 V265
相同 domain、32-byte nonce、big-endian 长度和 request/response SHA-256 重算。Profile 中所有
conformance、credential、Adapter、Provider、route、activation、execution、usage、market 与
settlement effect 均固定为 `none`。

## 3. Challenge 与候选报告

challenge 必须绑定当前 Profile ID/revision/digest、exact Adapter ID、release version、
implementation digest、capability-set digest、runtime image digest和 32-byte nonzero nonce；
有效期最多 10 分钟，并使用 canonical UTC nanoseconds。

候选报告必须绑定 exact challenge，run 位于 challenge 时间窗内且不超过 30 秒，并按固定顺序
包含八项观察：authenticated bootstrap、Config delivery、Credential delivery、Adapter request
generation、Broker exact exchange、Adapter response validation、authenticated shutdown 和
bounded reap。未知、缺失、重复、乱序、非 passed、零时长、超时或 policy violation 均失败关闭。

报告只接受公开 fixture 的摘要，不接受原始 Secret。child network、ephemeral 目录外写入和额外
进程尝试必须为零；ELNW nonce、长度、摘要和 root 由服务端验证器重新校验。

## 4. 权限边界

V266 的候选报告状态固定为
`unsigned_runtime_compatibility_candidate_no_authority`。它没有独立验证者签名，也不证明报告者
真正执行了第三方 binary。验证成功只证明 JSON 自洽且符合当前合同，不形成 conformance、
readiness、Adapter、Provider、route、activation、execution、usage、market、settlement 或 Sui
权限。

新模块不读取安装包、不启动进程、不连接网络、不解析 Secret、不访问 Store，也没有 HTTP、MCP、
PC 或 APK 入口。Provider 继续保持 `registering`，V254 的 18 项 temporary absolute deny 不变。

## 5. 下一硬门

下一阶段必须把 exact release package、runtime image、受控 upstream fixture 和 server-owned
sandbox runner 绑定到一次真实执行，并由独立 verifier key 对结果签名。只有该执行证据与当前
installation、credential、target、readiness 和 admission roots 在同一原子事务中再次验证后，
才可以讨论解除相应 deny 或进入 atomic activation；单独的 V266 Profile 或候选报告不能作为
激活依据。

动态证据和明确未验收项见
[`external-pool-adapter-runtime-compatibility-profile-acceptance.md`](external-pool-adapter-runtime-compatibility-profile-acceptance.md)。

## 6. V267 版本边界

V267 把 fresh V259 companion 使用的 current supervisor/session catalog 升到 V2，但 V266
Profile V1 继续绑定冻结 V1 catalog、原 JSON/digest 和历史 `6 passed / 0 failed`。这避免在
Profile revision 不变时静默改变机器合同；同时也意味着 V266 不能作为 V267 派生 launch
image、post-exec dumpable、Yama、ancillary 或 cleanup 的兼容性证据。

面向 current V2 的 Profile、challenge、candidate report 与 verifier/runner evidence 尚未形成，
当前为 `passed=0`。后续必须发布独立版本并重跑，不得复用 V1 digest 或把历史 unsigned report
改标签升级。
