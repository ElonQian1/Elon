# PC 工作台前端迁移路线

> 本文记录 `/pc` PC 工作台从原生静态 HTML/CSS/JS 迁移到 `Vite + React + TypeScript` 的路线、边界和模块状态。硬规则见 `.github/instructions/pc-frontend-migration.instructions.md`。

## 当前判断

- 当前 PC 工作台是后端内嵌静态资源：`server/src/assets/pc_app.html` 通过多个 `pc_*.js/css` 模块组成，服务端在 `server/src/web.rs` 用 `include_str!` 编译进 Rust binary，并在 `server/src/router.rs` 暴露 `/pc` 和 `/assets/pc_*`。
- 当前方式能继续支撑修复和小功能，但复杂状态、组件复用、类型检查和自动化验证已经接近维护临界点。
- 新 PC 功能应进入 `pc-frontend/`，旧 `pc_*` 静态资产只做修复、桥接和迁移删除。

## 目标形态

```text
pc-frontend/
  src/
    app/
    routes/
    features/
    components/
    api/
    styles/
  package.json
  vite.config.ts
  tsconfig.json

server/src/
  web.rs      # 托管 /pc、/pc-next 和构建产物
  router.rs   # 注册路由
```

迁移完成后，`/pc` 指向新前端；旧 `server/src/assets/pc_*.html/js/css` 删除或只保留短期 `/pc-legacy`，最终归零。

## 当前阶段：✅ 准备期完成（2026-06-26）

- `pc-frontend/` 工程已创建：Vite 5 + React 18 + TypeScript 5 + React Router 6 + Zustand
- `/pc-next` 路由已添加：`server/src/router.rs` 通过 `ServeDir` 托管 `$DATA_DIR/pc-next-dist/`
- SPA 路由通过 `not_found_service → index.html` fallback 支持
- 开发代理已配置：`vite.config.ts` 将 `/api` 代理到 `localhost:8080`
- 构建产物：`npm run build` → `pc-frontend/dist/`（已验证通过）
- **待完成**：`scripts/publish-server.ps1` 添加前端构建+上传步骤

## 路由策略

| 阶段 | `/pc` | `/pc-next` | `/pc-legacy` |
|---|---|---|---|
| **准备期** ✅ | 旧原生 PC | 新前端（ServeDir 从 `$DATA_DIR/pc-next-dist/` 服务） | 无 |
| 并行期 | 旧原生 PC | 新前端主要迁移入口 | 无 |
| 切换期 | 新前端 | 新前端同源或重定向 | 旧原生 PC |
| 收尾期 | 新前端 | 可重定向到 `/pc` | 删除 |

## 模块迁移顺序

| 模块 | 当前旧入口 | 目标状态 | 备注 |
|---|---|---|---|
| 应用壳、侧边栏、频道列表 | `pc_app.html`、`pc_app.js` | 未开始 | 新前端先建立壳和路由 |
| 登录、注册、账号菜单 | `pc_app.js` | 未开始 | 可优先迁移为独立 auth feature |
| 模型选择 | `pc_app_models.js`、`pc_app_models.css` | 未开始 | 状态清晰，适合早期迁移 |
| 项目中心、新建项目 | `pc_app_project_create.js`、`pc_app.js` | 未开始 | 迁移后旧项目创建弹窗应删除 |
| 项目广场 | `pc_app.js` | 未开始 | 注意与移动 `project_plaza.js` 区分 |
| AI 开发任务消息、审批、取消 | `pc_app_dev_tasks.js`、`pc_app_agent_runs.js`、`pc_app_task_snapshots.js` | 未开始 | 复杂度最高，应在新前端有稳定数据层后迁移 |
| 本机节点和客户端维护 | `pc_app_node.js`、`pc_app_node_admin.js`、`pc_app_client_maintenance.js` | 未开始 | 涉及 `elon-node://` 和本机 API，要保留兼容测试 |
| 电脑医生 | `pc_app_doctor.js` | 未开始 | 可作为独立 feature 迁移 |
| AI 声音/TTS | `pc_voice_project.js` | 未开始 | 与 `voice_tts_sdk.js` 边界要明确 |
| 通知 | `pc_app_notifications.js` | 未开始 | 可先保留为共享桥接，再迁移到新前端 |

## 多 AI 协作约定

- 开始 PC 前端任务前先 `git fetch origin main`，再读本文件和 `.github/instructions/pc-frontend-migration.instructions.md`。
- 一个任务只迁移一个明确模块，避免同时改新前端、旧多个模块和发布脚本。
- 如果必须修改旧 `pc_*.js/css/html`，先确认是否只是桥接或删除；新增复杂能力默认改到 `pc-frontend/`。
- 迁移模块后更新上方状态表，写清“旧入口是否删除、是否仍需 `/pc-legacy` 兜底”。
- 已有 AI 正在迁移某个模块时，其他 AI 不要同时编辑同一旧文件的大块逻辑；优先选择状态表中其他模块。

## 发布和验证原则

- 初期优先让新前端构建产物随 Rust binary 一起交付，保持现有后端发布模型简单稳定。
- 如果改为上传 `dist/` 到服务器，必须同步更新 Windows 和 Linux 发布脚本，并在文档中写明远端路径、缓存策略和回滚方式。
- 新前端任务至少验证 `npm run build`；涉及路由托管时还要验证 `/pc`、`/pc-next` 和静态资源路径。
- 旧 PC 修复至少验证对应页面能加载；涉及服务端内嵌资源时运行最小 Rust 检查或相关测试。
