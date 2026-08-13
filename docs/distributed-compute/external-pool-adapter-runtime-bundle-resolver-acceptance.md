---
title: 外部矿池 Adapter runtime bundle resolver 验收边界
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
verification_status: local_target_verified
---

# 外部矿池 Adapter runtime bundle resolver 验收边界

## 本批状态

V256 的 server-only ephemeral operator-mounted resolver 与 Store-private composition seam 已进入完整 `elon-server` 测试目标。定向验证实际运行 14 项测试，`14 passed / 0 failed`：6 项既有 source-contract，3 项 strict manifest 行为、3 项摘要/路径格式行为、2 项 test-only 文件的 locked-memory 精确读取与短读/超读拒绝。验证指纹为 `fd22ac143bc522669a3b224a4375fdc40e748d26d69bbb3330d1b408f3860618`。

Windows 源码已经编译，错误的 `DRIVE_FIXED` 模块导入已修复；protected-DACL parser 尚未实现，当前实现仍固定返回 `UnsafeCustody`，所以 Windows resolver 明确 fail-closed/unavailable。Linux 分支在本次 Windows 目标上未编译或运行；不能把任一平台的 handle/identity 骨架表述成生产 bundle 已解析。

数据库 schema 保持 V255：无 `migration_v256`、无 receipt、无 mutable head、无 secret metadata persistence。没有 HTTP/MCP/PC route，GET 不读取 secret。Provider 保持 `registering`、`runtime_launch_ready=false`；V254 18 个 temporary absolute deny 原样保留。

## 已执行定向断言

静态测试源码必须锁定：

- operator path 只由 exact 小写十六进制 `profile_digest` 按 `v1/sha256/<prefix>/<digest>` 派生，leaf 仅 `manifest.jcs/config.bin/credential.bin`；
- raw owner `vault-ref` locator 只用于 private commitment comparison，不进入 path、env、key、响应、日志或持久化；
- strict manifest 拒绝 unknown/non-canonical/非 UTF-8/多余 bytes，并绑定 V255、V254、Provider/release/config 与 current V253 receipt/material/expiry roots；
- Store 自己开启 `BEGIN IMMEDIATE` transaction，并以 Store 生成的同一近当前 `checked_at` 自选 current V253 head、消费 current V255；caller 不能提交 checked_at、V253 receipt ID/digest；
- `adapter_config_digest` 保持 opaque，config/credential content SHA 只属私有 manifest；`bundle_generation` 不提供 anti-rollback；
- config 1..1,048,576 bytes、credential 1..65,536 bytes，原始 bytes 不 trim/parse；
- no-follow/reparse、local-path、regular-file、single-link、owner/ACL/permission、identity/metadata before-after、size/hash drift 任一不能证明即失败关闭；
- sensitive bytes 只进入 locked、dump-excluded、zeroize-on-drop memory，经只能返回 `Result<()>` 的短生命周期 closure 借用；类型无 Clone/Serde/content Debug，manifest canonical/content-hash 中间字符串比较后也 zeroize；
- resolver 与 Store seam 无 SQL write、migration、router/service/MCP、Command/process、network、Provider activation、route、market、usage 或 settlement symbol；
- V254 market fence 名称/body 总数仍为 18，规范化 SHA-256 仍是 `7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6`。

这些断言即使未来执行通过，也只证明源码和选定 OS fixture 的合同，不证明生产 mount、ACL、内存锁、secret generation 或 runtime readiness。

## 待运行正向矩阵

- Linux 本地只读 mount：逐层 dirfd/no-follow 打开、exact permission/owner、single-link leaf、前后 dev/inode/metadata 稳定；
- Windows 本地 volume：非 reparse handle、受保护 DACL、允许主体最小化、file identity/metadata 稳定；
- canonical manifest、边界大小 config/credential、direct locked-memory read、hash 匹配与 Drop wipe；
- Store-owned `BEGIN IMMEDIATE` SQLite transaction/same near-now `checked_at` 的 current V255 + internally selected current V253 head + historical commitment exact composition；
- crash/restart 后旧 authority 不可恢复，fresh resolve 重新打开所有对象。

## 待运行失败关闭矩阵

- 相对/UNC/device/remote root，profile digest 非 exact hex，目录或 leaf symlink/reparse/hardlink，owner/ACL/mode 不安全；
- manifest BOM、换行、unknown/duplicate key、non-JCS、generation 0、任何 V253/V254/V255/Provider/release/config root 漂移；
- config/credential 空、超限、短读、identity/metadata/hash 漂移；
- current V253 缺失、撤销、过期或 material 漂移，V255/profile/policy 不 current，historical `vault_ref` commitment 不一致，伪造或历史回放 `checked_at`；
- OS allocation/lock/dump-exclusion 失败，读取中替换文件，SQLite transaction 回滚或进程崩溃；
- 任一失败均不留下 DB row、公开错误中的 secret/path/hash、进程、route、Provider 状态或经济副作用。

## 仍未验收

未验收 Linux 编译与 filesystem syscall，Windows protected DACL/真实 bundle path/`VirtualLock`，Drop 后内存取证式 zeroization，SQLite upgrade/reopen/concurrency/crash、生产 mount/secret、HTTP/TCP、Sidecar/transport、authenticated no-work probe、runtime identity、Provider activation、actor/route、Pool/Offer/Job/Attempt/Start、usage、verification或 settlement。

因此本批记录为 `implementation_partially_verified / local_target_verified / passed=14`。SQLite 与 filesystem 非原子；V253 只认证逻辑 credential subject，不证明 `credential.bin` exact bytes。后续不得因 V256 局部测试通过或一次 resolve 成功而删除 V254 absolute deny 或宣称 production readiness。
