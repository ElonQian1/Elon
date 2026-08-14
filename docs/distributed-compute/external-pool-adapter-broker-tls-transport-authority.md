---
title: 外部矿池 Adapter 服务端 Broker TLS 传输权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_windows_source_and_local_tls
---

# 外部矿池 Adapter 服务端 Broker TLS 传输权威

## 1. 唯一语义：认证 V258 exact target，不发送应用数据

V264 只解决一件事：服务端从 Store-private current V258 authority 取得 exact DNS
hostname、port、SNI、expected leaf SPKI pin 和固定 policy limits，在数据库事务外建立一条
经过 DNS、TCP、TLS 1.3、WebPKI 和 SPKI pin 验证的短时通道，再回到 Store 中复验同一
target 和完整安装根。

成功只表示服务端此刻连接到了 V258 声明的 TLS 身份。通道没有应用层读写接口，不发送
probe、config、credential、订单、任务或任何 Adapter frame，也不交给 V260-V263 child。
V264 因此不是上游 no-work probe，更不是 Provider activation 或市场准入。

## 2. DNS 与直连地址门卫

生产路径每次连接都执行有界 fresh `A/AAAA` 解析。原始答案数超过 V258 上限、答案为空、
端口不一致，或任一答案属于 loopback、private、link-local、CGNAT、documentation、
benchmark、multicast、unspecified 及其它特殊网段时，整次解析失败关闭，不能只挑出其中的
公共地址继续连接。

全部答案通过后才去重并按 IPv4 优先、地址稳定排序。TCP 直接连接本轮选中的
`SocketAddr`，最多尝试 V258 固定次数；不使用 HTTP、系统代理、应用代理、redirect、
DNS rebinding 后的第二次主机名连接或 child 网络。TLS 认证失败不会换地址重试，防止把
身份失败降级成容错连接。

本地测试辅助明确只在 `cfg(test)` 下绕过公网地址分类，以便连接 loopback TLS fixture；
生产 resolver 和 connector 仍强制执行全部公共单播门卫。

## 3. TLS 1.3、WebPKI 与 exact SPKI

rustls client 只启用 TLS 1.3，不启用 0-RTT、client certificate 或 ALPN 应用协议。信任根
来自服务端构建时当前 `webpki-roots`；SNI 固定使用 V258 exact DNS hostname，由 rustls
校验证书链、hostname 和当前时间。

握手完成后，服务端使用已锁定依赖中的 DER reader 结构化解析 leaf certificate 的
`TBSCertificate.subjectPublicKeyInfo` 完整 DER，并计算 SHA-256。observed digest 与 V258
expected pin 使用 constant-time 比较；证书缺失、DER 结构错误、非 TLS 1.3 或 pin 不一致
均丢弃通道。此解析不依赖字符串切片或证书文本输出。

## 4. Store 前后复验

Store 组合接收两份独立 V249 Prepared installation handle：

1. preflight 普通事务取得 current V258 authority，冻结完整 target receipt、完整
   installation binding 和 broker target snapshot；
2. 事务、连接和 Prepared authority 全部释放后，服务端在事务外执行 DNS/TCP/TLS；
3. postflight 使用 `TransactionBehavior::Immediate` 和第二份 Prepared handle 重新取得
   current V258 authority；
4. exact target receipt、policy/hostname/port/SNI/SPKI snapshot、target digest、完整
   installation binding、TLS 1.3 状态和 30 秒通道时效任一变化都失败关闭；
5. 只有复验通过时，Store-private callback 才短时借用不可 Clone、不可 Debug、不可序列化
   的 metadata-only authority，事务提交后通道立即 Drop。

网络 await 不持有 SQLite transaction、connection 或 Prepared filesystem handle。callback
只可读取 target id、选定地址和 checked-at，不具备应用 I/O 方法。

## 5. 模块边界

- `external_pool_adapter_broker_tls/address_policy.rs`：公网单播分类、全答案门卫和确定性排序；
- `external_pool_adapter_broker_tls/target.rs`：V258 policy/target exact snapshot；
- `external_pool_adapter_broker_tls/transport.rs`：fresh DNS、直连 TCP、TLS 1.3、WebPKI、DER
  SPKI 和短时 channel；
- `store/compute_external_pool_adapter_upstream_transport_target/broker_tls.rs`：前后两阶段
  currentness 与完整 installation binding 复验；
- `external_pool_adapter_broker_tls/tests.rs`：本地 TLS 1.3/1.2 和失败关闭动态矩阵；
- source-contract tests：冻结无应用字节、无持久化和无经济副作用边界。

## 6. 不变边界与下一硬门

Provider 继续 `registering`，V254 的 18 项 temporary absolute deny 逐字保留。V264 不读取或
发送 V256 Secret，不连接真实生产矿池，不生成 route/service actor、probe observation、
readiness、usage、settlement 或链上 effect，也不开放 HTTP/MCP/PC/APK，不部署服务器。

下一硬门是在这条 authenticated channel 上定义 Adapter-generated、严格有界、无任务领取
能力的 application no-work request/response，并把 observation 与 exact V258/V263 roots、
时效和失败关闭状态绑定。该 observation 仍不能单独移除 V254 deny 或激活 Provider。动态
证据见
[`external-pool-adapter-broker-tls-transport-acceptance.md`](external-pool-adapter-broker-tls-transport-acceptance.md)。
