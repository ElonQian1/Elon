---
title: 开放商业 HTTPS 出站公网地址固定 V2
status: accepted
owner: backend-security
reviewed_at: 2026-08-15
supersedes: open-commerce-outbound-public-address-pinning-v1
---

# 开放商业 HTTPS 出站公网地址固定 V2

## 背景

V1 已要求在请求前检查全部 DNS 答案并固定连接地址，但开放商业与算力代理各自维护
公网地址分类，规则已经漂移；开放商业还没有限制 DNS 等待时间和答案数量，并只固定
一个公网地址。单地址模式安全但可用性有限，也无法证明所有出站组件遵循同一地址边界。

## 决定

1. 服务端受控出站连接共用一份公网单播地址策略。策略依据 IANA IPv4/IPv6
   Special-Purpose Address Registry 维护，明确拒绝私网、回环、链路本地、共享、文档、
   基准测试、废弃转换、保留和不可全局到达的特殊用途前缀。IETF Protocol Assignments
   大范围默认拒绝，只放行注册表中更具体且适合公网连接的例外；转换前缀即使标记为
   可全球到达也按 SSRF 边界从严拒绝。
2. 开放商业 DNS 查询最长等待三秒，最多接收 32 个原始答案；第 33 个答案、空结果、
   端口漂移或任一非公网答案都使整次调用失败关闭。去重不绕过原始答案数量限制。
3. 全部答案验证通过后按 IPv4 优先、地址和端口确定性排序，并通过 reqwest
   `resolve_to_addrs` 固定为本次客户端唯一可用的地址集合。故障切换只能发生在这组
   已检查答案内，不能触发第二次系统 DNS 解析。
4. 客户端继续禁用环境代理和重定向；请求 URL、HTTP Host、TLS SNI 和证书主机校验
   继续使用原始域名，地址固定不把请求改写为 IP URL。
5. 域名 challenge、Webhook challenge、每笔 Webhook 投递和每笔商户运行时调用都在
   创建客户端前重新解析。算力代理 TLS broker复用地址分类与答案边界，但继续拥有独立
   的直连 TCP、TLS 1.3、证书主机名和 SPKI 固定策略。
6. 地址固定仍不替代精确主机白名单、签名、重放保护、响应体限制、业务幂等、动作确认、
   网络级 egress 防火墙或生产身份审核。

## 依据

- IANA IPv4 Special-Purpose Address Space：
  `https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xhtml`
- IANA IPv6 Special-Purpose Address Space：
  `https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xhtml`

## 实现引用

- `server/src/outbound_public_address_policy.rs`
- `server/src/open_commerce_outbound_security.rs`
- `server/src/compute_federation/external_pool_adapter_broker_tls/address_policy.rs`
- `docs/requirements/open-commerce-outbound-public-address-policy-v2.md`
- `docs/open-commerce-outbound-public-address-pinning-v1-acceptance.md`
