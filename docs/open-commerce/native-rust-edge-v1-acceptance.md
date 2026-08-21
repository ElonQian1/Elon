---
title: 开放商业 Rust 原生入口 V1 验收
status: current
owner: backend
reviewed_at: 2026-08-21
implementation_status: verified
---

# 开放商业 Rust 原生入口 V1 验收

## 结论

V1 已形成独立 Rust 二进制、严格配置合同、Host 与路径白名单、回环上游代理、请求和响应边界、TLS 1.3 监听、健康预检、整表热切换及 systemd 加固模板。它不修改 `elon-server` 的业务路由，也不把商户 ERP 管理端暴露到公网。

## 自动化证据

定向命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- `
  test --manifest-path server\Cargo.toml --bin yilong-commerce-edge
```

结果为 `7 passed; 0 failed`，验证指纹为 `f7cbe3ee6b91b40a527775c3421f03eecd90df87e4360a9f31fcc0f4eb349610`。覆盖：

- 配置默认值和域名归一化；
- 非回环上游、路径不匹配、未知字段和重复域名拒绝；
- Host、方法、路径和查询参数失败关闭；
- HMAC 签名头与原始 JSON 字节透传；
- Cookie 和不可信转发头移除；
- 商户管理路径拒绝；
- 不健康候选保留旧路由，健康候选整体替换。

独立构建命令同样通过，验证指纹为 `ddc35ea639224a126957c5e3353fd12b6d262b7bdf23f9c3d7b485b90d46966d`。

## 本机 TLS 纵向检查

使用一次性自签名证书、一次性回环上游和随机本机端口启动真实 `yilong-commerce-edge` 二进制，结果为：

```text
TLS_SMOKE=passed edge_health=200 merchant_health=200 admin=404 tls12_rejected=true
```

该检查证明 TLS 1.3 握手、入口自身健康、商户健康代理和后台路径阻断均经过真实套接字；TLS 1.2 客户端无法连接。测试没有访问生产域名、真实商户数据库或资金接口。

## 尚未验证

- 尚未在 Linux 真机安装 systemd 单元或绑定生产 443 端口。
- 尚未配置真实 CA 证书、生产 DNS、防火墙和商户 Runtime Binding。
- 尚未执行多商户长期并发、连接压力、证书轮换或故障恢复演练。
- ACME 自动申请和续证不属于 V1；证书内容变化仍需受控重启。

因此本结论是“代码和本机网络纵向已验证”，不等同于“生产商户入口已经发布”。
