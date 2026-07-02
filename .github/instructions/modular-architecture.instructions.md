---
applyTo: "**/*.{rs,kt,java,ts,tsx,js,jsx,toml,gradle}"
---

# 一龙项目 — 模块化与长期维护规则

> AI 代理编辑任何文件前，本规则自动生效。目标是避免继续制造巨型文件，让多 AI 并行开发时边界清楚、冲突更少、后续维护更稳。

## 🔒 最高级铁律（任何任务起手必须默念）

1. **写代码前先输出文件计划**：任何新功能，先用 5-15 行 JSON 说明"新建哪些文件、修改哪些文件、预估行数"，确认每个目标文件行数在预算内，再动手写代码。计划阶段发现冲突，改计划；不要先写代码再发现放不下。
2. **永远不允许制造新的巨型文件**：新建文件默认目标 ≤500 行；501-800 行可容忍但必须单一职责；超过 800 行必须拆分。入口/组装文件（`main.rs`、`router.rs`、`MainActivity.kt`）更严格，优先控制在 500 行以内。
3. **永远不允许在已有文件里继续叠加超额逻辑**：目标文件剩余预算不足时，必须先声明新模块接收本次新增逻辑，再动手。
4. **永远按职责模块化 + 长期主义**：先想"这块逻辑属于哪个领域、应该独立成什么模块"，再动手；不要因为"快"就塞回入口文件。
5. **永远先 `git fetch origin main` 再开干**：本项目长期多 AI 并发拆分，开始前 / 提交前都要同步远端，避免重复抽同一块代码。
6. **永远只 stage 本次任务的文件**：拆分提交 = `refactor(...)`，新功能 = `feat(...)`/`fix(...)`，不允许混合。

违反以上任何一条都视为工作流违规，必须回退并按规则重做。

## 核心原则

- 新功能默认按职责建模块，不把 UI、状态、网络、存储、Git、发布、诊断、prompt、解析等不同职责继续塞进同一个文件。
- 入口文件只负责组装和路由；业务逻辑、数据结构、协议解析、持久化、UI 构造、后台任务和外部命令执行应放到独立模块。
- 修改已有巨型文件时，优先把本次触碰到的成块职责抽到新文件；不要在 1500 行以上的文件里继续追加大段新逻辑。
- 拆分应保持行为不变，先搬迁再改功能；每次提交聚焦一个边界，避免“重构 + 新功能 + 文案 + 发布脚本”混在一起。
- 多 AI 并行时按模块分工，避免两个代理同时编辑同一个巨型文件；开始前先 `git fetch origin main` 并查看远端是否已有相同拆分。
- 新建模块必须显式 `git add`，并同步更新 `mod`/import/路由注册/测试入口，避免只提交引用文件漏掉新文件。
- 对 Rust 模块，优先使用 `server/src/<domain>/` 或明确命名的 sibling module；对 Android Kotlin，优先按 feature/helper/service 拆文件，不把所有行为留在 `MainActivity.kt`。
- 对用户项目也遵守相同长期主义：如果项目没有自己的模块边界说明，先按最小职责边界拆小，不要生成新的巨型入口文件。

## 判断阈值

### 按文件角色分级

| 文件角色 | 绿区（安全） | 黄区（可容忍） | 红区（必须拆分） |
|---|---|---|---|
| 入口/组装（`main.rs`、`router.rs`、`MainActivity.kt`） | < 300 行 | 300–500 行 | **> 800 行** |
| 业务逻辑模块 | < 500 行 | 501–800 行 | **> 800 行** |
| 工具/Helper 模块 | < 400 行 | 400–500 行 | **> 600 行** |
| 协议/Schema 定义 | < 600 行 | 600–800 行 | **> 1000 行** |
| 测试文件 | < 600 行 | 600–900 行 | **> 1000 行** |

### 函数级

- 单函数超过 80 行或同时处理两类以上职责：优先拆成小函数或独立 helper。
- 一次改动预计超过 5 个文件或跨多个职责：先拆成多个提交或多个任务。

> **500–800 行是可容忍区间**；800 行是所有普通业务模块的硬约束上限，超出即必须拆分，不讨价还价。

## 增量门禁

仓库提供 `scripts/check-source-size.ps1`，pre-push hook 会自动调用它。这个门禁不是一次性清空历史债务，而是阻止继续变差：

- 历史红区文件可以暂时存在，但本次提交不能让它继续增加行数。
- 原本未到红区的文件，本次提交不能跨入对应角色的红区阈值。
- 新增源文件超过 500 行会给出警告；新增文件进入红区会直接失败。
- 如果确实需要例外，必须先和用户确认，并显式运行 `scripts/check-source-size.ps1 -AllowRedGrowth` 说明理由；默认流程不得使用这个参数。

手动验证命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-source-size.ps1
```

## 本仓库重点治理对象

- `android/app/src/main/kotlin/com/elon/app/MainActivity.kt`：只保留 Activity 生命周期、顶层导航和模块组装；输入框、附件、会话列表、项目工作流、CLI 输出清洗、证据展示、账号/版本等职责应继续下沉。
- `android/app/src/main/kotlin/com/elon/app/McpDebugServer.kt`：HTTP/MCP 协议、工具注册、诊断工具、任务控制、网络探测、JSON 组装、鉴权应拆分。
- `server/src/project_api.rs`：HTTP handlers、WebSocket job、附件、Git/worktree、APK 分发、部署 key、项目状态应按领域拆分。
- `server/src/ai_cli.rs`：prompt 构建、CLI 进程执行、stream parser、native session/prewarm、intent gate、环境检查应保持分模块。

## 每次任务必做的"模块化执行清单"

> 这是**强制流程**，不是建议。AI 代理改任何代码前先按下表过一遍。

| # | 动作 | 命令 / 检查点 |
|---|---|---|
| 1 | 先看远端是否有人正在拆分 | `git fetch origin main && git log --oneline HEAD..origin/main` |
| 2 | **扫描目标文件行数，计算剩余预算** | `(Get-Content <file>).Count` 或 `wc -l <file>`，对照上方阈值表 |
| 3 | **输出文件计划 JSON（写代码前必做）** | 见下方"写前计划格式"；预算不足时先在计划里声明新文件 |
| 4 | 看自己要新增的逻辑是否能落进已有模块 | 优先复用 / 扩展现有 sibling module，不另起新文件 |
| 5 | 决策：在原文件内追加 vs 抽新模块 | 见下方"拆分决策树" |
| 6 | 如果要抽：先做"纯搬迁"提交（行为不变），再单独提交"新功能" | 两个 commit，message 类型分别是 `refactor` 与 `feat`/`fix` |
| 7 | 提交前再 `git fetch origin main`，避免其他 AI 已经抽走同一块 | 若已抽走则先基于远端新结构重定位修改；真正 rebase 仍只在 push 被 non-fast-forward 拒绝时发生 |
| 8 | 推送后立即 `git fetch origin main` 同步其他 AI 拆分进度 | 用 `git log --oneline --since="2 hours"` 看其他 AI 最近动作 |

### 写前计划格式（步骤 3 的输出）

```json
{
  "new_files": ["src/chat/session.rs"],
  "modify_files": [
    {"file": "src/router.rs", "current_lines": 148, "budget": 252, "estimated_add": 15},
    {"file": "src/chat/session.rs", "current_lines": 0, "budget": 600, "estimated_add": 280}
  ],
  "touch_large_files": false,
  "max_estimated_file_lines": 280
}
```

**规则**：`estimated_add` + `current_lines` 必须 ≤ 该角色的红区阈值；落在 501-800 行黄区时必须说明单一职责。超出则在 `new_files` 里声明新文件，不允许强行塞入现有文件。

### 拆分决策树

```
我要新加 N 行逻辑
  ├─ N < 30 行 且 目标文件在绿区 ──→ 直接加在原文件
  ├─ N < 30 行 且 目标文件在黄区 ──→ 优先落进已有 sibling 模块；无合适模块再原文件追加
  ├─ N < 30 行 且 目标文件在红区 ──→ 必须新建 sibling 模块，禁止原文件追加
  ├─ 30 ≤ N < 150 行 ─────────────→ 默认新建 sibling 模块（域名清晰即可）
  └─ N ≥ 150 行 或 涉及 ≥ 2 类职责 → 必须多个模块；如果原文件在红区，先拆再加
```

### 多 AI 并发模块化的协作纪律

- **同一巨型文件不要两个 AI 同时拆**：开始拆分前看 `git log --oneline --since="1 day" -- <file>`，最近 24 小时其他 AI 在改，就先选别的文件或等待。
- **频繁 `git fetch`**：每完成一个"纯搬迁"小提交就 fetch 一次，看其他 AI 是否已经在另一个分支抽走相同代码块。
- **抽出的模块名要稳定**：用领域名（`codex_stream`、`project_ws_protocol`、`peer_relay`），不要用 `utils`、`helpers`、`common` 这类垃圾筐名。
- **commit message 必须显式标注**：`refactor(server): 抽取 <模块名>，瘦身 <原文件> 从 <旧行数> → <新行数>`，便于其他 AI 一眼看出拆分进度。
- **不可强行重命名其他 AI 刚抽出的模块**：除非有充分理由（命名错误、领域混淆），否则尊重既有边界。

## 禁止行为（出现即视为违规）

- ❌ 跳过"写前计划 JSON"直接写代码
- ❌ 新建文件超过 800 行；入口文件超过 500 行仍继续塞功能
- ❌ 在红区文件中追加 ≥ 30 行新逻辑而没有顺手抽模块
- ❌ 一次 commit 同时包含"重构 + 新功能"
- ❌ 新建模块时漏掉 `mod` 声明 / import / 路由注册 / `git add`（pre-push hook 会拦截 Rust，其他语言要自查）
- ❌ 用 `utils.rs` / `helpers.kt` / `common.ts` 这类无领域含义的命名
- ❌ 不看 `git fetch` 结果就开始大改，导致和其他 AI 抽出的同一块代码冲突
- ❌ 把"我只改这一个小功能"当借口在红区文件里继续堆代码

## 推荐目录形态

```
server/src/
  main.rs                # 只声明 mod + 启动
  router.rs              # 路由组装
  types.rs               # 共享类型，<800 行；超出按领域拆子模块
  <domain>.rs            # 每个领域一个文件，常态 < 1500 行
  <domain>/              # 领域内继续拆分时，建子目录
    mod.rs               # 只 re-export
    handlers.rs
    state.rs
    parser.rs
```

```
android/app/src/main/kotlin/com/elon/app/
  MainActivity.kt        # 只做生命周期 + 顶层组装
  ui/                    # Compose / View 组件
  data/                  # repository / store
  service/               # 后台服务、调度器
  net/                   # HTTP / WS 客户端
  feature/<name>/        # 按业务功能聚合
```
