---
title: 商户运行时 HTTP 宿主 V1 验收
status: current
owner: open-commerce
reviewed_at: 2026-08-16
---

# 商户运行时 HTTP 宿主 V1 验收

## 已验证范围

- 独立子路径创建默认回环 HTTP 宿主和调用方证书 HTTPS 宿主。
- 本机真实 TCP 请求保留原始签名字节并进入既有商户运行时。
- 相同调用经 HTTP 重放只执行一次业务处理器并返回同一结果。
- 健康读取不调用商户能力、不创建幂等记录，也不暴露商户身份。
- 方法、路径、媒体类型、内容编码、声明体积、分块累计体积和 `Expect` 在运行时前拒绝。
- 运行时签名错误保留既有安全错误信封；抛错和非法响应只返回通用宿主错误。
- 优雅停机等待真实在途异步处理器；截止强制关闭连接并明确保留未完成计数。
- 重复关闭返回同一回执，不重复操作服务器或业务状态。

## 验证证据

2026-08-16 在隔离工作树执行：

```text
node --test sdk/open-commerce-connector/test/merchant-runtime-http-host.test.mjs
结果：7/7 通过

cd sdk/open-commerce-connector && npm test
结果：105/105 通过

cd sdk/merchant-erp-kernel && npm test
结果：13/13 通过

cd sdk/open-commerce-connector && npm pack --dry-run --json
结果：24 个发布文件，包含宿主 JS/类型定义，不包含测试证书私钥
```

HTTPS 专项使用仓库内公开、仅供本机测试的自签名 localhost 证书。该私钥不属于任何部署，
不能被复制为生产凭据。

## 尚未验证

- 生产证书、真实域名、反向代理、系统服务、防火墙和公网流量。
- 发布连接器、自动注册运行绑定、生产密钥注入和密钥轮换。
- 长时间慢请求、恶意流量、进程崩溃、负载均衡和多机高可用压力。
- 真实支付、履约、外部平台授权或生产 ERP 灾难恢复。

因此，本验收只证明宿主代码和本机网络边界可复用，不证明商户节点已经完成生产部署。
