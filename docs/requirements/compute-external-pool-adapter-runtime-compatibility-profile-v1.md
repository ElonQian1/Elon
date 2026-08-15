---
title: 外部矿池 Adapter 运行时兼容性 Profile V1 需求
status: accepted
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
feature_id: compute-external-pool-adapter-runtime-compatibility-profile-v1
---

# 外部矿池 Adapter 运行时兼容性 Profile V1 需求

## 1. 目标

在 V265 已认证 no-work seam 之上提供一个 server-owned、版本化、机器可读的运行时兼容性
Profile。Profile 必须复用并绑定现有六项 release capability、V255 Linux runtime launch
policy、V258 server-broker transport policy、V259 supervisor/session policy 与 V265 ELNW
边界，使第三方 Adapter 作者和独立验证者能够针对同一份互操作合同生成有界 challenge 与候选
报告，而不需要从分散源码或历史文档猜测 ABI。

本批只验证 Profile、challenge 和候选报告的规范形状、摘要、时限、观察项及 ELNW root
计算。候选报告不带独立验证者签名，不读取安装包、不启动进程、不连接网络，也不形成
conformance、readiness、Adapter、route、activation、usage 或 settlement authority。

## 2. 强制边界

1. Profile 只能引用现有 release/runtime/transport/session policy 真源；不得复制或另建第二套
   Provider、Offer、Adapter release、沙箱六能力或凭据验证合同。
2. Profile 固定 Linux x86_64 authenticated runtime，不得根据运行测试的 Windows host 静默
   生成另一份合同。V255 policy 必须通过明确的 Linux catalog 取得。
3. Profile 使用 RFC 8785 JCS 和 domain-separated SHA-256 形成稳定摘要。仓库内机器可读 JSON
   必须与代码生成结果逐字段一致，任一上游 policy digest 漂移都使测试失败。
4. challenge 必须绑定 exact adapter ID、release version、implementation digest、capability-set
   digest、runtime image digest、Profile digest、32-byte nonzero nonce 与最多 10 分钟有效期。
5. 候选报告必须绑定同一 challenge，包含固定顺序的 authenticated bootstrap、Config delivery、
   Credential delivery、Adapter request generation、Broker exact exchange、Adapter response
   validation、authenticated shutdown 与 bounded reap 八项观察；未知、缺失、重复或乱序观察
   均失败关闭。
6. 候选报告只能使用公开测试材料的摘要。request 为 1..16 KiB，response 为 1..64 KiB，
   probe 最多 15 秒；child network/write/process policy violation 必须全部为零。
7. ELNW probe root 必须由报告携带的 32-byte nonce、request/response 长度和 SHA-256 摘要按
   V265 domain 与 big-endian 编码重新计算，不接受验证者自报 root。
8. challenge/report 使用 canonical UTC nanoseconds；run 必须位于 challenge 时间窗内且不超过
   30 秒。报告验证不查询当前时间，因此只能证明自洽形状，不能证明提交时仍 current。
9. 所有 effect 固定为 `none`，candidate status 固定说明无签名、无执行与无 authority。不得
   解除 V254 18 项 deny，不改变 Provider `registering` 或任何 readiness 字段。

## 3. 非目标

- 不执行第三方 Adapter、不读取 V256 Secret、不创建 V257 capsule，也不调用 V262-V265
  Store seam。
- 不连接 loopback 或生产 upstream，不验证真实 DNS、TLS、SPKI、账号、credential 或 no-work
  业务语义。
- 不签发独立验证者收据，不新增数据库迁移、HTTP、MCP、PC、APK、Sui 或公开 SDK。
- 不实现 atomic Provider activation、完整 admission gate、任务派发、可信计量或跨主体结算。
- 不宣称仓库固定 fixture 或任意候选报告代表第三方生产 Adapter 已兼容。

## 4. 验收标准

1. 代码生成的 Profile 精确绑定六项 revision 1 release capability、三个现有 policy catalog
   及 ELSP/ELNW 固定边界，并能通过 canonical JSON 稳定计算 Profile digest。
2. checked-in JSON 与代码生成 Profile 完全一致；任一 protocol、limit、policy ID/revision/digest
   或 observation inventory 漂移均由测试阻断。
3. challenge 正向用例通过；错误 nonce、摘要、时间窗、adapter/release 标识与 capability set
   均失败关闭。
4. 候选报告正向用例通过；ELNW root、长度、摘要、观察项、时序、policy violation、effect 或
   challenge binding 任一篡改均失败关闭。
5. source contract 证明新模块不依赖 Store、SQLite、network、process launch、secret resolver、
   activation、market、usage、settlement、Sui、HTTP 或 MCP。
6. 生产 Rust check、定向单元/合同测试、源码与文档模块化门禁通过；证据和未验收项进入独立
   acceptance 文档。
