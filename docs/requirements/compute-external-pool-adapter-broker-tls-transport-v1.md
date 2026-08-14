---
title: 外部矿池 Adapter 服务端 Broker TLS 传输 V1
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: planned
verification_status: not_verified
---

# 外部矿池 Adapter 服务端 Broker TLS 传输 V1

## 目标

在 V258 exact upstream target 与后续应用层 no-work probe 之间实现 server-owned broker TLS 传输门。服务端只从 current V258 Store authority 准备 canonical DNS hostname、port、exact SNI、expected leaf SPKI pin 和固定 policy limits；fresh DNS 必须有界、去重且全部为公共单播地址，TCP 只能直连本次选中的地址，不使用系统或应用代理。TLS 只允许 1.3，使用服务端当前 WebPKI roots 验证证书链、hostname 和时间，再以 constant-time 比较 exact leaf SubjectPublicKeyInfo SHA-256。

网络操作必须在数据库事务外执行，随后使用独立 Prepared installation handle 在一个 `Immediate` 事务中重新取得同一 V258 current authority；target digest、policy、hostname、port、SNI、SPKI 与完整 installation binding 任一漂移都丢弃通道。只有复验通过时，Store-private callback 才能短时借用不可 Clone、不可 Debug、不可序列化的 authenticated TLS channel authority。

## 非目标

- 不通过 TLS 发送 config、credential、Adapter request 或任意应用数据，不实现 no-work request/response。
- 不修改 V263 child/session，不把 socket、DNS answer、目标地址或 TLS handle 交给 child。
- 不使用真实矿池、生产 Secret、生产客户端证书或外部付款，不执行生产网络验收或部署。
- 不创建 durable observation、HTTP/MCP/PC/APK API、Provider service actor、route、activation、Pool、Offer、Job、Attempt、usage、settlement 或 Sui effect。
- 不放宽 V254 的 18 项 temporary absolute deny；Provider 继续 `registering`。

## 架构边界

1. 网络实现位于独立 broker TLS 模块；V258 Store、V263 secret delivery 与 session core 不直接导入 DNS、TCP 或 TLS。
2. DNS 服从 V258 `max_dns_answers` 与 timeout；空结果、重复后越界、任一私网/环回/链路本地/文档/保留/多播地址均失败关闭，不能只挑出一个公共地址继续。
3. connect 最多使用 V258 `max_connect_attempts`，每次将 hostname 保留给 TLS SNI/证书验证，同时把 TCP 固定到同一已验证 `SocketAddr`；禁用 proxy、redirect、0-RTT 与 client certificate。
4. rustls 配置只启用 TLS 1.3 和 WebPKI roots。握手完成后必须取得唯一 leaf certificate，结构化解析 SPKI DER 并与 V258 expected pin constant-time 比较。
5. 失败不能降级到 TLS 1.2、跳过 hostname/time/WebPKI、接受 pin-only 证书、自签证书或私网地址，也不能持久化网络或证书原文。
6. Store 预备与后验复验使用两个独立 V249 Prepared handles；网络不持有数据库锁，callback 则在后验 `Immediate` currentness transaction 内执行。
7. authenticated channel 只暴露 transport identity/时效的私有只读证明；本批不提供 read/write 应用数据方法。Drop 必须关闭连接，失败不得产生 Provider 或经济副作用。

## 验收标准

1. 源码合同证明只有新 broker TLS 模块拥有 DNS/TCP/rustls，V258/V263/session/Provider/market 路径没有新增网络或公开入口。
2. 地址策略单元测试覆盖公共 IPv4/IPv6、环回、RFC1918、CGNAT、链路本地、文档、保留、映射 IPv4、多播和混合 DNS answer 失败关闭。
3. 本机 TLS fixture 使用临时测试 CA 与 `localhost` 证书，真实完成 TLS 1.3、hostname/time/WebPKI 与 SPKI pin 正向验证。
4. 动态失败用例至少覆盖 hostname 不匹配、SPKI 不匹配、不受信 CA、TLS 1.2-only、超时/拒绝连接，并证明无应用字节发送。
5. Store source-contract 证明 preflight/network/postflight 顺序、两个 Prepared handles、exact target/install binding 比较、后验事务 callback 与零持久化副作用。
6. 既有 V258/V259/V260/V261/V262/V263 边界和 V254 18 deny 保持；文档明确 TLS channel 已验证不等于 no-work probe、生产 Adapter 或 Provider activation。

## 预计实现范围

- `server/src/compute_federation/external_pool_adapter_broker_tls/`
- `server/src/store/compute_external_pool_adapter_broker_tls.rs`
- `server/src/compute_federation/*source_contract_tests.rs`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `docs/distributed-compute/`
- `AI_CURRENT.md`

## 依赖

- `compute-external-pool-adapter-ephemeral-secret-delivery-v1`
- `compute-v258-upstream-transport-target-verification`

## 当前结果

尚未实现。V258 只封存 target/policy，V263 只把测试 Secret 交给无网络 child；当前没有 server broker DNS、TCP、TLS 或 SPKI observation，不能宣称上游身份或连接可用。
