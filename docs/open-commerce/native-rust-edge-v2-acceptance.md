---
title: 开放商业 Rust 原生入口 ACME V2 验收
status: current
owner: backend
reviewed_at: 2026-08-21
implementation_status: verified
---

# 开放商业 Rust 原生入口 ACME V2 验收

## 结论

V2 在保留 V1 PEM、固定 Host、固定路径、回环上游和失败关闭边界的基础上，增加 Rust 原生 ACME TLS-ALPN-01、持久证书缓存、SIGTERM 有界排空、离线配置检查和默认预演的 Linux 安装入口。

本页只记录当前提交可复核证据。未真实访问 Let's Encrypt production，未修改生产 DNS、防火墙、咖啡店服务或 Runtime Binding，也未宣称目标 Linux 节点已经部署。

## 自动化证据

定向 Rust 测试：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- `
  test --manifest-path server\Cargo.toml --bin yilong-commerce-edge
```

结果通过，覆盖 12 个配置、路由、代理、热更新和 ACME 分流测试；验证指纹为 `d238d72d0bda1c6773283c24b576f081681c72fd35b1823691d2eae53e418b9b`。

独立 `build --manifest-path server\Cargo.toml --bin yilong-commerce-edge` 通过，验证指纹为 `d174b09e1fa72fc8a250a9040f3fe5bdebd5baca712cf4cd4d7ef942016bd503`。本机 Debug 二进制 SHA-256 为 `4B9A06DE1A6AEDA212A42831107CC82866C157DB033F196C56095C5D3CDBEBF1`；该哈希只用于绑定本次本机验收，不是 Linux 发布工件。

合同和运维静态检查：

```text
SCHEMA_JSON_PARSE=passed
SHELL_SYNTAX=passed
native-edge-v2.schema.json SHA-256=D1A2DD33931925A872883184E45ED80DF59B7294192A7BDB038F9BB71C42A73D
install-commerce-edge.sh SHA-256=FD498E2C5E6EDAB81DF74697A1CE4BD56D141E0625C0F97B26BC099219A8DAA3
```

一次性自签名证书、回环商户和真实二进制的本机纵向结果：

```text
TLS_SMOKE=passed edge_health=200 merchant_health=200 admin=404 tls12_rejected=true
V2_CONFIG_CHECK=passed cache_created=false
INSTALL_PREVIEW=passed apply=false
```

## 覆盖边界

- V1 PEM 配置仍可读取证书、完成 TLS 1.3 握手并拒绝 TLS 1.2；
- V2 拒绝非 443 ACME、相对缓存、非法联系人、重复域名和域名集合漂移；
- 未知 SNI 失败关闭，ACME challenge 与普通业务连接显式分流；
- challenge 连接不会进入 Axum 商户 Router；
- `--check-config` 不绑定端口、不探测商户、不访问 CA、不创建 ACME 缓存；
- SIGINT/SIGTERM 停止接收，活动连接最多排空 15 秒；
- systemd 只有低端口绑定能力和唯一状态写目录；
- 安装器默认预演，显式应用前以服务用户验证候选配置。

## 尚未验证

- 真实 Linux systemd 安装、低端口能力和重启回滚；
- 公网 DNS、TCP 443、防火墙和 NAT 路径；
- Let's Encrypt staging/production 的真实签发与自动续签；
- 多域名、大并发、长期连接和 CA 故障恢复演练。

因此当前验收目标是“生产路径代码与离线运维入口可交付”，不是“生产证书和公网入口已经上线”。
