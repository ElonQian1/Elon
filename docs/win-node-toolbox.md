# Win 节点轻量工具箱设计

最后更新：2026-07-05

## 目标

Win 端不做“大而全开发套装”。默认安装保持轻量，只保证用户能启动节点和 Codex；AI 真正执行任务时，再把已经存在的小工具临时暴露给 Codex CLI。

## 核心原则

1. 默认不吓人：不预装一堆用户看不懂的大工具。
2. 不污染系统：优先用进程级 `PATH`，少写用户 `PATH`，不写系统 `PATH`。
3. 绿色优先：平台提供的工具放在 `%LOCALAPPDATA%\ElonNode\tools\<tool>\...`。
4. 小核心可自动修复：只有体积小、收益高、风险低的工具允许在“安装/修复”流程里自动安装。
5. 可选工具只探测：`fd`、`jq`、`7z` 等先只在已安装时暴露，不主动安装。
6. 可审计：每个被注入给 Codex 的工具都有对应 `ELON_CODEX_*_PATH` 诊断环境变量。

## 工具分层

| 层级 | 含义 | 当前策略 | 示例 |
|---|---|---|---|
| core | 高频、小体积、显著提升 Codex 效率 | 安装/修复流程可自动补齐 | `rg` |
| profile | 按项目类型有用，但不是所有用户都需要 | 已安装则临时暴露，未来可由用户选择安装 | `fd`、`jq` |
| optional | 低频或可能让用户感到“装太多” | 不自动安装，只在已存在时使用 | `7z` |

## 运行时流程

```text
node-agent 启动 Codex 任务
  -> 读取工具 catalog
  -> 探测系统 PATH、ElonNode tools、Codex runtime、常见包管理器目录
  -> 只收集“已经存在”的工具
  -> 给 Codex 子进程临时 prepend PATH
  -> 设置 ELON_CODEX_<TOOL>_PATH 用于诊断
```

这意味着用户机器上已经有 `jq` 或 `7z` 时，Codex 可以用；没有时不会自动下载，也不会弹出吓人的安装过程。

`/api/status` 会暴露 `codex_toolbox` 诊断对象，PC 工作台可以显示每个工具是否找到、来源、版本、是否会注入 Codex 子进程 PATH，以及缺失时的修复动作。

## 安装/修复流程

`scripts/setup-node-env.ps1` 当前只自动修复 core 小工具：

- `rg`：优先发现现有安装；如果配置了 `ELON_RIPGREP_ZIP_URL`，使用平台绿色包；否则 fallback 到 winget。

PC 节点发布脚本会在构建机能找到 `rg.exe` 时额外上传 `ripgrep-windows.zip`，并把 `ripgrepZipUrl`、`ripgrepZipSha256`、`ripgrepZipFileSize` 写进 `node-agent-version.json`。Win 端不会把该工具包预装进客户端压缩包；只有用户主动点击 Codex 环境修复、且本机缺少 `rg` 时，`setup-node-env.ps1` 才按需下载到 `%LOCALAPPDATA%\ElonNode\tools\ripgrep\bin\rg.exe`。

后续新增工具时必须先进入 catalog，再决定安装策略：

- `AutoSmall`：小核心工具，允许安装/修复流程自动补齐。
- `ManualRepair`：只在用户点击对应修复项时安装。
- `NeverAuto`：永不自动安装，只探测和暴露。

## 收录标准

新增工具必须满足至少一个条件：

- 能明显减少 Codex 的检索时间或输出噪音。
- 能减少跨平台脚本失败率。
- 能让常见项目任务更稳定。

同时必须记录：

- 工具 ID、命令名、别名。
- 所属层级和安装策略。
- 绿色安装目录。
- 诊断环境变量名。
- 版本检查命令。
- 预估体积和来源校验方式。

## 当前 catalog

| 工具 | 层级 | 安装策略 | 作用 | 诊断变量 |
|---|---|---|---|---|
| `rg` | core | AutoSmall | 快速搜索代码和文件 | `ELON_CODEX_RG_PATH` |
| `fd` | profile | ManualRepair | 快速找文件 | `ELON_CODEX_FD_PATH` |
| `jq` | profile | ManualRepair | JSON 处理 | `ELON_CODEX_JQ_PATH` |
| `7z` | optional | NeverAuto | 解压和归档 | `ELON_CODEX_7Z_PATH` |

## 不做的事

- 不把所有工具都塞进安装包。
- 不在普通启动时自动下载 profile/optional 工具。
- 不把 Codex 桌面版内部资源当作长期安装来源。
- 不把工具目录无条件写入系统 PATH。
