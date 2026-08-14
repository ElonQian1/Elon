---
title: 外部矿池 Adapter 易失配置与凭据交付 V1
status: current
reviewed_at: 2026-08-15
owners: backend, security, ai-economy
implementation_status: planned
verification_status: not_verified
---

# 外部矿池 Adapter 易失配置与凭据交付 V1

## 目标

在 V256 operator-mounted runtime bundle resolver、V259 supervisor/session policy、V260 authenticated session、V261 Linux supervisor 和 V262 exec 后认证运行时之间建立一条完整但仍不联网的密钥交付链。服务端只能在 V256 Store-owned 短生命周期 authority 仍 current、retained filesystem handles 未漂移时借用 config 与 credential；两者只能通过已经完成 mutual bootstrap 的匿名 ELSP session 交付给同一 sealed child。

交付协议必须把 bundle generation、config/credential 长度和内容绑定为一次性 delivery root。该 root 使用 OS CSPRNG nonce 进行域分离，作为 V262 六项 session roots 中的 bundle root；nonce 只能进入已认证 IPC，不得进入 argv、环境变量、日志、数据库、HTTP、MCP 或公开 DTO。child 收到两类 binary-sensitive frame 后自行重算 delivery root，只有与 exec 前绑定的 root 完全一致，且 host/child 完成 receipt、commit、ready 三段确认，才形成进程内的 ephemeral delivered authority。

## 非目标

- 不解析 config 或 credential 的业务格式，不把 credential 当作 session key，也不写临时文件。
- 不连接 V258 hostname/port，不做 DNS、TLS、upstream network 或 no-work probe。
- 不创建 Provider service actor、route、activation、Pool、Offer、Job、Attempt、usage、settlement 或 Sui effect。
- 不开放 HTTP、MCP、PC 或 APK 入口，不部署服务端，不使用生产密钥或真实矿池账号。
- 不放宽 V254 的 18 项 temporary absolute deny，不改变 Provider `registering` 状态。
- 不把测试 capsule、固定假配置或固定假凭据声明为生产 Adapter 兼容性证据。

## 架构边界

1. delivery codec 与状态机放入独立 session core 模块，由 host 与测试 child 共用；不得复制第二套 frame、摘要或确认协议。
2. host preparation 只持有一次性 nonce、generation、长度、摘要和 delivery root，不拥有 config/credential 副本；真正发送时重新核对借用字节与 preparation 完全一致。
3. wire 顺序固定为 `begin -> config -> credential -> receipt -> commit -> ready`。任一 frame kind、长度、generation、root、顺序、重复、缺失或 payload 上限漂移都使 session terminal。
4. child 收到的 binary-sensitive payload 使用 zeroize-on-drop custody；交付失败、commit 缺失、host 断开或 child 退出时不得返回 delivered authority。
5. Store-private composition 必须在同一近当前时间锚中重验 V256 bundle、V258 target、V259 companion、V257 capsule 和 V262 session roots；公开层不能取得 config、credential、content hash、nonce、PID、fd 或本机路径。
6. host 只有收到与 exact delivery root 绑定的 final ready 后才能报告 ephemeral delivery 成功。成功仍只表示 secret 已进入受限 child memory，不表示 Adapter 已联网、可执行工作或可激活。
7. handshake 或 delivery 失败必须关闭 session，并由 V261 pidfd 生命周期终止、回收 child，清理 cgroup 与 scratch；不得留下 Provider、route、usage、market 或 settlement 副作用。

## 验收标准

1. Windows/当前宿主 source-contract 证明 V256 sensitive borrow 只能进入 V263 Store-private composition，且源码中不存在 secret argv、secret env、临时凭据文件、持久化或公开 API。
2. session core 单元测试覆盖 preparation 重验、固定帧顺序、Config/Credential 上限、generation/size/root 漂移、错误 frame kind、重复/缺失确认和 terminal fail-close。
3. Linux-musl 可完整链接服务端产品目标、测试目标和静态测试 capsule；普通服务端构建不会把测试 capsule 作为产品入口。
4. WSL2/真实 Linux kernel fixture 使用固定假 config/credential，证明 exec 后 child 完成 mutual authentication、接收两类 binary-sensitive frame、重算 root，并完成 receipt/commit/ready。
5. 动态 fixture 证明 delivered payload 不经 argv/env/文件出现；运行时 fd、cgroup、namespace、private root、rlimit、capability、no-new-privileges 和 seccomp 边界保持 V261/V262 约束。
6. 至少一项动态失败用例证明内容或 root 漂移时不形成 delivered authority，child 被回收，cgroup 与 scratch 清理，并保持零网络、零 Provider、零市场和零经济副作用。
7. V256、V259、V260、V261、V262 既有回归保持通过，V254 absolute deny 不变。
8. 权威状态、acceptance 证据和 Feature Registry 明确区分“易失 Secret delivery 已验证”与“TLS/upstream probe、生产 Adapter 和 Provider activation 仍未实现”。

## 预计实现范围

- `server/external-pool-adapter-session-core/`
- `server/src/compute_federation/external_pool_adapter_ephemeral_secret_delivery.rs`
- `server/src/compute_federation/external_pool_adapter_linux_supervisor/`
- `server/src/store/compute_external_pool_adapter_runtime_bundle/`
- `server/src/external_pool_adapter_session_fixture_main.rs`
- `server/src/compute_federation/*source_contract_tests.rs`
- `docs/distributed-compute/`
- `AI_CURRENT.md`

## 依赖

- `compute-v256-runtime-bundle-resolver-verification`
- `compute-external-pool-adapter-authenticated-runtime-v1`

## 当前结果

尚未实现。当前主线只证明 V256 能在 retained handles 与 locked host memory 中解析 config/credential，以及 V262 能在真实 exec 后建立 authenticated runtime；两者尚未组合，不能宣称 Secret 已交付。
