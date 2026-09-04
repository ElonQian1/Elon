---
title: "ESK Sui 发布只读观察器 V1"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, protocol]
---

# ESK Sui 发布只读观察器 V1

## 目标与边界

交付一个不依赖钱包的测试网命令行查询切片：对人工提供的公开 package ID、
发布交易 digest、完整 genesis chain identifier，从两个明确的公开 GraphQL
端点查询链、包创建交易和成功交易的 checkpoint，输出版本化、可审阅的观察结果。
这是 [首批用户路线图](esk-first-user-delivery-roadmap-v1.md) 的链证据基础，
不是完整发行认证、余额同步或上线许可。

沿用 [创世基础 V1](esk-sui-genesis-foundation-v1.md)。不修改 Move、旧清单校验器、
Paper 资产合同、APP 或生产服务。不读取密钥、不签名、不广播、不写余额。
拒绝主网模式。两个不同端点仅代表两个读取来源，不证明运营主体相互独立，
也不替代委员会签名、源码、供应、分配和权限验证。

## 验收

1. 输入严格校验网络、公开 HTTPS 端点、完整 32 字节 Base58 chain ID/digest 与
   package ID；禁止 URL 凭据、查询串、重定向、任意 GraphQL 查询和超大响应。
2. 两端都观测到预期链、预期 package 的创建交易、成功 effects 及相同交易
   checkpoint，才返回 `observed`；缺失、索引延迟、失败、超时或不一致均为
   `unverified`，命令失败退出，不填入猜测证据。记录有界错误代码而非服务端原文。
3. 输出总是 `publication_certified=false`、`balance_eligible=false`、
   `manifest_transition_allowed=false`，不自动推进 `testnet_published`。
4. 测试覆盖成功、错误链、包与交易不匹配、失败交易、缺少 checkpoint、
   两端矛盾、网络错误、超时、响应限制和敏感输入拒绝。旧创世和资产测试仍通过。
5. 运维文档提供无密钥调用方法、输出解释、索引延迟重试方式及后续完整门禁清单。

## 编辑计划与交付边界

新增 `scripts/esk-sui-publication-observer/` 下验证、GraphQL 传输和观察逻辑模块，
各文件不超过 350 行；新增 CLI、离线测试与使用手册。只读 API 调研可并行，
核心实现由本切片认领人负责。不扩展 599 行的旧创世验证入口。

源码/测试证据由 Feature Registry 绑定。代码推送即本工具的代码交付，
实际 ESK 查询须取得真实发布参数后另验；公共网络 smoke 不算 ESK 发布验收。
剩余完整认证需 verify-source、Currency Registry 注册/fixed supply、逐桶对象
与数量、团队 Move 归属、能力交接、多端一致性和清单历史绑定。
