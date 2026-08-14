---
title: 外部矿池 Adapter authenticated no-work probe V1 需求
status: accepted
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
feature_id: compute-external-pool-adapter-authenticated-no-work-probe-v1
---

# 外部矿池 Adapter authenticated no-work probe V1 需求

## 1. 目标

在 V263 已认证 child 与 V264 server-owned Broker TLS channel 之间实现一次严格有界的
application no-work request/response。请求必须由已取得 exact V256 ephemeral bundle 的 child
生成，服务端只负责把请求转发到 exact V258 target、按 child 声明的固定长度读取响应，再把
响应送回同一 authenticated ELSP session 由 child 验证。

成功只生成 Store-private、进程内、短时且不可序列化的 no-work observation。该 observation
证明本次 child 在 exact V256/V258/V259/installation roots 下验证了一个无任务响应，不证明
真实算力、任务领取能力、Provider readiness、市场准入、用量或结算。

## 2. 强制边界

1. 复用 V259 已授权的 authenticated `Control` frame，在其 payload 内定义独立 `ELNW`
   子协议；不修改 V259 durable policy，不增加未授权 ELSP frame kind。
2. 每个 child lifecycle 最多执行一次 probe。request 与 response 都必须非空并受服务端固定
   上限约束；response 使用 exact byte length，不允许读到 EOF、超时或分隔符猜测边界。
3. child 生成随机 probe nonce；request、期望 response 长度、response 以及 child validation
   receipt 必须绑定到同一个 probe root。原始 request/response 使用 zeroize-on-drop custody，
   不写数据库、日志、HTTP、MCP、PC 或 APK。
4. Adapter child 继续没有 network authority。DNS、TCP、TLS 和应用字节只允许位于 server
   Broker 模块；不使用代理、重定向、0-RTT 或 client certificate。
5. SQLite transaction、database connection 和 Prepared installation handle 不得跨越任何
   DNS/TCP/TLS/application await。连接前取得 exact roots；TLS 建立后、应用 exchange 前及
   exchange 后都必须重新证明 current bundle、companion、target、capsule 与完整 installation
   binding 未漂移。
6. probe timeout 必须同时符合 V255 launch profile 与 V259 companion 的
   `authenticated_no_work_readiness_v1` / `probe_timeout_ms`，当前固定为 15 秒。
7. 任一 frame、长度、sequence、MAC、root、currentness、TLS identity、timeout、child semantic
   validation 或 shutdown/reap 失败均不得形成 observation；session 终止且 child 有界回收。
8. 成功 observation 不持久化、不开放公共路由，最迟在 probe timeout 到期；消费 callback
   只能看到最小状态，不可取得 socket、secret、request、response、receipt raw bytes 或根明细。

## 3. 非目标

- 不连接生产 upstream，不使用生产 config、credential、账号、CA 或 SPKI。
- 不实现任务领取、share/job/attempt/start、算力派发、真实 usage、verification 或 settlement。
- 不创建 route/service actor，不改变 Provider `registering`，不移除 V254 18 项 deny。
- 不新增 HTTP、MCP、PC、APK、Sui 或公开 SDK 入口，不发布或部署服务。
- 不声明第三方生产 Adapter ABI 已兼容；动态测试只使用仓库固定测试 capsule 与本地 TLS fixture。

## 4. 验收标准

1. ELNW 正向协议证明 child-generated request、exact response length、child semantic validation、
   receipt root 和 graceful shutdown/reap 全部闭合。
2. 错误 kind、空/超限 payload、长度漂移、错误 response、重复 completion、timeout 和截断响应
   均失败关闭，且不返回 observation。
3. Broker 本地 TLS fixture 证明应用 exchange 只发生一次、request exact、response exact；额外
   或不足 response bytes 不被推断为成功边界。
4. source-contract 证明 transaction 不跨 network await、V263 child 无网络能力、无持久化或
   公共入口、Provider/market/usage/settlement/Sui 边界不变。
5. 生产 Rust check、定向 Windows 测试、Linux cross-build 与可用时的 WSL kernel/session 验证
   通过；证据和未验收项写入独立 acceptance 文档。
