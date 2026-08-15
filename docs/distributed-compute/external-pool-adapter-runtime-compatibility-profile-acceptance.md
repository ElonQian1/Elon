---
title: 外部矿池 Adapter 运行时兼容性 Profile 验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: frozen_v1_static_profile_and_unsigned_candidate_validation
---

# 外部矿池 Adapter 运行时兼容性 Profile 验收边界

## 本批状态

V266 已实现 server-owned Linux x86_64 runtime compatibility Profile、机器可读 JSON、10 分钟
有界 challenge、八项固定观察和 unsigned candidate report 的纯验证器。Profile 直接绑定现有六项
release capability、V255 runtime、V258 transport、V259 session 与 V265 ELNW 常量；所有 effect
固定为 `none`。

本批没有运行第三方 Adapter、读取 package 或 Secret、启动 sandbox、连接 upstream、访问 Store、
签发独立 verifier receipt，或改变 Provider、route、activation、usage、market、settlement 和 Sui
状态。

## V267 版本边界

下文 `6 passed / 0 failed`、checked-in JSON 和 digest 继续对应冻结 supervisor/session V1
Profile。V267 current catalog 为 V2，但不会在 Profile revision 1 下静默改写该机器合同。
V267 尚无新的 V2 Profile、challenge/verifier 或 runner evidence，相关结果严格为
`source_review_only / passed=0`；本批把 V1 builder 改接 historical catalog 的源码也未重新
编译或执行 JSON parity test。V1 通过数不能累计到 V2。

## 验收矩阵

1. checked-in JSON 与当前代码生成 Profile 逐字段相等，上游 policy digest 漂移会失败；
2. RFC 8785/I-JSON canonical JSON 可回读为同一 envelope，domain-separated digest 稳定；
3. challenge 正向通过，并拒绝空 Adapter ID、非小写 SHA-256、zero nonce 和超过 10 分钟时限；
4. candidate report 正向通过，并拒绝 lineage、ELNW root、观察顺序、policy violation、effect、
   30 秒 run 时限、child network、零长度 request 和 report digest 篡改；
5. source contract 冻结 V265 ELNW exact 常量、server broker exact exchange 与三个上游 policy
   catalog 绑定；
6. source contract 禁止 Store、SQLite、网络、进程、Secret resolver、activation、usage、market、
   settlement、Sui、HTTP 和 MCP 依赖。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 -- test --manifest-path server\Cargo.toml --bin elon-server v266_

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- check --manifest-path server\Cargo.toml
```

Profile dump helper只用于有意提升 Profile revision 时生成候选 JSON，默认 ignored，不是产品入口。
全部 Rust 构建复用 D 盘受管缓存，不使用 Docker。

## 验证回执

| 证据 | 指纹或结果 |
|---|---|
| production Rust check | `e62a7170e86ce803a89763945052986a0d5e9cbd0d199fd2ad5147c85d6c0270` |
| focused V266 test log | `0577448ccd4368a750d87b023eefc703c6b61d8297ffdb7288fb32a1b158b37d` |
| focused result | `6 passed / 0 failed / 1 ignored / 1968 filtered out` |
| checked-in Profile digest | `a63d30b6f2f75c78c156ddb9ea609312f8b9b6726f403fedb960ed8a754fa047` |
| combined V266 fingerprint | `04212b4018d4abbb85d88b74b3080f8cc478a76882ed172aa983e6f7085cac00` |

组合指纹按 production check、focused test log 和 Profile digest 的小写 SHA-256 依次以 LF
连接后再次计算 SHA-256。机器级日志和共享 validation evidence 不属于产品状态或发布工件。

## 未验收与禁止声明

- 未运行任何第三方或生产 Adapter binary，也未证明 caller-asserted observation 来源真实；
- 未校验独立 verifier 身份、签名、撤销状态、时间 currentness 或抗重放持久状态；
- 未读取 V256 Secret、V257 capsule 或真实安装包，未连接生产 DNS、TLS、SPKI 或 upstream；
- 未动态复验 V262-V265 Store orchestration、installation roots 或 durable readiness；
- 未实现完整 admission gate、atomic Provider activation、任务派发、可信计量或跨主体结算；
- 未开放 HTTP/MCP/PC/APK，未发布服务器、安装包或链上交易。

因此只能声明冻结 V1 的 V266 机器合同与 unsigned candidate validation 曾通过静态/单元边界
验收；本批 historical-catalog 接线和 V2 Profile 都是 `source_review_only / passed=0`。不能
声明第三方 Adapter 已兼容、Provider 已 ready，或生产算力交易链路完成。
