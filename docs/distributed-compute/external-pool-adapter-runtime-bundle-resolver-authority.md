---
title: 外部矿池 Adapter operator-mounted runtime bundle resolver 权威
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter operator-mounted runtime bundle resolver 权威

## 1. 唯一语义：短时保管快照，不是 runtime readiness

V256 新增服务端私有、按需、易失的 operator-mounted runtime bundle resolver 源码。它把一份与 exact V255 launch profile、current V253 credential re-attestation 和历史 onboarding credential subject 绑定的 manifest、config 与 credential 文件，以 retained filesystem handles 和 locked sensitive memory 组成短时保管快照。成功效果只允许描述为 `resolved_ephemeral`；`runtime_launch_ready` 固定为 `false`。

这不是另一条 receipt、currentness projection 或惰性计划。V256 不新增数据库表、migration 或 no-op migration，不持久化 bundle generation、路径、manifest、config/credential bytes、content hash 或 filesystem identity，也不提供 HTTP、MCP、PC 或 GET secret-resolve 入口。进程崩溃或重启会丢弃 handles 与内存；下一次消费必须从头解析。

resolver 不激活 Provider，不创建 v213 service actor/authorization/route authority，不启动 entrypoint、Sidecar 或进程，不连接网络、矿池、IPC 或 resolver backend，不运行 probe，也不生成 Pool、Offer、Snapshot、Job、Reservation、Attempt、Start、usage 或 settlement。Provider 必须保持 exact `registering`；V254 的 18 个 temporary market absolute deny 不得改变。

## 2. 唯一寻址规则与 confused-deputy 边界

operator 在服务端外配置一个绝对、本地的 runtime bundle mount root。resolver 只从服务端已验证的 64 字符小写十六进制 SHA-256 `profile_digest` 派生固定相对位置：

```text
<operator_root>/v1/sha256/<profile_digest[0..2]>/<profile_digest>/
  manifest.jcs
  config.bin
  credential.bin
```

目录层级和三个 leaf 名称固定；profile ID、Provider ID、owner 输入、raw credential locator、环境变量片段、manifest 字段或符号链接都不能改变相对路径。

历史 onboarding 中 owner 提交的 raw `vault-ref` locator 只在 Store 私有 authority 内重算 scheme 与 commitment，并与 V249/V255/manifest 根逐字比较。它是 credential 的逻辑 subject，不是服务器签发的 custody ACL；因此绝不能作为路径、环境变量名或值、vault key、resolver key、日志字段或公开输出。把 raw locator 用于全局 mount 查找会形成跨租户 confused deputy，必须失败关闭。除 exact `vault_ref` 外的 scheme 不进入 V256。

## 3. strict manifest ABI

`manifest.jcs` 必须是无 BOM、无前后空白或换行的严格 UTF-8 JSON，并且原始 bytes 必须等于该对象的 canonical JCS bytes；重复 key、unknown field、类型漂移、非 canonical 表示或多余 bytes 均失败关闭。manifest 固定绑定：

- schema 与 purpose；
- `bundle_generation > 0`，仅作观察标签，不提供 anti-rollback 保证；
- V255 `profile_id/profile_digest/launch_policy_digest`；
- V254 `candidate_id/candidate_digest`；
- Provider binding、Provider owner、logical adapter、release version 的 exact roots；
- `adapter_config_revision/adapter_config_digest`；其中 digest 是既有 opaque authority，不能解释为 config content SHA-256；
- `credential_ref_scheme=vault_ref` 与 credential locator commitment；
- 由 Store 在同一 `checked_at` 自选的 current V253 receipt ID、receipt digest、material digest 与 `report_expires_at`；调用方不提交 receipt ID 或 digest；
- config/credential 的 exact byte length 与私有 SHA-256。

config 大小固定为 1..1,048,576 bytes，credential 大小固定为 1..65,536 bytes；二者按原始 bytes 读取，不 trim、不解码、不解析。content hash 只在私有 manifest 校验中使用，不落库、不进入响应或日志，避免把 secret hash 变成离线猜测 oracle。

V253 证明 credential 的 Provider-specific 逻辑 subject/commitment 在签名报告期限内 current；它没有观察 V256 的 `credential.bin`，因此不能证明 exact secret bytes、secret version 或 config bytes 就是 verifier 测试过的材料。V256 也不得从 content hash 相等推导这种结论。

## 4. 同时点 Store authority 与非原子现实

唯一消费入口是 `crate::store` 内的 sealed authority。内部 supervisor seam 提供 fresh V249 Prepared installation、exact profile identity 与 operator root；Store 自己开启 `BEGIN IMMEDIATE` SQLite transaction、在调用瞬间生成非公开的 canonical `checked_at`，并执行：

1. 重建 exact current V255 launch-profile authority；
2. 以同一 `checked_at` 内部选择 exact current V253 head，不允许 caller 报 receipt ID/digest；
3. 私下重建 historical onboarding `vault_ref` commitment；
4. 形成 expected manifest roots，再打开固定 bundle path；
5. 严格校验 manifest、文件 identity、大小与 content hash，并返回不可序列化、不可 clone 的短时 authority。

authority 不写数据库；V256 数据库 schema 仍是 V255。它不得接收调用方指定或历史回放的 `checked_at`、公开 currentness JSON或缓存过的 V253/V255 投影作为 authority。所有 DB roots 必须在一次消费中按 Store 生成的同一近当前 `checked_at` 重算。

SQLite snapshot 与 operator filesystem 不存在跨介质原子事务。retained handles、打开前后 identity/metadata/content 复验只能把本次读取绑定到已打开对象，不能把 DB 与 mount 变成全局原子快照。未来 supervisor 必须在短事务/短借用内消费，并在 launch 前重新检查 DB roots、TTL 和所持 handles；不能把一次 V256 结果缓存成长期 readiness。

## 5. filesystem 与内存保管

resolver 必须逐层无跟随打开目录与三个固定 leaf，拒绝非本地/设备路径、symlink 或 reparse point、非普通文件、多硬链接、owner/ACL/permission 不安全、identity 或 metadata 前后漂移、大小越界、短读/超读以及 hash 漂移。平台无法证明要求的 custody 属性时必须返回私有 fail-closed error，不能降级为普通 path open。V256 Linux 源码包含 dirfd/no-follow、exact mode/owner、无 xattr、single-link 与 identity 复验；Windows 源码虽包含 fixed local volume、reparse/inherit/link/identity 检查和 retained handle 结构，但 protected-DACL parser 尚未实现，`validate_protected_dacl` 固定失败，因此 Windows resolver 本批明确不可用。

config 与 credential 直接读入 OS page-backed locked memory；allocation、lock 或 dump-exclusion 任一步失败即整体失败。敏感 byte buffer 不实现 Clone/Serde，不通过 Debug 展示内容，也不转换为 Vec/String。manifest canonical 校验产生的 content-hash 字符串中间值也必须在比较后显式 zeroize。唯一消费方式是在 sealed authority 的短生命周期闭包中借用 `&[u8]`；闭包只能返回 `Result<()>`，不能通过泛型返回值带出 secret；Drop 顺序为显式覆写、unlock、release。retained handles 与 buffers 都不得跨 HTTP、MCP、日志、数据库或 crash recovery。

本地 Windows 测试目标已编译该 resolver，并用 test-only 文件动态验证 locked-memory 精确读取、短读/超读拒绝、manifest canonical/unknown/root/大小边界与 SHA-256 漂移；这些证据不代表已验证 Linux `openat/mlock/madvise`、Windows handle/DACL/`VirtualLock` 生产行为，也不能抵御已取得服务进程权限、内核权限、调试器、CPU register、swap/dump 配置错误或物理攻击。

## 6. readiness 的下一道门

V256 只把生产 bundle 的 server-only custody/resolution seam 推进到 `implementation_partially_verified`。完整 `elon-server` 测试目标已编译，14 项定向测试通过；其中 8 项执行 manifest、摘要和 locked-memory 行为，6 项锁定私有边界、Store composition 与 V254 绝对市场门。证据指纹为 `bf17a791bd94e135404950399a2ba0e7322ce236b23b980c9295fa9fb862acfc`。未来 atomic activation 至少还要：

- 在 authenticated supervisor 内以 fresh handles 和短借用启动隔离 runtime；
- 用 no-work authenticated probe 或新签名 generation ABI，把实际消费的 exact credential/config generation 与 Provider/release/profile 绑定；
- 同次重验 V249/V250/V252/V253/V254/V255、runtime identity、route collision、ACK 和短 TTL；
- 原子创建 Provider actor/route/runtime authority，并用完整 readiness/currentness admission gate 替换 V254 absolute deny；
- 接通真实任务、验证计量和结算的独立证据链。

在这些条件完成前，任何 bundle resolve 成功、manifest current、memory locked 或 source review 都不能表述为 activation、route、runtime、market 或 production readiness。
