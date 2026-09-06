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
   大小与 SHA-256；归档中作为安装候选的 `sui.exe` 必须恰好一个且位于根目录
   `./sui.exe`，其他随官方归档发布的工具不解包、不安装；解包后再校验 fixture 已绑定的
   `sui.exe` 大小与 SHA-256。任一不符立即失败。
3. Framework/MoveStdlib 源码输入固定为官方 codeload 归档
   `https://codeload.github.com/MystenLabs/sui/tar.gz/46f18562f1f5af2438d35828e8b62d5e0b972db7`：
   大小必须为 `87498700` 字节，SHA-256 必须为
   `9046fce263794cca6772c59aacd328706d42522dd7408c0dcb4a7f2613016afd`，唯一归档根必须为
   `sui-46f18562f1f5af2438d35828e8b62d5e0b972db7`。只允许
   `crates/sui-framework/packages/move-stdlib` 与
   `crates/sui-framework/packages/sui-framework` 下精确 187 个文件；相对路径统一为 `/`
   并按 Ordinal 排序，以 UTF-8 路径、NUL、原始文件字节、NUL 逐项绑定后的摘要必须为
   `sha256:017e6a38b5d976c87b710e02b39d26988691d101bf914a42e6509c82d62e027b`。
4. 仓库中的两个 `Move.toml`、`Move.lock` 和全部 Move 源码/测试仍是受管输入，不得被
   验证器改写。CI 不缓存、不检出也不执行 live `.git` 依赖仓库，不追踪浮动分支；
   testnet 工具链或证据不得复用于 mainnet。
5. 创世 manifest 与六桶 allocation policy 是当前本地验证证据真源；synthetic 总量、
   比例、日期和 holder 仍不是正式发行参数。

## 安装与缓存合同

1. `scripts/install-esk-sui-toolchain.ps1` 只安装到调用方传入的明确目录。命中缓存时仍
   重算 CLI 摘要并核对版本；缓存污染不得静默复用。
2. 下载只允许固定的官方 GitHub HTTPS URL，最多跟随 3 次官方 GitHub release asset
   HTTPS 重定向，不接受镜像、环境变量覆盖 URL 或“latest”标签。重定向中的短期签名
   参数不作为信任依据，最终归档必须通过固定 SHA-256 后才解包并原子提升为可用工具链。
3. Framework 源码只允许上述精确 codeload URL，不能通过 Git remote、clone、fetch、
   submodule、环境变量或调用方 URL 取得。归档在解包前校验大小和 SHA-256；解包后再次
   校验唯一根、187 个允许文件及规范内容摘要。缓存对象只能是固定归档或由其验证得到的
   只读源码，不得是含 `.git` 的活动仓库。
4. 固定工具链缓存键必须同时包含 release、平台、CLI 归档 SHA-256 和 Framework 源码
   归档 SHA-256，且不得使用部分 restore key。缓存目录只保存固定 CLI 与源码归档，
   不得缓存钱包、keystore、client 配置、RPC 响应、临时 `Move.lock`、live `.git` 或仓库
   构建产物；任何缓存命中仍须重新验证其绑定摘要。
5. 安装脚本的离线合同测试使用合成归档和合成可执行文件，不联网，也不冒充真实 Sui
   工具链验证。

## Move 验证合同

1. `scripts/validate-esk-sui-move.ps1` 只接受显式 CLI 路径，先核对版本和二进制摘要，
   再运行现有 Node 合同验证与两个 Move 包的 build/test。
2. 验证器把两个包复制到一次性工作目录，把副本的 Sui/MoveStdlib 依赖改为已验证源码
   归档中的本地路径，并删除副本中临时生成或继承的 `Move.lock` 后再构建。验证结束删除
   整个一次性工作目录；不得执行或向缓存写入 live `.git`，不得改写仓库内 lockfile。
3. build/test 均启用 `--warnings-are-errors`，test 固定单线程。必须得到 `esk_currency` 3/3、
   `yilong_participation` 13/13，任何失败、警告、测试数变化或意外输出均失败关闭。
4. 货币核心 `esk.mv` SHA-256 必须匹配 genesis manifest；参与包按既有
   `production_bytecode_bundle_v1` 规则复算全部生产模块并匹配 allocation policy。生产
   模块必须在 build 后、test 前复制到隔离目录，防止 test-mode 字节码覆盖证据。
5. 测试输出移除 ANSI、规范为 UTF-8/LF，只允许固定构建/测试行及 Sui 1.79 的一条精确
   依赖提示；提示不进入回执，任何其他输出失败关闭。回执与受管 evidence 逐字节及摘要
   对比；该摘要是 `canonical_test_receipt_sha256_v1`，不是原始 stdout 摘要。验证完成后
   受管源码、lockfile、fixture 和 evidence 不得被改写。
6. 子进程使用显式空 keystore、`envs: []` client 配置和独立 `MOVE_HOME`。每次命令都
   显式传入 client 配置与 `--build-env testnet`；验证入口不得执行 `sui client`、生成
   key、读取用户配置、配置或查询 Sui RPC、构建发布交易、签名或广播。冷缓存网络只
   允许固定 GitHub release asset 及上述 codeload 源码归档，不得把这种源码下载写成
   “全程无网络”，也不得把 `--build-env testnet` 冒充链查询或测试网发布。

## CI 门禁

在 `.github/workflows/ci.yml` 增加固定 `windows-2025` 的独立 `sui-move` job：检出项目
源码、恢复固定 CLI/官方源码归档缓存、执行安装/完整验证，并在失败时上传不含秘密的诊断输出。不得
缓存或执行 Sui 源码 live `.git`。CI 输出必须明确区分：

- `toolchain_verified`：官方 archive、CLI 摘要和版本匹配；
- `move_verified`：两个包 build/test 与字节码/测试证据匹配；
- `publication_state=not_performed`：没有任何链上动作。

## 验收标准

1. 合成可执行文件的离线测试覆盖正确缓存和全新安装路径；损坏归档、损坏缓存、错误
   版本、非根目录和多个候选 `sui.exe` 均失败关闭。官方归档另以固定双摘要实测安装。
2. 官方源码归档的错误大小、摘要、根目录、越界条目、文件集合或内容摘要均失败关闭；
   验证与 CI 不创建、缓存或执行 live `.git` 依赖仓库。
3. 两个 Move 包在固定工具链及归档内本地依赖下以 warnings-as-errors 完成 build/test，测试数精确为
   3/3 与 13/13，生产字节码摘要匹配现有证据。
4. 修改 Move 源码、依赖提交、fixture 摘要、受管测试 evidence 或 CLI 摘要都会让门禁
   失败，不会自动刷新“正确答案”。
5. 合同测试证明安装器只接受官方固定 URL、所有写入留在明确根目录，且验证入口没有
   钱包、RPC、签名、广播和资金操作命令。
6. CI 配置由守卫测试固定 job、缓存键、安装入口、完整验证入口与只读权限；现有 Rust、
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
