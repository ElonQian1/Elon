---
title: 商户运行时 HTTPS 出站地址固定 V1
status: accepted
owner: backend
reviewed_at: 2026-08-03
implementation_status: implementation_uncompiled
---

# 商户运行时 HTTPS 出站地址固定 V1

## 背景

商户运行绑定原本在保存时要求 HTTPS 和精确主机白名单，但实际调用使用普通 HTTP 客户端。运营方后续缩小白名单、目标 DNS 变化，或白名单域名同时解析到公网和私网地址时，已经保存的绑定仍可能向不符合当前策略的地址发起请求。

## 决定

1. 生产商户运行地址只允许精确白名单内的标准 443 HTTPS；测试编译可继续访问本机回环服务。
2. 每次健康验证和业务调用都重新校验已保存地址，确保当前白名单、协议和端口规则仍然成立。
3. 每次调用都复用开放商业 HTTPS 出站安全模块：解析全部 DNS 地址，只要存在任一私网或特殊用途结果便失败关闭。
4. 通过检查后禁用环境代理和重定向，并把本次连接固定到已检查的公网地址；下一次调用重新解析。
5. 地址校验、DNS 解析或客户端构建失败均按基础设施错误处理，现有调用链会把运行绑定降级，要求商户重新验证后才能继续处理业务调用。
6. 该边界不证明商户身份、数据真实性、履约或支付结果，也不替代 HMAC、重放保护、Manifest 摘要、Grant 和用户明确确认。

## 实现引用

- `server/src/open_commerce_runtime_security.rs`
- `server/src/open_commerce_runtime_client.rs`
- `server/src/open_commerce_outbound_security.rs`
- `docs/open-commerce-merchant-runtime-egress-pinning-v1-acceptance.md`
