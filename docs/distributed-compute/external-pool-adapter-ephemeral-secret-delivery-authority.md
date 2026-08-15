---
title: 外部矿池 Adapter 易失配置与凭据交付权威
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
verification_status: historical_v263_fixture_superseded_v267_rerun_required
---

# 外部矿池 Adapter 易失配置与凭据交付权威

## 1. 唯一语义：把 V256 短时字节交给 V262 受限 child

V263 只解决一件事：服务端在 V256 runtime bundle authority 仍 current、retained
filesystem handles 与 locked memory 都未漂移时，把借用的 config 和 credential 通过
V262 已完成 mutual bootstrap 的匿名 ELSP session 交给同一受限 child。成功效果只表示
该 child 已确认收到并持有 exact bytes；它不表示 Adapter 已联网、能够领取任务或具备
Provider activation authority。

V263 不解析 config 或 credential 的业务格式，不把 credential 当作 session key，也不把
两类字节放入 argv、环境变量、临时文件、日志、数据库、HTTP、MCP 或公开 DTO。V258
hostname、port 与 SPKI 仍未使用，DNS、TLS、TCP 和 upstream no-work probe 均不在本批。

## 2. 一次性 delivery root

host preparation 接收 bundle generation 和两段借用字节，并使用 OS CSPRNG 生成 32-byte
nonce。固定 domain、generation、两段长度、nonce、config SHA-256 和 credential SHA-256
共同形成 delivery root。该 root 作为 V262 session roots 中的 bundle root，在 exec 前绑定
child；nonce 只在 mutual authentication 完成后的 Control frame 中传输。

host preparation 不保存 config 或 credential 副本，只保存 nonce、generation、长度、两项
摘要和 root。真正发送前必须重新计算借用字节的长度与摘要，并与 preparation 和 session
bundle root 做 constant-time 比较。任一 currentness、内容、长度或 root 漂移都会先终止
session，不能降级为不带验证的发送。

## 3. 固定协议状态机

V263 复用 V260 的 ELSP framing、方向密钥、序号、HMAC 和 terminal fail-close，不建立第二套
socket 或认证协议。delivery control frame 使用固定 `ELSD` magic、版本 1、零 flags，顺序为：

```text
begin -> config -> credential -> receipt -> commit -> ready
```

`begin` 固定携带 generation、两段长度、nonce 和 root；config 与 credential 必须使用各自
ELSP frame kind，并服从既有 1 MiB 与 64 KiB 上限。child 收齐后重新计算 root，只有 exact
匹配才发送 receipt。host 回送 commit 后，child 才发送 ready 并形成进程内 delivered
authority。错误 kind、顺序、长度、root、摘要、确认或连接状态都使 session terminal。

正常 no-work 收尾固定为 `shutdown -> shutdown_ack`。child 在发送 ACK 前清零 generation、
root、config 与 credential；host 收到 ACK 后清零 receipt root。连接中断、协议失败或回调
失败时，V261 child handle 的 Drop/pidfd 路径继续负责终止与回收，并清理 cgroup leaf 和
scratch root。

## 4. Store 私有组合

Store 入口只在 Linux x86-64 编译，并使用一个 `TransactionBehavior::Immediate` 事务和同一
`checked_at` 组合以下 authority：

1. V256 current runtime bundle、retained handles 与 locked bytes；
2. V259 current supervisor/session companion，以及其内含的 V258 target authority；
3. V250/V252 current vulnerability 与 sandbox roots；
4. V257 sealed entrypoint capsule；
5. V260/V262 session roots、mutual bootstrap 和 V261 launcher。

V256 bundle 与 V259/V258 路径分别接收独立 V249 Prepared installation handle；组合前必须
比较完整 installation binding，不能只比较路径字符串或公开 digest。两段敏感字节只在
Store-owned callback 中借用，callback 外无法取得 slice、hash、nonce、fd、PID、cgroup 或
本机路径。

child 返回 final ready 后，Store 才向同一私有 callback 暴露不可 Clone、不可 Debug、不可
序列化的 delivery authority。当前 callback 只能证明 `secret_delivery_ready=true`；随后立刻
执行 no-work shutdown 和有界 reap。V263 没有公共调用者，也没有写入 Provider、route、
market、usage、settlement 或 Sui 状态。

## 5. 模块边界

- `external-pool-adapter-session-core/src/delivery.rs`：delivery root、codec、状态机和清零；
- `external_pool_adapter_supervisor_session.rs`：服务端对 session core 的窄 re-export；
- `external_pool_adapter_session_fixture_main.rs`：显式 feature 下的静态假 child；
- `external_pool_adapter_linux_supervisor/authenticated_runtime_tests.rs`：exec 后正向与 root
  drift kernel fixture；
- `compute_external_pool_adapter_runtime_bundle/secret_delivery.rs`：Store 私有组合与收尾；
- source-contract tests：冻结 no-secret-egress、no-network、no-persistence 和 no-economics
  边界。

测试 capsule 只接受仓库固定的非生产 config/credential。它不是第三方 Adapter ABI，也不
进入普通服务端发布目标。生产 Adapter 仍必须经过后续独立兼容性、网络和运维验收。

## 6. 不变边界与下一硬门

V254 的 18 项 temporary absolute deny 继续逐字保留，Provider 继续 `registering`，
`runtime_launch_ready=false`、`activation_ready=false`。V263 不创建 route/service actor，
不开放市场准入，不生成 usage、settlement 或链上 effect，也不发布 Server、PC 或 APK。

V263 已完成 Store 私有组合、cross-build 与真实 Linux kernel fixture。后继 V264/V265 已在
不向 child 扩大网络 authority 的前提下，使用 V258 exact target 完成 Broker TLS seam，并在
固定本地 fixture 上完成 authenticated upstream no-work probe。probe 形成独立、可过期、
可复验的进程内 observation；它仍不能单独移除 V254 deny 或激活 Provider。V263 详细动态
证据与尚未验收项见
[`external-pool-adapter-ephemeral-secret-delivery-acceptance.md`](external-pool-adapter-ephemeral-secret-delivery-acceptance.md)。

## 7. V267 状态更正

V263 的历史 Secret delivery fixture 发生在 exec 后 dumpable 未重新置零的旧 runtime 上，因此
不能证明同 UID ptrace 暴露面在 config/credential 交付前已关闭。V267 保持 profile/installation
绑定 source capsule SHA-256，但把 session capsule root 改为派生 launch SHA-256，并让 launch
stub 在原 entry 前 SET/GET dumpable；Store 私有 binding 同时记录 source/launch digest 与
launch size。current policy V2 另要求 Yama 2/3、受限 prctl 与修正后的 execveat 参数过滤。

这些当前源码仅 `source_review_only`，未编译、未交付任何测试 Secret，`passed=0`。V263 旧
`18 passed` 保留历史 provenance，但整个 Store→launch→bootstrap→delivery→shutdown/reap
矩阵必须在 V267 上重跑，才能恢复当前 Secret delivery 的动态验收结论。
