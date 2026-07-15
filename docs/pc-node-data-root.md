# PC 节点数据根、构建缓存与磁盘治理

最后更新：2026-07-15

本文是一龙 Windows PC 节点大体积数据的产品合同。目标是：源码在哪个盘，工作区、构建缓存和任务临时文件就由用户选择的数据盘统一承载，不再因为 Windows 用户目录的默认行为悄悄写满 C 盘。

## 1. 为什么源码在 D 盘，C 盘仍会爆满

PC 节点过去只有分散的目录设置，没有统一的数据根：

- 项目工作区默认落在 `%USERPROFILE%\Elon\workspaces`。
- 硬盘节点仓库默认落在 `%APPDATA%\elon-node-agent\storage`。
- Rust 开发和节点发布 target 默认落在 `%LOCALAPPDATA%\Elon\build-target`。
- Cargo registry、Gradle、npm/pnpm 和应用临时文件继续使用各工具的用户目录或 `%TEMP%`。
- 会话 worktree 在 D 盘并不代表编译 target、Gradle 用户缓存和临时文件也在 D 盘。

Rust 首次编译常产生数 GB 到数十 GB target；多个项目、toolchain 或发布 profile 再各建一份，很容易成为 C 盘耗尽的直接触发因素。会话文本通常不是主要占用。

## 2. 唯一设置：`ELON_NODE_DATA_ROOT`

普通用户不需要选择目录。升级节点在第一次写代码或构建前，会优先在已绑定项目的同级目录自动创建独立数据根，例如项目位于 `D:\Projects\my-app` 时使用 `D:\Projects\ElonNodeData`；已有外部项目保持原位置。环境变量只供无人值守部署和高级管理员覆盖，例如：

```dotenv
ELON_NODE_DATA_ROOT=D:\ElonNodeData
```

该环境变量用于安装器、无人值守部署和首次启动引导；本地管理页保存后，`node.json` 中的持久化值优先，确保重启不会被安装包里遗留的旧环境值悄悄改回。管理员若要重新以环境变量接管，应先显式清除持久化数据根，而不是同时维护两个相互竞争的真源。

要求：

1. 必须是绝对路径，不能直接选择 `C:\`、`D:\` 等磁盘根。
2. 不能与旧 workspace、storage 或另一个数据根互相嵌套。
3. 目录必须可创建、可写，且不能是重解析点、junction 或符号链接。
4. 首次绑定时目录必须为空；节点先以不覆盖语义独占创建 marker，再创建派生目录。已有非空目录不能被“顺手认领”为数据根。
5. 根目录标记绑定当前节点 `install_id`，不能让两台节点误用同一目录；每次清理前都会重新校验 marker。
6. 节点凭证、登录 token 和小体积配置仍保留在 `%APPDATA%` 与安装目录的 `_internal\node-agent.env`；它们不是构建缓存，不能随着可移除数据盘迁移。

`ELON_NODE_WORKSPACE_ROOT`、`ELON_PC_WORKSPACE_ROOT`、`NODE_WORKSPACE_ROOT`、`NODE_STORAGE_ROOT` 和 `ELON_STORAGE_ROOT` 只保留给尚未配置统一数据根的旧节点。统一数据根一旦生效，它们只用于迁移发现，不能继续覆盖真实 workspace/storage，否则状态页面会显示 D 盘而新任务仍悄悄写回 C 盘。

## 3. 目录合同

```text
<ELON_NODE_DATA_ROOT>\
├─ .elon-node-data-root.json
├─ workspaces\
│  ├─ <user-id>\<project-id>\repo\
│  └─ conversation-worktrees\<project-id>\<conversation-id>\
├─ storage\
│  ├─ git\projects\<user-id>\<project-id>.git\
│  └─ worktrees\users\<user-id>\<project-id>\repo\
├─ cache\
│  ├─ cargo-home\
│  ├─ rust-targets\<project-id>\<toolchain-key>\target\
│  ├─ gradle-home\
│  ├─ npm\
│  ├─ pnpm-store\
│  └─ yarn\
└─ temp\<task-id>\
```

职责边界：

- `workspaces`：用户代码和会话 Git worktree，属于重要数据。
- `storage`：硬盘节点裸仓库和 owner checkout，属于重要数据。
- `cache`：依赖、编译 target 和包管理器缓存，可重建。
- `temp`：任务级下载、解包、附件和中间产物，可重建。

自动清理只能处理 `cache`、`temp`。任何 TTL、LRU 或“立即清理缓存”都不得删除 `workspaces`、`storage`、`.git`、未提交文件或用户 artifact。

## 4. Rust target 的共享边界

Rust target 采用：

```text
cache\rust-targets\<project-id>\<toolchain-key>\target
```

规则：

- 同一项目的基础 repo 和所有会话 worktree 共享 target，避免每个会话重复下载和编译。
- 不同项目使用不同 target，避免 feature、build script、环境变量和绝对 dep-info 互相污染。
- 不同 Rust toolchain 使用不同 `toolchain-key`，例如 `stable-msvc`、`nightly-msvc`；Cargo 会继续在 target 内按 target triple 分目录。
- 每个任务创建原子 lease，TTL/LRU 和人工清理都必须避开活跃 lease；同一项目与 toolchain 还持有跨进程 target 独占锁，不能仅依赖某一种构建工具自己的锁实现。
- 云端只把带项目上下文的 CLI/Exec 任务派给声明 `project_build_cache_v1` 能力的新节点；滚动升级期间，旧节点不会静默绕过治理并继续使用用户目录默认缓存。
- 跨项目复用依赖编译结果应使用受控的编译缓存层，不能把所有项目硬塞进一个 `D:\rust\shared\target`。
- 服务端 musl 发布、Windows 节点发布和用户项目开发 profile 不能混用同一个 target。

节点启动项目任务时应注入绝对 `CARGO_TARGET_DIR`。如需迁移 Cargo registry，可同时注入：

```text
CARGO_HOME=<root>\cache\cargo-home
```

平台仓库自身的开发与发布缓存不属于用户项目缓存，应在本机未提交的 `.env.local` 分开设置：

```dotenv
ELON_DEV_CARGO_TARGET_DIR=D:\rust\shared\elon-dev-cargo-target
ELON_NODE_AGENT_TARGET_DIR=D:\rust\shared\elon-node-agent-target
RUST_SERVER_MUSL_TARGET_DIR=D:\rust\shared\server-musl-target
```

三者的 target triple、profile 和 features 不同，不能为了“共享”而指向同一个目录。`cargo-dev.ps1` 和 `publish-node-agent.ps1` 会读取 `.env.local`；共享仓库脚本不会把某台机器的 `D:` 盘写死为所有用户默认值。

## 5. Gradle、Node 与任务 Temp 路由

节点启动 AI CLI、生成项目命令或构建子进程时，环境变量至少应包含：

| 工具 | 路由 |
|---|---|
| Gradle | `GRADLE_USER_HOME=<root>\cache\gradle-home` |
| npm | `npm_config_cache=<root>\cache\npm` |
| pnpm | store 指向 `<root>\cache\pnpm-store` |
| Yarn | `YARN_CACHE_FOLDER=<root>\cache\yarn`；项目内 `.yarn/cache` 随 workspace 一起位于数据盘 |
| Cargo | `CARGO_HOME` 和项目级 `CARGO_TARGET_DIR` |
| Windows 临时目录 | `TEMP=<root>\temp\<task-id>`、`TMP=...` |
| 跨平台临时目录 | `TMPDIR=<root>\temp\<task-id>` |

Gradle 项目自己的 `.gradle`、`build` 和 Android 输出仍位于项目 workspace；workspace 已在数据根，因此不会回流 C 盘。Android SDK、JDK、Rust toolchain 和 Codex 安装目录属于工具安装，不应被缓存清理接口删除。

任务结束后可回收任务 temp；异常终止时由 TTL 扫描补清。子进程仍在运行时不得删除对应 task temp。

## 6. 旧节点迁移策略

升级后自动完成数据根绑定，同时用以下规则保护 Git 数据：

1. 有现成外部项目路径时，在项目同盘、同级选择独立目录；不认领、不移动、不改名现有项目。
2. 候选目录必须为空或带当前 `install_id` marker；被其他文件占用时自动改用安装实例专属名称。
3. 路径、marker、重解析点、目录重叠全部校验成功后，才原子写入 `node.json`；失败不覆盖原配置。
4. 旧 workspace/storage 继续作为只读兼容和回滚来源；新会话 worktree、cache、temp 写入新根。
5. 旧会话 worktree 不递归搬运；需要继续时从已验证的基础 Git repo 重建，脏文件和未 push 提交原地保留。
6. storage 迁移只有在复制到暂存目录、执行 Git 完整性校验并可原子切换时才自动完成；否则旧目录保持不变并继续报告迁移计划。
7. 自动绑定失败时只阻止需要写入托管目录的操作；明确只读的外部项目诊断继续使用原项目，不进入构建容量门禁。

缓存不需要复制：新根创建空 cache 即可。旧 C 盘 target 可在确认没有 Cargo/Rustc 进程后删除；旧 Temp 只按白名单类型和年龄处理。

升级后若尚未配置有效统一根，客户端会在第一次写任务前自动准备并持久化。只有已有显式配置损坏、所有候选目录不可写或发生安全校验冲突时才停止，并明确说明原项目未被移动或删除。

## 7. 容量、TTL 与 LRU 默认

产品默认建议如下；管理员可以收紧，不能扩大到 workspace/storage：

| 项目 | 默认 |
|---|---|
| 构建前硬保留 | 4 GiB，可用 `ELON_NODE_BUILD_MIN_FREE_BYTES` 覆盖 |
| 单次构建增长余量 | 8 GiB，可用 `ELON_NODE_BUILD_HEADROOM_BYTES` 覆盖；准入时与硬保留、其他活动任务预留相加 |
| 节点 cache 配额 | 80 GiB，可用 `ELON_NODE_BUILD_MAX_CACHE_BYTES` 覆盖 |
| 单项目 Rust cache 配额 | 24 GiB，可用 `ELON_NODE_BUILD_MAX_PROJECT_RUST_BYTES` 覆盖 |
| 成功任务 temp | 子进程结束后立即清理 |
| 失败、取消或异常任务 temp TTL | 24 小时 |
| Rust/Gradle/Node cache TTL | 30 天未使用后进入 LRU 候选 |
| LRU 顺序 | 最久未使用且没有活跃 lease 的项目缓存优先；活动项目绝不删除 |
| 旧根回滚观察期 | 至少 7 天，并由用户显式确认清理 |

默认情况下，第一次真正需要写入或构建的项目任务启动前至少需要 12 GiB 可用空间：4 GiB 是磁盘安全底线，8 GiB 是本次任务余量；并发任务各自预留 8 GiB。明确只读的项目诊断不创建构建 lease，也不以该容量线阻断。任务结束后若仍低于“安全底线 + 活动任务预留 + 下一任务余量”，节点会先清理无活跃 lease 的可重建缓存。大型 Rust/Android 项目仍建议使用空间更充足的 D/E 盘，管理员可按项目实测提高余量。

## 8. 设置、状态与清理 API

本地管理 API 受本地管理员 token 保护。

查看状态和迁移计划：

```http
GET /api/node-data-root
```

设置新根：

```http
POST /api/node-data-root
Content-Type: application/json

{"root_path":"D:\\ElonNodeData"}
```

通过本地 API 设置时，`node.json` 是唯一持久化真源；写入采用同目录原子替换，失败时不会发布新的内存状态。当前进程会立即更新 `ELON_NODE_DATA_ROOT` 及所有派生路径，但仍建议重启节点，让以后启动的所有后台组件继承一致环境。安装目录 `_internal\node-agent.env` 只用于安装器或管理员手工配置，不和 API 进行非事务双写。

清理前预览：

```http
POST /api/node-data-root/cleanup
Content-Type: application/json

{"apply":false}
```

明确执行：

```json
{"apply":true}
```

清理接口检测到活动 CLI 或 Exec lease 时必须拒绝执行；未配置数据根时也必须拒绝，不能退回清理任意 `%USERPROFILE%` 或 `%TEMP%`。切换、任务准入和清理共用同一门闩，避免“检查时无任务、删除时任务已启动”的竞态。

## 9. 回滚

1. 停止创建任务并等待活动 CLI/Cargo/Gradle 进程退出。
2. 保留新旧根，不要先删除任一侧。
3. 在本地管理页将数据根恢复为上一个带相同 `install_id` marker 的根；无人值守节点应先清除持久化值，再修改 `ELON_NODE_DATA_ROOT`。
4. 重启节点并确认状态、项目 Git 远端和 workspace 可用。
5. 对 storage 执行完整性检查，对项目检查 `git status` 和远端分支。
6. 回滚完成后，新根的 cache/temp 可清；workspace/storage 仍需用户确认。

不能用修改环境变量的方式“回滚”尚未迁移或未 push 的工作区内容。长期恢复来源始终是已验证的 Git repo、remote 和 branch。

## 10. 旧 C 盘只读盘点

仓库提供 `scripts/inspect-node-disk-usage.ps1`：

```powershell
# 默认只预览
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\inspect-node-disk-usage.ps1

# 显式清理两个已知可重建 Rust target
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\inspect-node-disk-usage.ps1 -Apply

# 再包含 30 天前的严格 Temp 候选
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\inspect-node-disk-usage.ps1 -Apply -IncludeExpiredTemp -MinAgeDays 30
```

脚本拒绝系统根、候选根越界、重解析点和 Cargo/Rustc 活跃进程；默认不删除任何内容。它不是通用磁盘清理器，不处理 VS Code、Gradle、Codex 会话、浏览器或未知应用目录。
