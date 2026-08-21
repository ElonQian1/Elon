---
title: 开放商业 Rust 原生入口 ACME V2
status: accepted
owner: backend
reviewed_at: 2026-08-21
---

# 开放商业 Rust 原生入口 ACME V2

## 目标

在不安装 Nginx、Certbot 或其他常驻反向代理的前提下，让 `yilong-commerce-edge` 直接通过 ACME TLS-ALPN-01 获取和续签公开证书，并继续复用 V1 的固定 Host、固定路径、回环上游和失败关闭边界。

V2 必须兼容现有 `yilong.commerce-edge.v1` PEM 配置。生产 ACME 模式使用独立、可持久化且权限收紧的账户与证书缓存；测试默认使用 Let's Encrypt staging，只有配置明确选择 production 时才访问生产目录。

## 用户工作流

1. 运维准备已解析到目标服务器的公开域名，并确保公网 TCP 443 直接到达 Rust 入口。
2. 运维生成 `yilong.commerce-edge.v2` 配置，选择 `acme_tls_alpn_01`、联系人、环境和绝对缓存目录。
3. 安装工具默认只输出预演；显式应用后创建受限系统用户、状态目录、配置、二进制和 systemd 单元。
4. 入口先校验配置并探测全部启用商户上游，再绑定 443；ACME 状态机在同一端口处理挑战、签发和续期。
5. 普通 TLS 请求仍只能进入 V1 已允许的商户公开端点；ACME 不扩大业务路由、权限或数据访问。
6. systemd 停止服务时，入口停止接收新连接并在有界时间内排空现有连接。

## 安全边界

- ACME 域名集合必须与 `public_hosts` 完全一致，不允许为未公开 Host 申请证书。
- ACME 模式只允许监听 443，挑战固定为 TLS-ALPN-01，不额外开放 80 端口。
- 联系人必须是单一 `mailto:` URI；缓存目录必须是绝对路径且不得是符号链接。
- 生产和 staging 必须显式区分；缓存持久化，避免因账户丢失或重复签发触发 CA 限额。
- 正常业务 TLS 仍只允许 TLS 1.3 和 HTTP/1.1；ACME 挑战连接不进入 Axum 路由。
- 安装工具不得读取或输出商户运行密钥、数据库凭据、Token、Cookie 或证书账户私钥。
- 不健康候选路由不能替换当前路由；证书管理失败不得自动放宽 Host、路径或上游限制。

## 验收标准

1. V1 PEM 配置继续解析、构建和完成既有 TLS 冒烟测试。
2. V2 合同拒绝未知字段、非 443 ACME 监听、相对缓存目录、非法联系人、重复域名及 ACME 域名与公开 Host 不一致。
3. ACME 使用文件缓存、明确 staging/production 目录、TLS-ALPN-01 和 Rustls 动态证书解析器。
4. ACME 挑战按 SNI 分流，不进入业务 Router；普通连接仍受 Host、路径、方法、体积和超时限制。
5. SIGINT/SIGTERM 均触发停止接收；现有连接在有界排空期后才被中止。
6. systemd 只授予绑定低端口能力，并为 ACME 缓存提供唯一可写状态目录。
7. 安装工具默认预演，只有显式 `--apply` 才修改目标机；安装前必须调用二进制配置检查。
8. 定向 Rust 测试、构建、Shell 语法检查和本机 PEM TLS 纵向检查形成验收证据。

## 非目标

- 本批不修改生产 DNS、防火墙或现有咖啡店线上入口。
- 本批不真实访问 Let's Encrypt production，不消耗生产签发限额。
- 本批不实现 DNS-01、HTTP-01、通配符证书、外部 HSM 或多节点证书复制。
- 本批不实现商户自助开通、数据库创建、正式 RBAC、平台适配器、支付或 Sui 结算。
- 代码与本地测试通过不等于目标 Linux 服务器已经安装或生产证书已经签发。

## 实现范围

- `server/src/commerce_edge/certificate_config.rs`
- `server/src/commerce_edge/acme.rs`
- `server/src/commerce_edge/tls.rs`
- `server/src/commerce_edge_main.rs`
- `contracts/open-commerce/native-edge-v2.schema.json`
- `scripts/install-commerce-edge.sh`
- `scripts/systemd/yilong-commerce-edge.service`
- `docs/open-commerce/native-rust-edge.md`
- `docs/open-commerce/native-rust-edge-v2-acceptance.md`
