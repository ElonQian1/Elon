---
title: 外部矿池 Adapter authenticated no-work probe 权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: verified_windows_local_tls_and_linux_kernel_fixture
---

# 外部矿池 Adapter authenticated no-work probe 权威

## 1. 唯一语义：验证一次无任务响应

V265 只解决一件事：把 V263 已认证、已取得短时 Secret 的受限 child 与 V264
server-owned authenticated TLS channel 组合起来，完成一次 child-generated、严格有界的
上游 request/response，并由同一个 child 验证响应的无任务语义。

成功只形成 Store-private、进程内、短时的 observation，证明本次 exact
V256/V258/V259/installation roots 下的 child 接受了一个响应。它不证明上游具备任务、算力、
计量或结算能力，也不使 Provider ready 或 active。

## 2. ELNW 子协议与一次性状态机

V265 不增加新的 ELSP frame kind，而是在 V259 已授权的 authenticated `Control` payload 内
定义固定 `ELNW` v1 子协议：`BEGIN -> REQUEST -> RESPONSE -> RECEIPT`。每一帧继续受 ELSP
方向密钥、sequence、transcript 和 HMAC 保护。

child 使用 OS CSPRNG 生成 32-byte nonce，声明非空 request 和 exact response length。request
最多 16 KiB，response 最多 64 KiB，单阶段 timeout 最多 15 秒。receipt 把 nonce、两侧长度、
request SHA-256 和 response SHA-256 绑定为 probe root。host request 被消费后不能再次完成；
错误 kind、长度、nonce、MAC、sequence、response 语义或 receipt 均令 session terminal。

原始 request/response、nonce 和摘要使用 zeroize-on-drop custody，不写日志或 Store。child
仍无 socket、DNS 或 TLS 权限；只有 server broker 能接触 upstream application bytes。

## 3. Broker 的窄应用权限

V264 TLS channel 只新增一个专用操作：一次 `write_all(request)`、一次
`read_exact(expected_response_bytes)`，完成后 channel 标记已使用且不能复用。broker 不暴露
generic stream、任意 read/write、EOF parser、delimiter parser、HTTP、proxy 或 redirect。

交换沿用 V258 exact hostname、port、TLS 1.3、WebPKI 和 leaf SPKI pin。V265 测试仅连接固定
loopback TLS fixture，不连接生产 upstream。exact-length read 保证不足响应失败；超过已声明
长度的字节不会进入 child、不会进入后续 parser，channel 在本次交换后立即销毁。

## 4. Store 私有编排与 currentness

完整顺序固定为：

1. 使用两份 Prepared installation handle 完成 V264 TLS preflight、事务外 connect 和 postflight；
2. 提交 SQLite transaction 后返回不携带 Prepared 或数据库 authority 的 one-shot channel；
3. 在独立 `Immediate` transaction 内组合 current bundle、companion、capsule、target 与完整
   installation binding，启动 V263 child 并交付 Secret；
4. 比较 broker target 与 child delivery target，接收 child request，再在事务外执行 TLS exchange；
5. 把 exact response 送回同一 ELSP session，由 child 返回 root-bound receipt；
6. 丢弃 response 和 network channel，再用独立 Prepared handles 在 `Immediate` transaction
   中复验 bundle、companion、target、capsule 与完整 installation binding；
7. transaction commit 后，才向 Store-private callback 借用短时 observation；随后 shutdown、
   pidfd wait、scratch/cgroup cleanup。

任何 SQLite transaction、connection 或 Prepared filesystem handle 都不跨 network await。
currentness 漂移返回 `false` 或失败关闭，不降级为旧 root 或部分 root。

## 5. Observation 权限

`CurrentExternalPoolAdapterNoWorkProbeObservationAuthority` 不可 Clone、Debug、序列化或持久化，
只在 `crate::store` 内可见。callback 只能读取 no-work 布尔结论、request/response 字节数、
checked-at 和 expires-at；不能取得 Secret、socket、PID、fd、原始 application bytes、probe root
或内部 root 明细。callback 调用前会检查时效，最迟在 V255/V259 固定 probe timeout 后失效。

当前没有公共调用者、HTTP/MCP/PC/APK 路由或数据库 observation 表。这是有意边界，不是遗漏。

## 6. 模块边界与下一硬门

- `external-pool-adapter-session-core/src/no_work.rs`：ELNW codec、状态机、root 和清零；
- `external_pool_adapter_broker_tls/no_work.rs`：一次性 bounded TLS exchange；
- `compute_external_pool_adapter_runtime_bundle/no_work_probe.rs`：Store 私有组合和 post-exchange
  reproof；
- `external_pool_adapter_session_fixture_main.rs`：固定 request/response 的非生产假 Adapter；
- Linux supervisor tests：真实 exec、fd、cgroup、Secret、probe 和 reap；
- source-contract tests：冻结 no-network-child、no-persistence、no-public-route 和 no-economics。

Provider 继续 `registering`，V254 的 18 项 temporary absolute deny 逐字保留。V265 不创建
route/service actor，不领取 share/job/attempt，不生成 verified usage、settlement 或 Sui effect，
不发布 Server、PC 或 APK。

下一硬门是为生产 Adapter 定义独立兼容性与受控 upstream 验收，再设计与 atomic activation
同批提交的完整 admission gate。单个 no-work observation 不能单独移除 V254 deny。动态证据见
[`external-pool-adapter-authenticated-no-work-probe-acceptance.md`](external-pool-adapter-authenticated-no-work-probe-acceptance.md)。
