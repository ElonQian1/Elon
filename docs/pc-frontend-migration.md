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

## 分阶段迁移计划（长远路线图）

> 每阶段完成后更新下方状态表。单次 AI 任务只迁移一个明确模块。

### 阶段 P1 — 对话核心补全（当前优先）

目标：让 `/pc` 的消息体验与旧版对齐。

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| Markdown 渲染（代码块/表格/列表/引用/链接） | 🔴 高 | `src/features/markdown/` | ✅ P1.1 完成 |
| 频道 WebSocket 实时消息推送 | 🔴 高 | `src/features/conversation/useChannelSocket.ts` | ✅ P1.2 完成 |
| 消息流式输出（AI 打字指示器 + 智能滚动）| 🟡 中 | `src/features/conversation/` | ✅ P1.3 完成 |
| 统一实时刷新架构（事件标准化 + 资源 key + 共享刷新 hook） | 🔴 高 | `src/features/realtime/` | ✅ P1.8 完成 |
| 消息附件上传（图片/文件） | 🟡 中 | `src/features/conversation/AttachmentButton.tsx` | ✅ P1.4 完成：支持不限扩展名选择、整会话区拖放、粘贴、多附件预览，并随 AI 任务发送结构化 attachments |
| 任务状态实时更新（task_done 事件） | 🟡 中 | `src/features/dev/` | 🟡 P1.7 已补过程覆盖诊断、当前卡点阶段、结构化命令/文件/测试过程卡、侧栏心跳陈旧提示、运行中停止入口、恢复控制面、项目级多任务现场总览；task_done 细化仍继续 |

### 阶段 P2 — 项目管理

目标：项目详情、成员、设置可以在新前端操作。

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 项目详情页（概览/设置/成员 Tab）| 🔴 高 | `src/features/projects/ProjectDetailPage.tsx` | ⬜ 未开始 |
| 成员列表 + 邀请 + 移除 | 🟡 中 | `src/features/projects/MemberPanel.tsx` | ⬜ 未开始 |
| 项目任务进度面板 | 🟡 中 | `src/features/projects/ProjectReadiness.tsx` | ⬜ 未开始 |
| 项目工作区设置（Git/Node） | 🟢 低 | `src/features/projects/WorkspaceAccessPanel.tsx` | 🟡 已补本机目录选择、节点重绑与 Route A 完全访问确认；Git 设置继续迁移 |
| 频道管理（新建/改名/删除）| 🟢 低 | `src/features/projects/ChannelSettings.tsx` | ⬜ 未开始 |

### 阶段 P3 — 个人 AI 对话

目标：旧版 "一龙AI" 频道 → 个人 AI 会话管理。

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 个人 AI 会话列表（`/api/me/ai/conversations`）| 🔴 高 | `src/features/ai/AiConversationList.tsx` | ⬜ 未开始 |
| AI 会话消息页 | 🔴 高 | `src/features/ai/AiChatPage.tsx` | ⬜ 未开始 |
| 会话历史分页加载 | 🟡 中 | `src/features/ai/` | ⬜ 未开始 |

### 阶段 P4 — 好友 & 社交

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 好友列表 + 在线状态 | 🟡 中 | `src/features/friends/FriendsPage.tsx` | ✅ P4.1 完整 presence 实时同步 |
| 好友私聊消息 | 🟡 中 | `src/features/friends/FriendChat.tsx` | ⬜ 未开始 |
| 添加好友 / 扫码 | 🟢 低 | `src/features/friends/AddFriend.tsx` | ⬜ 未开始 |

### 阶段 P5 — 项目广场 & 发现

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 项目中心双 Tab（我的项目 / 项目广场）| 🔴 高 | `src/features/projects/ProjectsPage.tsx` | ✅ 已恢复，`/pc/projects` 保留第二/右侧栏；默认中间展示项目广场 |
| 项目广场列表（过滤/搜索/分页）| 🟡 中 | `src/features/plaza/ProjectPlazaView.tsx` | ✅ 已完成，`/pc/plaza` 与项目中心广场 Tab 复用同一视图 |
| 加入/申请加入项目 | 🟡 中 | `src/features/plaza/` | ✅ 已完成，加入后刷新我的项目 |
| 项目卡片分享 | 🟢 低 | `src/features/plaza/ProjectCard.tsx` | ⬜ 未开始 |

### 阶段 P6 — 账号 & 设置

| 任务 | 优先级 | 模块/文件 | 状态 |
|---|---|---|---|
| 账号信息 + 修改密码 | 🟡 中 | `src/features/account/AccountPage.tsx` | ⬜ 未开始 |
| 绑定手机/邮箱 | 🟢 低 | `src/features/account/` | ⬜ 未开始 |
| 积分/余额查看 | 🟢 低 | `src/features/billing/BillingPage.tsx` | ⬜ 未开始 |

### 阶段 P7 — 移动端适配 & 收尾

| 任务 | 优先级 | 说明 | 状态 |
|---|---|---|---|
| 移动端响应式（< 780px）| 🟡 中 | 面板折叠、滑动抽屉 | ⬜ 未开始 |
| 键盘快捷键（Esc 关弹窗等）| 🟢 低 | 体验细节 | ⬜ 未开始 |
| PWA 离线壳 | 🟢 低 | service worker | 🟡 基础壳完成：缓存 `/pc` shell 和当前 hash 资源；浏览器要求 HTTPS/localhost 安全上下文 |

---

## 执行原则

1. **每次只做一个任务**：单次 AI 对话只迁移上表中一个模块，完成后更新状态为 ✅
2. **先 P1，再 P2**：对话核心不完整时不开始项目管理
3. **有 API 先看路由**：开始前先查 `server/src/router.rs` 确认 API 端点
4. **实时刷新复用**：`/ws/app` 消息先进入 `src/features/realtime/`，统一标准化为公开事件和资源 key；页面模块只声明自己关心的资源，并用共享刷新 hook 触发加载
5. **Markdown 统一**：渲染逻辑放 `src/features/markdown/`，不在各页面各自实现

## PC 实时刷新架构

PC 前端动态刷新分三层，后续新增模块必须接入这条链路，避免每个页面重复写 WebSocket 监听、轮询和焦点刷新：

1. `src/features/notifications/useNotifications.ts` 只负责连接 `/ws/app`，收到后端消息后调用 `normalizeRealtimeEvent`，同时保留旧 DOM 事件名给未迁移模块兼容。
2. `src/features/realtime/` 负责把后端事件映射为资源 key，例如 `project.space:{projectId}`、`channel.messages:{projectId}:{channelId}`、`conversation.any:{projectId}:{conversationId}`、`task.timeline:{projectId}:{taskId}`。
3. 页面或业务 hook 使用 `useRealtimeResourceRefresh`，传入 `resourceKeys`、`refresh` 和必要的兼容判断函数。通用 hook 统一处理事件防抖、运行中短轮询、完成后短暂收尾轮询、窗口重新聚焦刷新。

当前已接入模块：

- 当前频道消息刷新：`src/features/conversation/useChannelAutoRefresh.ts`
- 项目成员会话消息、任务过程和最终回复刷新：`src/features/conversation/useConversationRealtimeRefresh.ts`

新增实时模块时不要直接在页面组件里 `window.addEventListener('elon:...')`；优先新增或复用资源 key，再用共享 hook 连接加载函数。

---

## 下一步行动

**P1.1：Markdown 渲染** — 这是当前最高优先级，AI 回复纯文本严重影响体验。
实现计划：
- 安装 `marked` 或 `marked-react`
- 创建 `src/features/markdown/MarkdownContent.tsx`
- 在 `ConversationPage` 的 MessageItem 中集成

说"开始P1.1"即可直接实施。

- **全部 11 个旧 `pc_*` 模块迁移完成，27 个旧资产文件已删除**
- `/pc` 现在直接指向新 Vite + React + TypeScript 前端（ServeDir from `$DATA_DIR/pc-next-dist/`）
- `/pc-next` 保留为向后兼容别名（同样指向新前端）
- `/pc-legacy` 指向发布脚本从 `d1f89950`（`e7b4a35b` 引入新框架前的父提交）导出的旧版静态对照快照
- 发布脚本会在旧版“打开移动端”旁注入“打开新版”，并把旧版 token 桥接到新版 `elon_auth`
- `server/src/web.rs`：所有 `PC_*` 常量和 `pc_app_*` 处理器已删除
- `server/src/router.rs`：所有旧 `/assets/pc_*` 路由已删除
- 前端 Vite base 已改为 `/pc/`，React Router basename 已改为 `/pc`

## 路由策略（最终状态）

| 路径 | 服务内容 |
|---|---|
| `/pc` | 新 React 前端（主路由） |
| `/pc-next` | 新 React 前端（向后兼容别名） |
| `/pc-legacy` | 旧原生 PC 静态快照（只读对照，不恢复旧源码） |
| `/web` | 移动网页版（不变） |
| `/` | 移动网页版（不变） |

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
- Windows 本地预览 `/pc` 时用 `scripts\start-pc-frontend-dev.ps1`。这个脚本固定解析 `npm.cmd`；不要用 `Start-Process -FilePath npm`，否则某些机器会先命中 `npm.ps1` 并按系统文件关联打开到记事本。
- 旧 PC 修复至少验证对应页面能加载；涉及服务端内嵌资源时运行最小 Rust 检查或相关测试。
