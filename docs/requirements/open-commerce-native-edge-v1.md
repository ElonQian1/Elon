---
version_status: current
reviewed_at: 2026-08-21
implementation_status: proposed
---

# 开放商业 Rust 原生入口 V1

## 目标

在电商托管节点上提供独立的 Rust HTTPS 入口，将一个公网域名下的商户路径安全路由到同机不同端口的商户 ERP 实例。该入口替代开放商业公网链路中的 Nginx 反向代理职责，但不接管商户业务、平台授权或 ERP 数据。

```text
一龙平台 / 消费者 AI
  -> TLS 1.3 :443
  -> yilong-commerce-edge
  -> /merchants/<instance_id>/固定公开端点
  -> 127.0.0.1:<merchant_port>
  -> 独立商户 ERP 实例
```

## 用户工作流

1. 运维为入口提供 PEM 证书、私钥和 `yilong.commerce-edge.v1` 配置。
2. 入口启动前校验配置、证书路径、域名白名单、商户路径和回环上游地址。
3. 入口探测所有启用商户的 `/health`；任一新增候选不健康时拒绝整批切换。
4. 一龙将商户 Runtime Binding 配置为 `https://<host>/merchants/<instance_id>`。
5. 平台通过该地址读取 Manifest，并转发带 HMAC 签名的能力调用。
6. 修改配置后，入口先验证并探活候选路由，再用整张不可变路由表替换当前版本；失败时继续使用旧版本。

## 安全边界

- 只允许上游为 `127.0.0.1` 或 `::1`，禁止配置 URL、域名、凭据、查询参数或任意内网地址。
- 商户路径必须等于 `/merchants/<instance_id>`，不得配置通配符或根路径。
- 对外只开放商户 `GET /health`、`GET /commerce/v1/manifest` 和 `POST /commerce/v1/invoke`。
- 管理 API、数据库、上传目录、浏览器控制、调试路径和其他商户端点一律返回拒绝。
- 原样保留运行时 HMAC 所需的请求体和端到端请求头；移除 Host、转发头和 hop-by-hop 头，并由入口生成可信转发元数据。
- 请求体、响应体、连接、TLS 握手和上游调用均有明确上限或超时；入口不使用系统代理，也不跟随上游重定向。
- TLS 首期只启用 TLS 1.3 和 HTTP/1.1。私钥不进入 Git、JSON 配置、日志或错误响应。
- 配置热更新不热换监听地址、证书路径或资源上限；这些变化需要受控重启。

## V1 验收标准

1. 新增独立 `yilong-commerce-edge` 二进制，现有 `elon-server` 和商户 ERP 业务代码无需改动。
2. 配置合同拒绝未知字段、重复域名、重复实例、重复公开路径、非回环上游、非法端口和不匹配的商户路径。
3. TLS 监听使用 PEM 证书和私钥，仅接受 TLS 1.3，并限制握手与连接时间。
4. 请求只有在 Host、路径和方法同时命中时才会转发；任意管理路径和查询参数失败关闭。
5. 代理保留 `x-yilong-runtime-*` 签名头和原始 JSON 字节，移除不可信转发头、hop-by-hop 头及 Cookie。
6. 请求和响应超过限制时返回有界 JSON 错误，不把上游地址、文件路径或请求正文写入响应。
7. 配置候选只有在静态身份不变、路由合法且全部启用上游健康时才整体生效；失败候选不覆盖当前路由表。
8. 提供 systemd 加固模板、脱敏配置示例、启动检查和回滚说明。
9. 自动化测试覆盖配置失败关闭、路由白名单、签名头透传、管理路径拒绝和健康候选切换。

## 非目标

- 本批不实现 ACME 账号、DNS-01/HTTP-01 挑战或自动续证；V1 使用受控提供的 PEM 文件，后续可替换证书提供器而不改代理核心。
- 本批不实现 HTTP/2、HTTP/3、WebSocket、流式大文件、静态资源托管或通用反向代理。
- 本批不自动修改 DNS、防火墙、生产证书、真实 Runtime Binding 或咖啡店线上服务。
- 本批不开放 ERP 管理端，不实现支付、资金结算、Sui 上链或真实平台适配器。

## 实现范围

- `contracts/open-commerce/native-edge-v1.schema.json`
- `server/src/commerce_edge_main.rs`
- `server/src/commerce_edge/`
- `scripts/systemd/yilong-commerce-edge.service`
- `docs/open-commerce/native-rust-edge.md`
