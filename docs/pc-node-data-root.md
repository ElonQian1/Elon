# PC 节点项目数据架构体检与渐进治理

最后更新：2026-07-17

本文是一龙 Windows PC 节点数据根与构建缓存的产品合同。数据根不是“项目能不能运行”的许可证，而是一项 Harness 式辅助能力：先继承已经跑通的项目和缓存，再由 AI 只读分析、解释风险、给出整理方案，最后按用户确认渐进迁移。

## 1. 用户能听懂的一句话

旧项目原来怎么跑，升级后还怎么跑。一龙推荐数据根只是给新建托管项目准备一个更整齐的工作区、缓存和临时文件目录；没配置、空间偏少或自动准备失败，都只提示，不会拦住已有项目，也不会复制一份旧缓存来额外占空间。

## 2. 永久产品不变量

1. **继承优先**：已验证可运行的项目继续使用原路径、原环境变量和原共享缓存。
2. **按路径判定**：只有实际位于当前节点数据根 `workspaces` 或 `storage` 下的项目，才启用平台推荐构建环境；不能仅凭“有项目上下文”就强制接管。
3. **建议不阻断**：磁盘余量、缓存大小和项目数量都是体检指标，不是 CLI、Exec、写任务或构建任务的准入门禁。
4. **自动回填可失败**：缺少推荐数据根时客户端可安全创建并持久化；失败后继续使用原项目，不把配置问题伪装成“项目空间不足”。
5. **只读发现不等于接管**：发现外部缓存后 `automatic_action` 必须为 `none`；不得自动移动、改名、删除、写 marker 或改写项目环境。
6. **显式清理有边界**：节点数据清理只覆盖当前节点数据根内由平台创建、可重建的 `cache` 和 `temp`；开发机显式激活的 `rust-cache-v2` 另按磁盘水位治理。外部共享缓存、源码、Git、workspace、storage 和 artifact 永不进入自动清理范围。
7. **迁移可预览、可回滚**：先说明收益、兼容性和空间变化，再由用户或 AI 在获得明确授权后执行；原缓存保留到新路径验证通过和观察期结束。

## 3. 为什么以前会突然多占空间

旧版本里可能同时存在多套已经被脚本和项目复用的缓存：

- 项目同级或跨子项目的共享 Rust `target`；
- 当前仓库开发检查、测试共用的 target；
- Windows 节点 EXE 发布缓存；
- Linux 服务器/musl 发布缓存；
- 仓库内部历史 `target`。

如果升级只创建一个空数据根并强制注入新的 `CARGO_TARGET_DIR`，Cargo 会重新编译依赖，旧缓存仍在磁盘上，于是用户看到的是“缓存明明还在，却又要额外空间”。这是升级兼容缺失，不是用户项目突然变大。正确做法是对旧项目保留原环境，对新托管项目采用新架构，并把旧缓存先登记为外部可复用候选，而不是遗忘或复制。

## 4. 两种数据策略

| 项目类型 | 判定 | 默认行为 |
|---|---|---|
| 旧项目、用户自选目录、外部 Git 项目 | 工作目录不在当前数据根的 `workspaces/storage` 下 | 保留原项目和旧版会话 worktree 规则；保留传入和进程继承的缓存环境；不进入托管缓存准入 |
| 新建平台托管项目 | 工作目录位于当前数据根的 `workspaces/storage` 下 | 使用推荐会话工作区、项目级构建目录和任务临时目录；容量只提示 |

明确的新建托管 workspace/storage 协议仍可要求新节点能力，因为它是在创建新数据，不是让旧项目继续运行。服务器不得因为旧节点缺少新缓存能力而拒绝已有项目的普通 CLI/Exec，也不得在节点选择时过滤掉已经绑定且可运行的旧节点。

## 5. 推荐数据根

普通用户无需手动设置。客户端可以已绑定项目的位置为提示，在同盘安全位置自动准备推荐根；高级管理员可设置：

```dotenv
ELON_NODE_DATA_ROOT=D:\ElonNodeData
```

目录合同：

```text
<ELON_NODE_DATA_ROOT>\
├─ .elon-node-data-root.json
├─ workspaces\
├─ storage\
├─ cache\
│  ├─ cargo-home\
│  ├─ rust-targets\<project-id>\<toolchain-key>\target\
│  ├─ gradle-home\
│  ├─ npm\
│  ├─ pnpm-store\
│  └─ yarn\
└─ temp\<task-id>\
```

安全要求：

- 根必须是绝对路径，不能直接使用磁盘根，也不能嵌套进已有项目、旧 workspace/storage 或另一个数据根。
- 首次认领必须是空目录或带当前 `install_id` marker 的既有根；拒绝文件、符号链接、junction、重解析点和其他节点 marker。
- 先完成路径和 marker 校验，再原子持久化 `node.json`，最后更新内存状态；失败保留旧配置。
- 凭证、登录 token 和小体积节点配置仍在既有安全位置，它们不是构建缓存。

## 6. 五类缓存的体检规则

| 类别 | 常见来源 | 推荐作用域 | 默认建议 |
|---|---|---|---|
| 机器级 Rust 缓存平台 v2 | `ELON_RUST_CACHE_ROOT`、`shared\rust-cache-v2` | 所有注册 Rust 项目 | sccache 跨项目共享，build-dir 按兼容域和工作区隔离；只治理平台自有分区 |
| 历史跨子项目共享 Rust 缓存 | `CARGO_TARGET_DIR`、项目祖先的 `shared\target` | 机器/产品家族共享 | 登记并原地复用；兼容性变化时只重建受影响部分 |
| 当前开发检查、测试共享缓存 | `ELON_DEV_CARGO_TARGET_DIR`、`%LOCALAPPDATA%\Elon\build-target\elon-dev-cargo` | 同仓库跨 worktree 的旧版路径 | 作为 legacy 登记；新版 `cargo-dev` 使用 v2 build-dir 与 workspace-local target |
| Win 节点发布缓存 | `ELON_NODE_AGENT_TARGET_DIR`、`...\elon-node-agent` | Windows 发布 | 保持发布专用，不和开发或服务器 target 混用 |
| 服务器发布共享缓存 | `RUST_SERVER_MUSL_TARGET_DIR`、`ELON_BUILD_TARGET_DIR`、`shared\server-musl-target` | Linux/musl 发布 | 继续由服务器发布脚本复用 |
| 仓库旧缓存 | `<repo>\target`、`<repo>\server\target` | 单仓库历史 | 先确认最后使用入口；可保留复用，不自动删除 |

“共享”不是把所有 target 粗暴合并。AI 必须比较 toolchain、target triple、profile、features、build script、环境变量和锁策略；不兼容的发布/开发作用域应分开，兼容的同项目 worktree 应尽量共享。

## 7. 自动回填与失败降级

推荐数据根不存在时，客户端可尝试：

1. 从持久化配置、环境变量和已绑定项目位置推导候选根。
2. 校验路径、可写性、重解析点、marker 和目录重叠。
3. 创建托管目录并原子保存。
4. 只对新托管项目启用推荐环境。

任一步失败都应记录可诊断原因并继续旧项目任务。无配置状态对 API 表示为 `configuration_recommended=true`、`configuration_required=false`、`governance_mode=advisory`。只有显式创建新托管 workspace 这类确实需要托管目录的操作，才可返回明确的创建失败；不得把失败扩大到外部项目。

## 8. 容量是建议，不是硬门禁

以下值保留作为体检基线，帮助 AI 估算风险：

| 指标 | 默认建议 |
|---|---|
| 磁盘安全余量 | 4 GiB |
| 单任务增长余量 | 8 GiB |
| 节点托管 cache | 80 GiB |
| 单托管项目 Rust cache | 24 GiB |
| 失败任务诊断 temp | 建议保留 24 小时 |
| 长期未使用缓存 | 30 天后列为人工整理候选 |

上述指标适用于节点数据根，超过建议值时普通 CLI/Exec 任务继续运行，只生成警告。节点数据根不因压力自动删除外部或历史缓存；成功任务自己的临时目录仍可在进程结束后清理。

显式安装的开发机 `rust-cache-v2` 使用另一套合同：不设置项目级固定容量，只在整盘低于 warning 水位时按旧工具链、quarantine、LRU 顺序回收平台自有分区，达到 recovery 水位即停止；活动锁和 Cargo/rustc 进程会阻止实际删除。详见 `docs/rust-cache-platform.md`。

## 9. 体检与管理 API

本地管理 API 受管理员 token 保护。

```http
GET /api/node-data-root
```

返回推荐根状态、建议容量和不计算递归大小的缓存候选，保证常规页面刷新轻量。

```http
POST /api/node-data-root/analyze
```

用户点击“分析本机缓存架构”后执行只读大小统计，返回五类候选、来源、作用域、是否由平台管理、估算大小和建议。该接口不移动、不认领、不删除目录。

```http
POST /api/node-data-root
Content-Type: application/json

{"root_path":"D:\\ElonNodeData"}
```

只改变后续新建托管数据的位置，不改变已有外部项目。

```http
POST /api/node-data-root/cleanup
Content-Type: application/json

{"apply":false}
```

`apply=false` 只预览；`apply=true` 只删除推荐根内平台自建的可重建 cache/temp。活动任务期间禁止切换根和实际清理，避免竞态。

## 10. 渐进整理流程

1. **盘点**：记录观察到的工作区、环境变量、`.env.local` 和约定目录；不修改现场。
2. **解释**：告诉用户哪些缓存正在共享、哪些是重复或隔离合理，以及迁移会新增还是释放多少空间。
3. **建议**：优先保持已跑通路径；只有新架构能明显降低重复、提升可观测性或避免 C 盘风险时才建议迁移。
4. **预演**：给出来源、目标、兼容性检查、预计耗时、磁盘峰值和回滚路径。
5. **执行**：取得明确授权后，先复制/重建并验证，再切换单个作用域；不做跨类别“大搬家”。
6. **观察**：完成真实 check/test/build/publish 验证，保留旧缓存观察期。
7. **清理**：再次预览并由用户确认；外部缓存即使已迁移也不自动删除。

## 11. 发布回归要求

每次修改数据根、节点握手、任务派发、构建环境或缓存清理时，至少覆盖：

- 无数据根的旧 `node.json`、损坏显式配置和自动回填失败；已有外部项目仍能执行 CLI 与 Exec。
- 旧节点没有 `project_build_cache_v1` 能力；已有项目仍可派单，新建托管 workspace 协议才拒绝。
- 外部项目保留 cwd 和环境；托管项目才注入推荐缓存。
- 低磁盘、超建议配额、项目数偏多只产生建议，不拒绝任务、不自动清理。
- 五类缓存盘点结果全部为只读候选，`automatic_action=none`。
- 清理越界、重解析点和活动 lease 均被拒绝。

事故与灰度规则见 `docs/node-agent-upgrade-compatibility.md`。
