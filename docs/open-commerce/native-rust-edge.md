---
title: 开放商业 Rust 原生 HTTPS 入口
status: current
owner: backend
reviewed_at: 2026-08-21
---

# 开放商业 Rust 原生 HTTPS 入口

## 作用

`yilong-commerce-edge` 在电商托管节点直接完成 TLS 终止和受限反向代理，不要求安装 Nginx。它只把固定商户公开端点转发到同机回环地址，不读取 ERP 数据，也不接管一龙平台的授权、签名、计量或结算。

```text
https://commerce.example.com/merchants/coffee-a
  -> yilong-commerce-edge :443
  -> http://127.0.0.1:18081
  -> coffee-a 独立 ERP 进程
```

每个商户继续拥有独立进程、系统用户、端口、数据库和秘密文件。入口共用证书和公网端口，但不会把一个商户的请求路由到另一个商户。

## 公开范围

| 公网路径 | 上游路径 | 方法 |
|---|---|---|
| `/health` | 入口自身健康状态 | `GET` |
| `/merchants/<instance>/health` | `/health` | `GET` |
| `/merchants/<instance>/commerce/v1/manifest` | `/commerce/v1/manifest` | `GET` |
| `/merchants/<instance>/commerce/v1/invoke` | `/commerce/v1/invoke` | `POST` |

其他路径、查询参数和方法均失败关闭。尤其不能通过该入口访问 `/api/admin/*`、数据库、上传目录、诊断浏览器或商户后台。

## 配置版本

V1 遵循 `contracts/open-commerce/native-edge-v1.schema.json`，继续支持运维提供的 PEM 文件。配置文件只引用证书路径，不包含证书正文、私钥、运行密钥或数据库凭据。

```json
{
  "schema": "yilong.commerce-edge.v1",
  "listen_addr": "0.0.0.0:443",
  "certificate_chain_path": "/etc/yilong-commerce-edge/tls/fullchain.pem",
  "private_key_path": "/etc/yilong-commerce-edge/tls/privkey.pem",
  "public_hosts": ["commerce.example.com"],
  "connect_timeout_ms": 2000,
  "request_timeout_ms": 15000,
  "tls_handshake_timeout_ms": 10000,
  "connection_timeout_ms": 30000,
  "reload_interval_seconds": 5,
  "max_request_body_bytes": 1048576,
  "max_response_body_bytes": 4194304,
  "routes": [
    {
      "instance_id": "coffee-a",
      "public_base_path": "/merchants/coffee-a",
      "upstream_addr": "127.0.0.1:18081",
      "enabled": true
    }
  ]
}
```

V2 遵循 `contracts/open-commerce/native-edge-v2.schema.json`。它把证书来源改成显式 provider；以下配置使用 Rust 原生 ACME TLS-ALPN-01，公网只需开放 443：

```json
{
  "schema": "yilong.commerce-edge.v2",
  "listen_addr": "0.0.0.0:443",
  "certificate_provider": {
    "mode": "acme_tls_alpn_01",
    "domains": ["commerce.example.com"],
    "contact": "mailto:ops@example.com",
    "cache_dir": "/var/lib/yilong-commerce-edge/acme",
    "environment": "staging"
  },
  "public_hosts": ["commerce.example.com"],
  "routes": [
    {
      "instance_id": "coffee-a",
      "public_base_path": "/merchants/coffee-a",
      "upstream_addr": "127.0.0.1:18081",
      "enabled": true
    }
  ]
}
```

ACME 的 `domains` 必须与 `public_hosts` 完全一致，监听端口必须是 443，缓存目录必须是绝对路径且最终路径不能是符号链接。先用 `staging` 验证 DNS、防火墙和续签流程；只有受控切换配置并重启后才使用 `production`。V2 也允许 `mode: pem`，便于先升级配置合同、后切换证书来源。

`upstream_addr` 只接受回环 IP，且端口必须不小于 1024。`public_base_path` 必须由 `instance_id` 唯一派生，不能写通配符或任意路径。

环境文件只需要指向配置：

```text
YILONG_COMMERCE_EDGE_CONFIG_PATH=/etc/yilong-commerce-edge/edge.json
RUST_LOG=yilong_commerce_edge=info
```

## 安装和启动

安装器默认只预演，不修改目标机：

```bash
bash scripts/install-commerce-edge.sh \
  --binary ./target/release/yilong-commerce-edge \
  --config ./edge.json
```

核对输出后，以 root 显式应用：

```bash
sudo bash scripts/install-commerce-edge.sh \
  --binary ./target/release/yilong-commerce-edge \
  --config ./edge.json \
  --apply
```

安装器创建不可登录的 `yilong-edge` 用户、受限配置目录和唯一可写状态目录，先以服务用户运行候选二进制 `--check-config`，再安装 systemd 单元并启动。已有文件会复制到 `/var/backups/yilong-commerce-edge/<时间戳>/`；新服务未能进入 active 时自动恢复旧文件和原启用状态。输出不包含配置正文或密钥。

各商户 ERP 必须先只监听 `127.0.0.1:<独立端口>`，且 `/health` 返回成功。真正启动入口时会探测全部启用路由；任一商户不健康时入口拒绝启动。`--check-config` 只解析合同并验证 PEM 材料，不探测上游、不绑定端口、不访问 ACME，也不会创建 ACME 缓存。

入口只需要绑定 443 的系统能力，不需要 root 身份持续运行。systemd 模板通过 `CAP_NET_BIND_SERVICE` 提供最小能力，并启用只读文件系统、私有临时目录、受限地址族和 `/var/lib/yilong-commerce-edge` 独立状态目录。SIGINT 或 systemd 的 SIGTERM 会停止接收新连接，现有连接最多排空 15 秒；systemd 在 20 秒后执行最终终止。

## 热更新和回滚

入口按 `reload_interval_seconds` 读取配置摘要。内容变化后依次执行：

1. 解析并严格校验新配置；
2. 确认监听地址、证书 provider、超时和资源上限未改变；
3. 构建一张新的不可变路由表；
4. 探测所有启用上游的 `/health`；
5. 一次性替换当前路由表。

任一步失败都会保留旧路由表。回滚时恢复上一份配置；它通过健康检查后会自动重新生效。监听地址、证书 provider、证书文件内容、ACME 环境或资源上限变化需要受控重启。

## 平台绑定

单个商户的一龙 Runtime Binding 使用商户基础路径：

```text
https://commerce.example.com/merchants/coffee-a
```

平台仍按 `docs/open-commerce/merchant-runtime.md` 生成并验证 `x-yilong-runtime-key-id`、`x-yilong-runtime-timestamp` 和 `x-yilong-runtime-signature`。入口保留这些头和原始 JSON 字节，但不会记录其值或正文。

## 验证

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -- `
  test --manifest-path server\Cargo.toml --bin yilong-commerce-edge
```

部署后先执行只读检查：

```bash
curl -fsS https://commerce.example.com/health
curl -fsS https://commerce.example.com/merchants/coffee-a/health
curl -fsS https://commerce.example.com/merchants/coffee-a/commerce/v1/manifest
curl -i https://commerce.example.com/merchants/coffee-a/api/admin/stores
```

最后一个请求必须返回 `404`。`/commerce/v1/invoke` 不接受无签名调用，不能为了验活放宽商户运行时的 HMAC 验证。

## 当前边界

V2 已提供 Rust 原生 ACME TLS-ALPN-01、持久缓存、配置预检、systemd 模板和安装工具；ACME 只替换证书提供器，不扩大代理路径、上游范围或商户数据权限。代码和本机测试通过不代表生产 DNS 已解析、443 已放行、Linux 服务已安装或真实 CA 已签发。生产启用仍需目标服务器、域名、联系人和变更窗口。
