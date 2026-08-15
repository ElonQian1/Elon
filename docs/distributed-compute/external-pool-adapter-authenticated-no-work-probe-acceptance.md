---
title: 外部矿池 Adapter authenticated no-work probe 验收边界
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: historical_v265_and_v267_controlled_fixture_verified
---

# 外部矿池 Adapter authenticated no-work probe 验收边界

## 本批状态

V265 已实现 ELNW one-shot 子协议、child-generated request、exact-length server broker exchange、
child semantic validation、root-bound receipt、post-exchange Store reproof 和短时私有 observation。
生产 Rust check、Windows 定向动态/源码合同、Linux-musl 产品与 fixture 链接、Linux session-core
单元测试和 WSL2 真实 exec/cgroup fixture 均已执行。

所有 application bytes 都是仓库固定测试值，网络仅使用 loopback TLS fixture。没有真实矿池、
生产 Secret、Provider activation、任务、用量、资金、链上状态或部署。

## V267 状态更正

下文 Windows/Linux/WSL2 结果来自 V267 之前的 V1/source-capsule runtime。它们保留 ELNW 与
loopback TLS 的历史 provenance，但没有覆盖 current post-exec dumpable、Yama、launch root、
policy V2、ancillary rejection 和 lifecycle cleanup，因此不能计作 V267 passed。

V267 已编译，ignored runtime fixture 已经 production materializer 生成，并在 Yama 2 下通过
authenticated no-work、shutdown/reap 与 cgroup/scratch cleanup。完整 Store orchestration、真实
Secret delivery、ELNW/TLS exchange、response semantic validation、postflight currentness 与
生产 fault matrix 仍须补齐。

## 动态与源码合同矩阵

1. ELNW 正向：child 生成 nonce/request/response length，host 只完成一次，child 验证 exact
   response 后返回 authenticated receipt；
2. ELNW 失败关闭：错误 response 长度和 child 语义拒绝均令 host/child terminal；
3. Broker 正向：本地 TLS 1.3 server 精确接收 request，client 精确读取固定 response；
4. Broker 失败关闭：截断 response 不形成成功结果，同一 channel 第二次交换被拒绝；
5. Store source contract：TLS connect、application exchange 均不跨 SQLite transaction，exchange
   后复验完整 bundle/companion/target/capsule/installation roots；
6. 无副作用 contract：无 observation 持久化、公开路由、Provider activation、market、usage、
   settlement 或 Sui 写入；
7. Linux session-core：2 项线程级正向/失败关闭测试通过；
8. WSL2 kernel：真实 sealed fixture exec 后完成 Secret delivery、ELNW probe、graceful shutdown、
   pidfd wait、scratch 与 cgroup 清理。

## 验证命令

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- check --manifest-path server\Cargo.toml

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain agent-validation -- test --manifest-path server\Cargo.toml --bin elon-server no_work

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-cross.ps1 -- zigbuild --target x86_64-unknown-linux-musl --manifest-path server\Cargo.toml --locked --features external-pool-adapter-session-fixture --bin elon-server --bin elon-external-pool-adapter-session-fixture
```

WSL2 另以 root delegated cgroup、预构建静态 fixture 和仓库 exact ignored test 执行；测试脚本是
未提交的验证工件，不是产品入口。全部 Rust 构建复用共享缓存，未使用 Docker。

## 验证回执

| 证据 | 指纹或结果 |
|---|---|
| production Rust check | `6f24baf26ccb30c4209671a2f3b0ccdd964eca954bafca46a4936094f4e59898` |
| Windows no-work tests | `a7dfe34bfb8ba3d0a0aee0972fefa8ace1ad4e83807558a8c251fb1c959cf9a3` |
| Windows result | `6 passed / 0 failed / 1962 filtered out` |
| Linux-musl `elon-server` | `19ac3079666d3f62e1263fa8ddad7ecbd2d79fdf8d1165b4e8d1fd8e4fb9709c` |
| Linux-musl fixture | `1866fdfc9ddd43e162a174631a931a5f388b3dcc115428bceceab2d3028d305b` |
| Linux session-core log | `5a63494bf72ad007c73291f29edfb3e772483645b74c5b4b631cdd30408c339a` |
| Linux session-core result | `2 passed / 0 failed` |
| WSL2 kernel log | `6d2980109c5fdf5ec086af44856de518dacbece851a8f28900ae043b18dc0551` |
| WSL2 kernel result | `1 passed / 0 failed`, `V265_WSL_CGROUP_CLEAN=true` |
| combined V265 fingerprint | `1e6d35c1d91d7dc6cdbf1e53c31486ab78001ebb768466f8d90356d575116984` |

组合指纹按表中 production check、Windows tests、两份 Linux-musl 工件、session-core log 和
kernel log 的小写 SHA-256 依次以 LF 连接后再次计算 SHA-256。机器级日志和共享 validation
evidence 不属于产品状态或发布工件。

## 未验收与禁止声明

- 未用完整 SQLite fixture 动态调用 Store 顶层 no-work orchestration；历史 V265 层曾有 production
  compile/source-contract 证据，但 current V267-V270 已实质改为六次晚绑定与 cleanup-before-callback，
  当前组合只有 source-contract 证据且尚未编译；
- 未连接生产 DNS、公网 upstream、真实 CA/SPKI、真实 Adapter binary、账号或 Secret；
- 未验证响应协议在不同第三方矿池中的 framing、no-task 语义、重连、错误映射和运维兼容性；
- 未做生产 Linux host、network namespace、代理污染、IPv4/IPv6 故障转移、长时稳定性或
  高并发压力验收；
- 历史 V265 fixture 未生成 durable observation 或 readiness；V270 现有 cleanup 后短时 readiness
  receipt 与 admin/owner HTTP 源码仍未编译、运行或动态验收，也没有 route/service actor、Provider
  activation、任务、usage、settlement 或 Sui effect；
- V265 本身未开放直接 HTTP/MCP/PC/APK；V270 间接 trigger 当前仅为未运行源码，未发布服务器或安装包。

因此 V265 direct-seal 旧 runtime 只保留历史 provenance；current V267 production-materialized
no-work fixture 已有 WSL2/Yama 2 kernel subset 证据。不能声明外部矿池生产接入、真实 upstream、
算力供应或交易结算链路完成。
