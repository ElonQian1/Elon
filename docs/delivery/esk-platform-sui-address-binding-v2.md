---
title: "ESK 平台认证 Sui 地址绑定 V2 交付证据"
version_status: current
reviewed_at: 2026-09-06
owners: [platform-assets, protocol]
---

# ESK 平台认证 Sui 地址绑定 V2 交付证据

本记录说明平台认证、离线验签和私有追加式地址绑定的实现与验证。它不表示 ESK 已在
Sui 发布、任何地址已经获得 ESK、平台余额已经迁移、真实用户已经完成钱包签名，也不
表示发生了 USDT、币安、量化或其他资金操作。正式范围见
[需求文档](../requirements/esk-platform-sui-address-binding-v2.md)。

## 状态矩阵

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| 需求与实现 | implemented | 真实平台会话、V1 挑战、三种单签本地复核、一次消费和本人查询已组合 |
| 定向验证 | passed | 最终生产源码快照上的 harness 17/17、服务目标 14/14、旧 V1 58/58 通过 |
| Feature Registry | in_progress | 证据与最终状态在提交前统一登记 |
| Git 推送 | pending | 本文件写入时尚未推送，不能把工作区实现冒充主线交付 |
| 后端部署 | pending | 本文件写入时尚未发布服务器 |
| 真实用户验收 | not_performed | 未使用真实钱包、真实地址、生产会话或真实用户数据 |
| Sui 发币/链终局 | not_performed | 无 RPC、交易构建、签名、广播、包 ID、对象 ID 或 checkpoint 证据 |

## 已实现能力

- 新增三个本人接口：创建短时 testnet 挑战、完成挑战、读取本人地址绑定。唯一 Bearer
  会话在读取路径、查询或正文前验证；管理员静态 token、`local-owner`、停用用户、
  过期或撤销会话均失败关闭。
- 复用 V1 固定消息，服务端生成 subject commitment、nonce、时间和 challenge ID；
  客户端只能提交规范地址、TTL 以及钱包返回的四个精确字段。
- 本地重建 Sui personal-message intent 和 BCS byte-vector，执行 Blake2b-256，并验证
  ED25519、Secp256k1、Secp256r1。错误 flag、长度、公钥、地址、消息、签名、high-S
  ECDSA 与非规范 Base64 均拒绝。
- 同一 SQLite 写事务重新核对会话、挑战时间、用户、地址和未消费状态。完全相同响应
  可返回同一绑定；篡改重放、跨用户、并发第二记录、用户换地址和地址换用户均拒绝。
- 同一用户和地址的未过期挑战被复用；每用户最多 3 个同时有效挑战和滚动 24 小时
  20 个新挑战，门禁在数据库写事务内执行。
- 私有 subject、challenge、binding 三表均为 `WITHOUT ROWID`，并以 `BEFORE INSERT`
  守卫覆盖全部主键和唯一键冲突，同时禁止 UPDATE 和 DELETE。完整 wallet response
  只进入私有账本；公共响应只投影规范地址、方案、时间、回执摘要和七个真实性标志。
- 所有成功与错误响应均带 no-store/no-cache/no-referrer。成功绑定仍固定
  `chain_finality_verified=false`、`asset_identity_verified=false`、
  `balance_eligible=false`、`manifest_transition_allowed=false`，并保持正式 ESK 余额及
  `platform_recorded/not_deployed` 投影不变。

## 已执行验证

- Rust harness：17/17 通过，覆盖密码学向量、挑战重构、认证、过期/未来时间、撤销、
  幂等、篡改、用户/地址唯一性、限流、并发消费、迁移重入、余额不变和追加式门禁。
  重构到 `origin/main` 后的最终验证指纹
  `ec19e95d2849cd0fab270a44dd7a6fbff3732fb7f78fceda094bed03028417e2`，结果为
  `17 passed / 0 failed / 142 filtered`，
  回执 `6bd4a7ee0f2ee2b0636033a493aeb3eaaeb31d65ec5b57eb3d54685ab51ce90a`。
- 正式 `elon-server` 目标：14/14 通过，包含五项进程内 HTTP 合同与九项密码学/Store
  单元验证。重构后的最终验证指纹
  `3349125b113fac8466aceeabe6c23deac2f29c88cdb56766bf975f3010b32bd7`，结果为
  `14 passed / 0 failed / 2420 filtered`，
  回执 `308aa611ed7fe26671cad48a4929c1be094edd6029926317a503cd2aef3cdc72`。
- 两份 Rust 回执均绑定 PowerShell 5.1 权威验证入口计算的最终源码快照
  `a96e99e0151b983b6a169e85e4a43f2d36f1afd3e24a1caa7ca76706076d36a8`，
  使用 locked/offline Cargo 路径。重构后重新执行的旧 V1 Node 合同 58/58 通过，并明确
  输出网络请求、钱包或私钥读取均为 none，真实用户验收为 not performed。
- TDD 先复现两类真实缺陷：普通 `INSERT OR REPLACE` 覆盖失败指纹
  `9dc5f5892644dcf95ddf705eda8ca25030641c0ed51001ab2e47771c98858d5a`；显式隐藏
  `rowid` 替换失败指纹
  `c5acee1b98c41c52f8963a74bf04d3b87246515bddd2fc71d5f0ff9932d8024f`。
  最终测试实际发起隐藏 rowid 攻击、确认三表不暴露 rowid，并从 `sqlite_master` 逐项
  锁定 subjects 2 个、challenge 1 个、bindings 4 个主键/唯一键碰撞谓词。
- 三轮独立只读审查分别覆盖密码学、HTTP/隐私和 SQLite 追加式语义。数据库审查发现并
  推动修复上述两类替换旁路；修复后复核没有 P0-P3 遗留问题，合法全新键追加仍允许。

最初对整个 server 的未限定测试命令会编译与本功能无关的其他二进制测试目标，并被
既有 `node_agent_compute_plugin_host` 编译问题阻断；本功能没有修改那些文件。随后按
生产二进制 `--bin elon-server` 精确限定，正式组合目标通过。此背景问题不得写成 V2
失败，也不得把定向通过扩大成全仓库全部目标通过。

## 安全与后续边界

本批没有读取钱包配置、私钥、助记词、keystore、剪贴板或生产秘密，没有实例化 Sui
RPC client，没有构建、签名或广播交易，也没有读取或改写付款、ESK 余额、卖回、量化、
USDT 或 Binance 数据。私有 Bearer 接口仍要求安全传输，本批不通过放宽明文传输门禁
来换取可用性。

下一批 `esk-sui-testnet-publication-v1` 必须先冻结正式参数、职责地址和无签名发布计划，
再在获得网络、钱包、gas 与明确授权后执行受控 testnet 发布。样例供应、六桶比例、地址
或日期都不能自动晋级正式参数。只有 package、Currency、六桶/锁仓、MetadataCap、
UpgradeCap、源码对应性和 checkpoint 终局证据完整后，才能讨论平台余额迁移；迁移还需
独立的幂等领取与反向结转防双计设计。
