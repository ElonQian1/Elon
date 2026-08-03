---
title: 开放商业 HTTPS 出站公网地址固定 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 开放商业 HTTPS 出站公网地址固定 V1

## 背景

精确主机白名单只能限制域名文本，不能单独阻止白名单域名解析到回环、私网或特殊用途地址，也不能阻止安全检查与真正连接之间发生第二次 DNS 解析。域名验证、Webhook challenge 和真实 Webhook 投递必须共享同一出站安全边界。

## 决定

1. 开放商业 HTTPS 出站调用在请求前解析目标主机的全部地址；只要任一结果属于私网或特殊用途地址，整次调用失败关闭。
2. IPv4 拒绝未指定、私网、回环、链路本地、共享地址、基准测试、文档、组播和保留地址；IPv6 拒绝未指定、回环、组播、唯一本地、链路本地、站点本地、文档、6to4、NAT64 特殊前缀及映射后的非公网 IPv4。
3. 通过检查后，客户端禁用环境代理和重定向，并用 reqwest DNS override 把本次请求固定到已检查的公网地址，避免连接阶段再次解析到不同地址。
4. 当前优先选择公网 IPv4，缺少 IPv4 时使用第一个已检查的公网 IPv6；下一次验证或投递会重新解析和检查。
5. 域名验证、Webhook challenge 和每一笔真实 Webhook 投递均使用此模块。实际投递如果目标重新解析为不安全地址，会失败关闭并停用订阅。
6. 地址固定只降低 SSRF 与 DNS 重绑定风险，不替代主机白名单、TLS 证书验证、签名、超时、响应限制或接收端幂等。

## 实现引用

- `server/src/open_commerce_outbound_security.rs`
- `server/src/open_commerce_developer_domain_service.rs`
- `server/src/open_commerce_webhook_verification.rs`
- `server/src/open_commerce_webhook_worker.rs`
- `docs/open-commerce-outbound-public-address-pinning-v1-acceptance.md`
