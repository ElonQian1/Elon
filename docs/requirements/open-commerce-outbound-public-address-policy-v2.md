---
title: 开放商业出站公网地址策略 V2
status: accepted
reviewed_at: 2026-08-15
owners: backend, security
priority: p0
---

# 开放商业出站公网地址策略 V2

## 问题

开放商业 Webhook、开发者域名验证和商户运行时已经在连接前检查 DNS 地址，
但当前公网地址分类与算力代理 TLS broker 各维护一份实现，规则已经发生漂移。
开放商业解析还缺少有界超时和答案数量限制，并只把单个地址固定给 HTTP 客户端，
因此既缺少可复核的统一安全策略，也无法在多个已验证公网地址之间受限故障切换。

## 目标

1. 开放商业与算力代理 TLS broker 共用一份公网单播地址分类实现，拒绝私网、
   回环、链路本地、共享、文档、基准测试、组播、保留、未获更具体公网例外的
   IETF 协议分配范围及已知转换特殊网段。
2. DNS 解析必须有超时和答案数量上限；空结果、超限、端口漂移或任一非公网答案
   都使整次调用失败关闭。
3. 验证后的地址必须去重并确定性排序，优先 IPv4；HTTP 客户端只使用本次已验证的
   全部地址，禁用环境代理和重定向，同时继续用原始域名完成 TLS SNI 与证书校验。
4. Webhook challenge、真实 Webhook 投递、开发者域名 challenge 和商户运行时调用
   继续通过同一开放商业入口，不得出现绕过路径。
5. 用编入服务器测试目标的离线回归覆盖 IPv4、IPv6、IPv4-mapped IPv6、混合答案、
   重复答案、端口漂移、空答案和答案数量上限。

## 验收标准

- 共享策略拒绝 ADR 已列出的 IPv4、IPv6 和 IPv4-mapped IPv6 特殊地址，并接受代表性公网地址。
- 混合公网与私网答案整体失败，不会只选择其中一个公网地址继续连接。
- 多个公网答案去重、稳定排序并全部固定给 reqwest，IPv4 排在 IPv6 前。
- DNS 查询在三秒内超时，最多接受 32 个答案，第 33 个答案触发失败关闭。
- reqwest 客户端禁用环境代理和重定向，并保留原始 HTTPS URL 作为请求及 TLS 主机身份。
- 开放商业四类调用方继续引用统一入口；算力代理现有 broker 测试继续通过。
- Rust 定向测试、源码体积、文档模块化和 Git diff 门禁通过。

## 非目标

- 不执行真实公网 DNS、TLS、Webhook 或商户生产环境验收。
- 不实现 DNSSEC、DoH、网络级 egress 防火墙或独立出站代理。
- 不声称完成真实平台授权、支付、Sui 主网提交或生产部署。
- 不改变商户白名单、签名、Grant、动作确认、审计和降级恢复等既有业务边界。

## 预计实现范围

- `server/src/outbound_public_address_policy.rs`
- `server/src/open_commerce_outbound_security.rs`
- `server/src/compute_federation/external_pool_adapter_broker_tls/address_policy.rs`
- `server/src/compute_federation/external_pool_adapter_broker_tls/tests.rs`
- `docs/open-commerce-outbound-public-address-pinning-v1-acceptance.md`
- `docs/open-commerce-merchant-runtime-egress-pinning-v1-acceptance.md`
