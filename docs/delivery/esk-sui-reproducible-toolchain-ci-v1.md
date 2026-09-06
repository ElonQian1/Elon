---
title: "ESK Sui 可复现工具链与 CI V1 交付证据"
version_status: current
reviewed_at: 2026-09-06
owners: [protocol, ci, platform-assets]
---

# ESK Sui 可复现工具链与 CI V1 交付证据

本记录只说明固定工具链、源码归档、Move build/test 和 CI 门禁的实现与本地验证。
它不批准 ESK 经济参数、职责地址、钱包、签名、发布或资金操作。正式范围见
[需求文档](../requirements/esk-sui-reproducible-toolchain-ci-v1.md)。

## 状态矩阵

| 状态轴 | 当前值 | 边界 |
| --- | --- | --- |
| implementation_status | implemented_in_worktree | 固定双归档安装器、隔离本地依赖验证器、CI job 与失败关闭合同已在当前工作树实现 |
| local_verification_status | passed | 官方源码 tar → 本地依赖的最终路径已真实得到 3/3、13/13、187 文件及两组纠正生产摘要 |
| remote_ci_status | not_performed | 尚无绑定当前提交的 GitHub Actions run，不能把本地通过写成远端 CI 通过 |
| push_status | pending | 当前改动尚未推送；本记录不填写未来提交 SHA |
| publication_status | not_performed | 未执行 Sui testnet/mainnet 发布，所有 package/object/transaction/checkpoint 证据仍为空 |

Feature Registry 的当前功能在本记录生成时仍处于实现收尾阶段；涉及旧功能的需求与
证据摘要必须通过正式 registry workflow 显式重绑、全量记录并复核，不能手改旧哈希或
在漂移仍存在时宣称 `released`。最终 registry 状态和 drift 结果以收尾时的只读检查为准。

## 固定供应链输入

### Sui CLI

- release：`testnet-v1.79.0`；版本：`sui 1.79.0-46f18562f1f5`；平台：Windows x86_64。
- 官方归档：`sui-testnet-v1.79.0-windows-x86_64.tgz`，大小 `273621059` 字节，
  SHA-256 `9d8442bad8fd516116a76ff52213cdfd43b98857c3f7fbbde124ed5e2041a558`。
- 解包后的 `sui.exe` 大小 `138727424` 字节，SHA-256
  `d9b7ff7b4bb3cbbf3f327ddf5998b388773956ce30c897798b56a6c0db9fee7f`。
- GitHub release asset 只允许受限的官方 HTTPS 重定向；短期 URL 参数不作为信任
  依据，最终仍由固定大小和摘要裁决。

### Sui Framework 与 MoveStdlib

- 固定提交：`46f18562f1f5af2438d35828e8b62d5e0b972db7`。
- 唯一下载 URL：
  `https://codeload.github.com/MystenLabs/sui/tar.gz/46f18562f1f5af2438d35828e8b62d5e0b972db7`。
- 归档大小 `87498700` 字节，SHA-256
  `9046fce263794cca6772c59aacd328706d42522dd7408c0dcb4a7f2613016afd`。
- 唯一归档根：`sui-46f18562f1f5af2438d35828e8b62d5e0b972db7`。
- 只接受 `crates/sui-framework/packages/move-stdlib` 与
  `crates/sui-framework/packages/sui-framework` 下精确 187 个文件；相对路径统一为 `/`
  并按 Ordinal 排序，以 UTF-8 路径、NUL、原始文件字节、NUL 逐项复算的集合摘要为
  `sha256:017e6a38b5d976c87b710e02b39d26988691d101bf914a42e6509c82d62e027b`。

CI 不缓存、检出或执行 live `.git` 依赖仓库。冷缓存网络只允许上述固定 codeload URL
及固定 GitHub release asset；归档命中缓存后仍重算摘要和内容集合，不信任缓存来源名。

## 隔离构建与回执

验证器把两个仓库包复制到一次性目录，并把副本中的 Sui/MoveStdlib 依赖指向已验证
归档内的本地目录。副本里的临时 `Move.lock` 在构建前删除，构建产生的 lockfile、
字节码和日志均留在一次性根；验证结束删除该根。仓库中的 `Move.toml`、`Move.lock`、
Move 源码、测试、fixture 和受管 evidence 必须保持逐字节不变。

两个包均以 `--warnings-are-errors` build/test，测试固定单线程。每个包在 `move build`
成功后立即把精确生产模块复制到独立工件目录，再执行 `move test`；因此测试模式重建
不能覆盖随后参与摘要计算的生产字节码。

测试证据类型是 `canonical_test_receipt_sha256_v1`，不是 raw stdout。运行输出先去除
ANSI、规范为 UTF-8/LF，只接受固定依赖、构建、测试名称和最终计数行；Sui 1.79 对显式
依赖的一条精确提示被明确排除，其他未知输出全部失败关闭。当前回执为：

- `esk_currency` 3/3：
  `sha256:e1f934234dd2b6d9236d8e46a1430c732836962787067234368dc1a84212244a`；
- `yilong_participation` 13/13：
  `sha256:2a404f6feeda16125e79a35063285e268919c94b4dfbf0817ec230ac741f4d9e`。

## 旧字节码证据纠正

2026-09-06 的复核确认，旧流程在 `move test` 之后从同一 build 目录取样。虽然只选择了
生产模块名，文件本身已被测试模式重新生成，并可能包含 `#[test_only]` 入口，因而不能
作为将发布生产包的字节码证明。本轮没有改变 Move 源码、供应、六桶语义或测试结果；
只纠正取样时点和 fixture/文档中的证据摘要。

| 包 | 旧 test-mode 摘要 | build 后、test 前冻结的生产摘要 |
| --- | --- | --- |
| `esk_currency/esk.mv` | `sha256:b1881cd12ebf1fae560f71f668cd768a2c8838f2e0298eeb2e95e273bfd97472` | `sha256:314273ecd53a54793c8b70f35e4a1e853fdc7c6751c20dc0baf0628907b03ca7` |
| `yilong_participation` 两模块 bundle | `sha256:dded066357305b827bdc11cb6765c4def72f7ed21fa7a47ac4e493c79be1986f` | `sha256:fa691e2e7d7c1c347b8fd88a2dc9f3ca2590ee56813c0bb313ef2ea8d477d3ef` |

旧摘要仅用于解释历史错误，不得再用于发布计划、fixture 晋级或源码对应性证明。

## 网络、账号与链状态边界

运行时使用空 keystore、`envs: []`、无 active environment/address 的显式 client 配置。
`--build-env testnet` 只选择构建环境；没有配置或查询 Sui RPC，没有链端点读取。允许的
冷缓存 HTTPS 下载不等于 Sui RPC 或链查询，因此本批不宣称“全程无网络”。

本批没有读取默认 Sui 配置、真实钱包、私钥、助记词或生产凭据，没有创建地址、PTB
或交易，没有签名、广播、领取 faucet、移动 SUI/ESK/USDT，也没有生成 package ID、
object ID、transaction digest 或 checkpoint。manifest 与 allocation policy 的最高
事实仍是 `local_verified`，不能写成 `testnet_published` 或 `mainnet_published`。

## 远端与交接

固定 `windows-2025` 的独立 `sui-move` job 已写入工作树，但必须在推送后以当前提交对应的 GitHub
Actions 运行结果另行验收。只有实际 run 完成后，才可记录 run URL、commit SHA、缓存
命中情况及成功/失败结论；当前 `remote_ci_status=not_performed`。

当前 `push_status=pending`。推送、Feature Registry 证据刷新和统一 finish 回执均由本批
主任务后续完成；本文件不预写提交、远端 CI 或发布成功。即使以后 CI 与推送成功，也
只证明仓库源码可复现 build/test，不授权或证明 ESK 已在任何网络发布。

## 当前本地验证回执

2026-09-06 已完成以下当前工作树验证：

- 用两份固定官方归档完成双工件原子安装，再次命中缓存时同时复核 CLI、Framework
  archive 的大小、SHA-256 与 CLI 精确版本；
- 以本地固定 CLI 和缓存内官方 Framework archive 运行最终验证器：Framework 精确
  187 个文件，`esk_currency` 3/3、`yilong_participation` 13/13，生产字节码与当前
  manifest/policy 摘要完全匹配；
- 冷缓存 codeload 下载路径真实通过；即使响应没有 `Content-Length`，仍由
  `expected + 1` 流式上限、最终精确字节数和固定 SHA-256 失败关闭；
- 合成双归档测试覆盖 fresh/cache、CLI 与 Framework 同长度字节翻转、长度错误、错误
  CLI 版本、嵌套及多个 `sui.exe`；
- 保存的证据仅含严格白名单行，敏感形态扫描无命中；隔离 `MOVE_HOME` 只留下零字节
  锁文件，没有活动 Git checkout；最终状态明确为
  `ESK_SUI_RPC_STATE=not_configured_or_queried` 与
  `ESK_SUI_PUBLICATION_STATE=not_performed`。

这些是本机运行证据，不等于 GitHub Actions 远端运行；后者仍保持
`remote_ci_status=not_performed`，直至存在绑定已推送提交的真实 run。
