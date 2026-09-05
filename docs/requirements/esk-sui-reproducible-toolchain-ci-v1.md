---
title: "ESK Sui 可复现工具链与 CI V1"
status: accepted
implementation_status: in_progress
owner: protocol, ci
priority: p0
reviewed_at: 2026-09-06
decision_refs:
  - "docs/decisions/esk-sui-economic-foundation-v1.md"
  - "docs/requirements/esk-sui-genesis-foundation-v1.md"
  - "docs/requirements/esk-sui-allocation-vesting-v1.md"
---

# ESK Sui 可复现工具链与 CI V1

## 用户结果

ESK 的两个 Sui Move 包不再只依赖某台开发电脑曾经成功的验证记录。仓库提供固定版本
工具链安装、摘要校验和单一验证入口；每次主线或拉取请求变更都由 CI 用相同工具链
重新编译并运行货币核心 3 项、参与/分配包 13 项测试，同时复算生产字节码证据。

通过本功能表示“当前源码可以由固定工具链重复构建且合同测试仍通过”。它不表示 ESK
已经发布，不创建钱包、地址、交易、签名、package/object ID 或 checkpoint，不连接
Sui RPC，也不移动 SUI、ESK、USDT 或任何资金。

## 固定输入

1. 网络工具链固定为 Sui `testnet-v1.79.0`，CLI 版本必须精确为
   `sui 1.79.0-46f18562f1f5`，Sui Framework 依赖提交必须精确为
   `46f18562f1f5af2438d35828e8b62d5e0b972db7`。
2. Windows x86_64 官方 release archive 固定为
   `sui-testnet-v1.79.0-windows-x86_64.tgz`；安装前校验 GitHub Release 公布的 archive
   SHA-256，解包后再校验 fixture 已绑定的 `sui.exe` SHA-256。任一不符立即失败。
3. 两个 `Move.toml` 和 `Move.lock` 必须继续固定同一 Framework 提交；禁止 CI 自动更新
   lockfile、追踪浮动分支或把 testnet 证据复用于 mainnet。
4. 创世 manifest 与六桶 allocation policy 是当前本地验证证据真源；synthetic 总量、
   比例、日期和 holder 仍不是正式发行参数。

## 安装与缓存合同

1. `scripts/install-esk-sui-toolchain.ps1` 只安装到调用方传入的明确目录。命中缓存时仍
   重算 CLI 摘要并核对版本；缓存污染不得静默复用。
2. 下载只允许固定的官方 HTTPS URL，不接受镜像、重定向后参数、环境变量覆盖 URL 或
   “latest”标签。归档写入隔离临时目录，摘要通过后才解包并原子提升为可用工具链。
3. CI 缓存键必须包含 release、平台和官方 archive SHA-256；不得缓存钱包、keystore、
   client 配置、RPC 响应或仓库构建产物。
4. 安装脚本的离线合同测试使用合成归档和合成可执行文件，不联网，也不冒充真实 Sui
   工具链验证。

## Move 验证合同

1. `scripts/validate-esk-sui-move.ps1` 只接受显式 CLI 路径，先核对版本和二进制摘要，
   再运行现有 Node 合同验证与两个 Move 包的 build/test。
2. build/test 均启用 `--warnings-are-errors`。必须得到 `esk_currency` 3/3、
   `yilong_participation` 13/13，任何失败、警告、测试数变化或意外输出均失败关闭。
3. 货币核心 `esk.mv` SHA-256 必须匹配 genesis manifest；参与包按既有
   `production_bytecode_bundle_v1` 规则复算全部生产模块并匹配 allocation policy。
4. 测试输出按规范 UTF-8/LF 重算并与受管 evidence 对比；验证完成后受管源码、lockfile、
   fixture 和 evidence 不得被改写。
5. 子进程使用隔离的空 Sui 配置目录。验证入口不得执行 `sui client`、生成 key、读取
   用户配置、查询 RPC、构建发布交易、签名或广播。

## CI 门禁

在 `.github/workflows/ci.yml` 增加独立 Windows `sui-move` job：检出源码、恢复固定工具链
缓存、执行安装/完整验证，并在失败时上传不含秘密的诊断输出。CI 输出必须明确区分：

- `toolchain_verified`：官方 archive、CLI 摘要和版本匹配；
- `move_verified`：两个包 build/test 与字节码/测试证据匹配；
- `publication_state=not_performed`：没有任何链上动作。

## 验收标准

1. 正确缓存和全新安装路径都通过双摘要与精确版本校验；损坏归档、损坏缓存、错误版本
   和多个候选 `sui.exe` 均失败关闭。
2. 两个 Move 包在固定工具链下以 warnings-as-errors 完成 build/test，测试数精确为
   3/3 与 13/13，生产字节码摘要匹配现有证据。
3. 修改 Move 源码、依赖提交、fixture 摘要、受管测试 evidence 或 CLI 摘要都会让门禁
   失败，不会自动刷新“正确答案”。
4. 合同测试证明安装器只接受官方固定 URL、所有写入留在明确根目录，且验证入口没有
   钱包、RPC、签名、广播和资金操作命令。
5. CI 配置由守卫测试固定 job、缓存键、安装入口、完整验证入口与只读权限；现有 Rust、
   PC 前端和 Android job 不受影响。

## 明确不在本功能内

- 正式总供应、六桶比例、锁仓日期、真实职责地址和多签阈值的批准；
- unsigned PTB、testnet faucet、gas、钱包创建/导入、签名、广播和发布；
- Currency 最终注册、真实逐桶分配、观察器正向链证据或 Manifest V2；
- mainnet 工具链、审计、密钥仪式、真实用户迁移和生产入口开放。

上述内容分别属于后续 `esk-sui-testnet-publication-v1`、
`esk-paid-user-migration-v1` 与 `esk-mainnet-launch-v1`，必须单独登记和授权。

## 回滚

移除独立 CI job 和本功能脚本即可回滚自动门禁，不修改 Move 合约或任何链状态。不得以
回滚 CI 为理由删除历史本地证据，也不得降低后续测试网/主网发布的证据要求。
