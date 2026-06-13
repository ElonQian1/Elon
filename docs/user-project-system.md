# 用户、项目与 Git 工作区体系设计

本文档用于把当前“按 user_id/project_id 拼工作区”的雏形，升级为适合给朋友使用的产品级体系。目标是让用户重新登录后能找回自己的项目，能新建项目，服务端能为每个项目建立独立 Git 仓库，并且 Web 端和 APK 端都能显示同一套项目选择与管理 UI。

## 1. 核心原则

1. 客户端不再决定真实用户身份。
   - 现在客户端会传 `user_id`，服务端直接信任它。
   - 升级后客户端只传登录 token，服务端从 token 解析真实用户。

2. 项目成为一等对象。
   - 项目不只是 `user_id__project_id` 目录名。
   - 项目有数据库记录、成员权限、工作区路径、Git 仓库和任务历史。

3. 用户和项目是多对多关系。
   - 一个用户可以有多个项目。
   - 一个项目也可以授权给多个用户。
   - 通过 `project_members` 表控制 owner/editor/viewer 权限。

4. 服务端负责恢复状态。
   - 用户重新登录后，服务端根据用户 ID 查询项目列表。
   - 客户端只负责展示，不需要记住项目目录。

5. 每个项目一个 Git 仓库。
   - 新建项目时服务端创建工作区目录。
   - 初始化模板代码。
   - 执行 `git init`。
   - 提交初始版本。
   - 后续每次 AI 修改都产生 commit。

## 2. 数据模型

初期推荐使用 SQLite，部署简单。后续用户多了再迁移 PostgreSQL。

### users

```sql
CREATE TABLE users (
  id TEXT PRIMARY KEY,
  phone TEXT UNIQUE,
  email TEXT UNIQUE,
  password_hash TEXT NOT NULL,
  nickname TEXT,
  role TEXT NOT NULL DEFAULT 'user',
  status TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### sessions

```sql
CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  token_hash TEXT NOT NULL UNIQUE,
  device_name TEXT,
  expires_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### projects

```sql
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  workspace_key TEXT NOT NULL UNIQUE,
  template TEXT NOT NULL DEFAULT 'android',
  status TEXT NOT NULL DEFAULT 'active',
  created_by TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (created_by) REFERENCES users(id)
);
```

### project_members

```sql
CREATE TABLE project_members (
  project_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  role TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, user_id),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (user_id) REFERENCES users(id)
);
```

角色建议：

| 角色 | 权限 |
| --- | --- |
| owner | 创建任务、修改项目、管理成员、下载 APK、删除或归档项目 |
| editor | 创建任务、修改代码、构建 APK、下载 APK |
| viewer | 查看项目、查看历史、下载 APK，不允许触发 AI 修改 |

### tasks

```sql
CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  message TEXT NOT NULL,
  status TEXT NOT NULL,
  git_branch TEXT,
  git_commit TEXT,
  apk_url TEXT,
  error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### project_events

```sql
CREATE TABLE project_events (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  user_id TEXT,
  event_type TEXT NOT NULL,
  payload_json TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (user_id) REFERENCES users(id)
);
```

用于记录项目创建、成员变更、AI 修改、构建成功、构建失败、APK 下载等历史。

## 3. 工作区目录结构

推荐从当前：

```text
WORKSPACE_ROOT/{user_id__project_id}/
```

升级为：

```text
WORKSPACE_ROOT/
  projects/
    prj_8f3a92c1/
      .git/
      android/
      agent_config.json
      build/
      artifacts/
```

其中：

- `projects.id` 是数据库项目 ID。
- `projects.workspace_key` 是目录名，比如 `prj_8f3a92c1`。
- 服务端只通过数据库查 workspace，不再用客户端传来的字符串拼目录。

## 4. 用户重新登录后如何找回上一轮项目

登录流程：

1. 用户输入手机号/邮箱和密码。
2. 服务端验证账号。
3. 服务端创建 session，返回 token。
4. 客户端保存 token。
5. 客户端请求 `/api/me/projects`。
6. 服务端根据 token 找到 `user_id`。
7. 服务端查询 `project_members`，返回该用户可访问的项目列表。

关键点：项目能找回，是因为项目记录在数据库里，不是因为 APK 本地记住了目录。

接口：

```http
POST /api/auth/login
```

请求：

```json
{
  "account": "friend@example.com",
  "password": "123456"
}
```

响应：

```json
{
  "token": "plain-token-returned-once",
  "user": {
    "id": "usr_123",
    "nickname": "小王",
    "role": "user"
  }
}
```

项目列表：

```http
GET /api/me/projects
Authorization: Bearer <token>
```

响应：

```json
{
  "projects": [
    {
      "id": "prj_8f3a92c1",
      "name": "我的第一个 APK",
      "role": "owner",
      "template": "android",
      "status": "active",
      "last_task_status": "done",
      "last_apk_url": "https://example.com/download/...",
      "updated_at": "2026-05-22T12:00:00Z"
    }
  ]
}
```

## 5. 如何新建项目

> 2026-06 更新：服务器磁盘不再承载普通用户的新代码项目。新建用户项目必须创建在用户自己的在线 PC 节点上；服务器只保存项目档案、成员权限、会话消息、任务状态和 artifact 索引。

新建项目流程：

1. 用户在 Web 或 APK 点击“新建项目”。
2. 输入项目名，选择模板，比如 Android。
3. 客户端请求 `/api/projects`，默认 `execution_target = "pc_node"`。
4. 服务端选择当前用户的在线 PC 节点；没有在线节点则拒绝创建，有多个在线节点则要求客户端显式传 `node_id`。
5. 服务端创建项目数据库记录和 owner 成员记录。
6. 服务端通过 PC relay 发送 `ProvisionProjectWorkspace`。
7. 如果项目带 `repo_url`，PC 节点在自己的受控根目录下 `git clone/fetch/checkout`；如果没有 `repo_url`，PC 节点创建本地新仓库、初始化 Git 仓库、写入项目说明文件。
8. PC 节点回传 `workspace_path`、`git_head`、`git_remote_origin` 和 `git_branch`。
9. 服务端把项目标记为 `source_type = pc_managed`，写入 `node_id + workspace_path`，并保存 `repo_url + branch` 作为后续迁移的重建来源。
10. 返回项目详情。

PC 节点创建项目的门槛是 `workspace_provision_ready`（一龙 PC 开发运行时可用），不是 `cli_project_ready`。`cli_project_ready` 只表示 Codex/Copilot 等 AI CLI 可用，影响后续 AI 开发任务，不应阻止“创建目录 + 初始化 Git”的基础项目创建。

PC 节点默认目录结构：

```text
ELON_NODE_WORKSPACE_ROOT/
  usr_xxx/
    prj_xxx/
      repo/
      worktrees/
      artifacts/
      logs/
```

如果 PC 节点创建失败，服务端应清理本次新建的项目档案，不给客户端返回一个不能执行的假项目。

手机端不能直接传任意 PC 路径。路径必须由 PC 节点根据 `project_id` 在受控根目录内生成。

服务端不能把 `workspace_path` 当作本机路径处理。只要项目有 `node_id`，代码、编译、部署都必须通过该 PC 节点执行。

接口：

```http
POST /api/projects
Authorization: Bearer <token>
```

请求：

```json
{
  "name": "记账小工具",
  "description": "给自己用的安卓记账 APK",
  "template": "android",
  "execution_target": "pc_node",
  "node_id": "node-xxx",
  "repo_url": "git@github.com:friend/accounting-app.git",
  "branch": "main"
}
```

`node_id` 在用户只有一个在线 PC 节点时可省略。

`repo_url` 和 `branch` 可省略。省略时项目只保证在当前 PC 节点本地可执行；后续如果要在其它 PC 节点重建，必须先配置 Git 远端并把代码 push 到远端。

### 5.1 PC 项目跨节点恢复

PC 工作区的本地路径只代表“当前执行位置”，不能作为长期可信来源。长期可信来源分两层：

1. `node_id + workspace_path`：当前应该在哪台 PC 节点、哪个本地目录执行。
2. `repo_url + branch`：当当前 PC 离线或需要迁移到其它 PC 节点时，从哪里重建代码。

恢复/迁移规则：

1. `recreate_workspace` 可以在原绑定节点重建或确认目录。
2. `migrate_workspace` 必须有可访问的 `repo_url`；目标 PC 节点会 clone/fetch 该远端并 checkout 指定分支。
3. `bind_pc_node` 如果是把已有项目换到另一台 PC 节点，也必须有 `repo_url`。
4. 没有 Git 远端的项目不能跨节点“空重建”，服务端必须返回明确错误，提示用户先配置远端并 push。
5. 远端仓库的认证由目标 PC 节点上的 Git/SSH/token 环境负责；服务器只保存项目的远端地址和分支，不持有用户 PC 的本地 Git 凭证。

响应：

```json
{
  "project": {
    "id": "prj_8f3a92c1",
    "name": "记账小工具",
    "role": "owner",
    "template": "android",
    "source_type": "pc_managed",
    "node_id": "node-xxx",
    "workspace_path": "D:\\Elon\\workspaces\\usr_xxx\\prj_xxx\\repo",
    "status": "active"
  },
  "provisioned": true,
  "workspace_created": true
}
```

错误场景：

- 没有在线 PC 节点：返回 503，提示用户先启动 PC 节点。
- 多个在线 PC 节点但未指定 `node_id`：返回 409，提示选择节点。
- PC 节点目录创建或 Git 初始化失败：返回 503，并清理本次项目记录。

### 5.1 用户档案、系统项目与会话归档

每个用户登录后，服务端以 `users.id` 作为档案根。档案下面统一挂载三类项目：

1. `手机控制`
   - `projects.source_type = 'agent_balloon'`
   - 悬浮球语音、手机控制脚本、悬浮球普通对话都归档到这个真实项目 ID。
   - `/api/agent-balloon/ensure` 和默认 `/api/llm/chat` 都必须返回并使用这个项目 ID。

2. `聊天记忆`
   - `projects.source_type = 'chat_memory'`
   - PC/Web/APK 的普通聊天归档到这个项目。
   - `/api/llm/chat` 传 `scope = "chat_memory"` 时，服务端自动确保该项目存在，并把会话写入该项目。

3. 用户新建或打开的真实项目
   - `projects.source_type = 'pc_managed'` 或 `local_path`。
   - 每个项目有自己的会话列表、任务列表、成员权限、`node_id` 和 `workspace_path`。
   - 只要项目绑定 `node_id`，代码、编译和部署必须路由到该 PC 节点。

会话归档规则：

```text
users.id
  手机控制(project_id=A)
    conversations / messages
    user_memories(scope_type=phone_control, scope_id=A)
  聊天记忆(project_id=B)
    conversations / messages
    user_memories(scope_type=chat_memory, scope_id=B)
  用户项目(project_id=C)
    conversations / messages / tasks
    user_memories(scope_type=project, scope_id=C)
    node_id + workspace_path -> PC 节点真实目录
```

`user_memories` 必须支持 `scope_type + scope_id`。读取上下文时合并全局记忆和当前作用域记忆；写入时只写当前作用域，避免悬浮球、普通聊天和项目开发互相污染。

## 6. PC 节点如何建立 Git

PC 节点收到 `ProvisionProjectWorkspace` 时执行下面的逻辑。

伪代码：

```rust
fn create_project_workspace(project: &Project, user: &User) -> Result<()> {
    let workspace = pc_workspace_root
        .join(safe(user.id))
        .join(safe(project.id))
        .join("repo");

    std::fs::create_dir_all(&workspace)?;

    copy_template("android", &workspace)?;

    run("git", ["init"], &workspace)?;
    run("git", ["config", "user.email", &format!("{}@elon.app", user.id)], &workspace)?;
    run("git", ["config", "user.name", &user.nickname_or_id()], &workspace)?;
    run("git", ["add", "."], &workspace)?;
    run("git", ["commit", "-m", "chore: initialize project"], &workspace)?;

    Ok(())
}
```

每次 AI 任务执行时：

1. 查用户是否有 `editor` 或 `owner` 权限。
2. 查项目工作区。
3. 创建任务记录。
4. 可选：创建任务分支。
5. 让 AI 修改代码。
6. 构建 APK。
7. 提交 commit。
8. 更新任务记录。

推荐分支格式：

```text
task/{task_id}
```

简单版也可以先只在 `main` 分支提交。给朋友少量使用时，先用 `main` 更省心；等并发任务变多，再升级成任务分支。

## 7. 聊天和构建接口如何改

当前 `/api/chat` 请求里有 `user_id` 和 `project_id`。升级后改成：

```http
POST /api/projects/{project_id}/chat
Authorization: Bearer <token>
```

请求：

```json
{
  "message": "帮我把首页按钮改成蓝色",
  "agent": "codex_cli"
}
```

服务端流程：

1. token -> user。
2. project_id -> project。
3. 检查 `project_members`。
4. 查 workspace。
5. 执行 AI agent。
6. 记录 task。
7. 返回 reply、apk_url、commit。

WebSocket 也类似：

```text
GET /ws/projects/{project_id}
Authorization: Bearer <token>
```

或者在连接参数里传 token：

```text
wss://example.com/ws/projects/prj_123?token=...
```

消息体只需要：

```json
{
  "message": "帮我生成一个登录页",
  "agent": "codex_cli"
}
```

## 8. 下载 APK 如何做权限

当前下载接口：

```text
/download/{user_id}/{filename}
```

升级为：

```text
/api/projects/{project_id}/artifacts/{artifact_id}/download
```

服务端检查：

1. token 是否有效。
2. 用户是否属于该项目。
3. artifact 是否属于该项目。
4. 返回 APK 文件。

如果希望分享给朋友安装，可以生成短期公开链接：

```sql
artifact_links
- id
- artifact_id
- token_hash
- expires_at
- created_by
```

公开链接：

```text
/public/download/{share_token}
```

## 9. Web 端 UI

Web 端应该从现在的单页聊天，升级为三层结构：

```text
登录页
  ↓
项目工作台
  ↓
项目详情 / AI 对话 / 构建历史 / 设置
```

### 登录页

元素：

- 账号输入框。
- 密码输入框。
- 登录按钮。
- 首次使用可以由管理员创建账号，不开放公开注册。

### 项目工作台

左侧或顶部：

- 当前用户昵称。
- 新建项目按钮。
- 退出登录。

主体：

- 项目卡片列表。
- 每张卡片显示：
  - 项目名
  - 用户角色
  - 最近一次任务状态
  - 最近 APK 下载入口
  - 更新时间

空状态：

- 没有项目时显示“新建项目”主按钮。

### 新建项目弹窗

字段：

- 项目名称。
- 项目描述。
- 模板选择：
  - Android APK
  - 后续可加 Web、Rust 服务、空项目

提交后：

- 显示“正在创建工作区”。
- 创建成功后自动进入项目详情。

### 项目详情页

建议四个 Tab：

1. AI 对话
   - 输入需求。
   - 选择模型/代理。
   - 显示执行进度。
   - 显示最终 APK 链接。

2. 构建历史
   - 每次任务一行。
   - 状态、提交、APK、错误日志。

3. 文件/Git
   - 当前分支。
   - 最近 commit。
   - 后续可显示变更摘要。

4. 设置
   - 项目名称。
   - 成员管理。
   - AI 代理设置。
   - 归档项目。

## 10. APK 端 UI

APK 端也应该和 Web 端共用同一套接口。

### 首次打开

流程：

```text
启动页
  ↓
检查本地 token
  ↓
token 有效：进入项目列表
token 无效：进入登录页
```

### 登录页

元素：

- 账号输入。
- 密码输入。
- 登录按钮。

登录成功后：

- token 保存到 Android SharedPreferences 或 EncryptedSharedPreferences。
- 请求 `/api/me/projects`。
- 进入项目列表。

### 项目列表页

底部导航建议：

- 项目
- 设置

项目页：

- 顶部显示用户昵称。
- 列出项目卡片。
- 右上角或底部悬浮按钮“新建项目”。

项目卡片：

- 项目名称。
- 最近状态。
- 最近 APK 下载按钮。
- 点击进入项目聊天页。

### 新建项目页或弹窗

元素：

- 项目名称。
- 模板选择。
- 创建按钮。

创建成功后：

- 自动进入项目聊天页。

### 项目聊天页

元素：

- 顶部项目名称。
- 消息列表。
- 进度消息。
- 输入框。
- 发送按钮。
- 构建完成后显示“安装/下载 APK”按钮。

### 项目切换

聊天页顶部项目名可以点击，弹出项目选择器：

- 当前项目打勾。
- 其他项目列表。
- 新建项目入口。

## 11. 管理后台 UI

管理后台应该从“AI 代理配置中心”升级为“系统管理中心”。

Tabs：

1. 用户
   - 创建用户。
   - 停用用户。
   - 重置密码。
   - 查看该用户的项目。

2. 项目
   - 查看所有项目。
   - 查看成员。
   - 添加/移除成员。
   - 查看最近任务。
   - 归档项目。

3. AI 代理
   - 继续保留当前能力。

4. 系统日志
   - 项目事件。
   - 构建失败。
   - 登录记录。

## 12. 推荐实现顺序

### 第一阶段：最小可用多用户

1. 接入 SQLite。
2. 建表：`users`、`sessions`、`projects`、`project_members`、`tasks`。
3. 实现登录接口。
4. 实现 `/api/me`。
5. 实现 `/api/me/projects`。
6. 实现 `/api/projects` 新建项目。
7. 服务端新建项目时创建工作区并执行 `git init`。

### 第二阶段：把聊天绑定项目

1. 新增 `/api/projects/{project_id}/chat`。
2. 新增 `/ws/projects/{project_id}`。
3. 旧接口保留一段时间兼容，但标记 deprecated。
4. AI agent 不再使用 `user_id__project_id`，改用数据库项目 workspace。
5. 每次 AI 修改后写入 `tasks`。

### 第三阶段：Web 端项目工作台

1. 登录页。
2. 项目列表页。
3. 新建项目弹窗。
4. 项目聊天页。
5. 构建历史页。

### 第四阶段：APK 端项目工作台

1. 登录页。
2. token 保存。
3. 项目列表页。
4. 新建项目页。
5. 项目聊天页使用 project_id。
6. 下载 APK 权限化。

### 第五阶段：管理后台

1. 用户管理。
2. 项目管理。
3. 成员授权。
4. 任务审计。
5. 公开下载链接。

## 13. 关键改造点对应当前代码

> 历史记录：Phase 5（commit `1840525`）已完成 legacy 网关清理。
> 旧文件 `client_gateway.rs`、`client_protocol.rs`，以及 `/ws`、`/api/chat`、
> `/ws/elon`、`/api/elon/download/:filename` 等旧入口均已删除。
> 当前所有客户端入口统一收敛到 `project_api.rs` 下的项目级 WS 与 HTTP 端点。

当前文件：

- `server/src/project_api.rs`
  - 项目级 WS（`/ws/user/:user_id/projects/:project_id`）与 HTTP（聊天 / 附件 / 下载）唯一入口。

- `server/src/user_api.rs`
  - 用户/项目/会话的 REST 管理接口。

- `server/src/types.rs`
  - `get_user_workspace` / `get_project_workspace` 已就绪。

- `server/src/admin.rs`
  - 管理后台 API，数据来自数据库。

- `server/src/web.rs`
  - 内置 Web 页（登录、项目列表、项目详情）。

- `android/app/src/main/kotlin/com/elon/app/MainActivity.kt`
  - 当前主界面需要先进入项目列表，再进入聊天。

- `android/app/src/main/kotlin/com/elon/app/SettingsActivity.kt`
  - 当前按 `user_id` 保存代理，后续应从 token 获取当前用户，也可以支持项目级代理设置。

## 14. 最终用户体验

用户重新登录后：

1. 打开 Web 或 APK。
2. 登录账号。
3. 自动看到自己有权限的项目列表。
4. 点击上次的项目。
5. 继续上一轮 AI 对话、构建或下载 APK。

用户新建项目时：

1. 点击新建项目。
2. 输入项目名。
3. 选择 Android 模板。
4. 服务端创建数据库记录、工作区和 Git 仓库。
5. 用户进入项目聊天页。

服务端保存历史时：

1. 每个项目一个 Git 仓库。
2. 每次 AI 修改一个 commit。
3. 每次构建一个 task 记录。
4. 每个 APK 都归属到项目和任务。

这套结构能支撑从“几个朋友试用”到“多人、多项目、可审计”的演进。

## 15. 数据保存、备份与恢复体系

用户、项目、代码、构建产物和 AI 对话记录都属于核心资产，不能只存在运行时内存或临时目录里。系统需要把数据分层保存，并且支持定期备份和可验证恢复。

### 15.1 需要保存的数据

| 数据类型 | 保存位置 | 重要性 | 说明 |
| --- | --- | --- | --- |
| 用户账号 | 数据库 | 高 | 登录、权限、状态 |
| 登录会话 | 数据库 | 中 | 可过期，可重建 |
| 项目信息 | 数据库 | 高 | 项目名、工作区、创建者、状态 |
| 项目成员权限 | 数据库 | 高 | 决定谁能看到和操作项目 |
| AI 任务记录 | 数据库 | 高 | 谁在什么时候让 AI 做了什么 |
| 聊天消息 | 数据库 | 中高 | 便于继续上下文和审计 |
| Git 仓库 | 文件系统或 Git 远端 | 极高 | 用户项目代码本体 |
| APK 构建产物 | 文件系统或对象存储 | 中高 | 可重新构建，但用户会依赖下载 |
| AI 代理配置 | 数据库或配置文件 | 高 | 包含模型、API 地址、密钥引用 |
| 密钥 | 加密存储或环境变量 | 极高 | 不应明文散落在项目目录 |
| 系统日志 | 日志文件 | 中 | 排查问题和审计 |

### 15.2 推荐保存结构

```text
/opt/elon/
  data/
    elon.db
    elon.db-wal
    elon.db-shm
  workspaces/
    projects/
      prj_xxxxx/
        .git/
        android/
        artifacts/
  backups/
    daily/
    weekly/
  logs/
    server.log
  config/
    agents.json
```

不要把数据库、项目工作区和日志放在程序发布目录里，避免升级服务端代码时误删数据。

### 15.3 数据库保存策略

初期使用 SQLite 时建议：

1. 开启 WAL 模式，提高并发读写和备份安全性。
2. 数据库文件放在固定数据目录，比如 `/opt/elon/data/elon.db`。
3. 所有表都带 `created_at`、`updated_at`。
4. 关键业务记录不物理删除，优先使用 `status = archived/deleted`。
5. 数据结构变更必须通过 migration 脚本，不手动改线上库。

SQLite 初始化时执行：

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

后续用户量增长后，可以迁移到 PostgreSQL。表结构设计保持通用，迁移成本会比较低。

### 15.4 Git 仓库保存策略

每个项目一个 Git 仓库，Git 仓库是项目代码的最终可信来源。

推荐策略：

1. 新建项目时优先使用用户提供的 `repo_url` clone/fetch；没有远端时执行 `git init` 和初始 commit。
2. 每次 AI 修改后必须 commit。
3. commit message 包含 task id 和用户 id。
4. 每天把所有项目仓库打包备份。
5. 条件允许时，把项目 Git 仓库推送到一个私有 Git 远端；只有已 push 的项目才能在其它 PC 节点可靠重建。

commit message 示例：

```text
feat: task tsk_123 by usr_456 - update login page
```

如果使用私有 Git 远端，目录可以这样映射：

```text
prj_8f3a92c1 -> git@example.com:elon-projects/prj_8f3a92c1.git
```

### 15.5 APK 产物保存策略

APK 可以重新构建，但用户实际安装会依赖某个构建产物，所以也应该保存。

建议新增 `artifacts` 表：

```sql
CREATE TABLE artifacts (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  task_id TEXT,
  file_name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);
```

产物目录：

```text
WORKSPACE_ROOT/projects/prj_xxxxx/artifacts/
  app-debug-20260522-120000.apk
```

服务端保存 APK 时应该计算 SHA-256，下载时可用于校验文件是否损坏。

### 15.6 聊天和任务上下文保存

为了让用户回到上一轮项目时能继续理解上下文，建议保存聊天消息。

```sql
CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  task_id TEXT,
  user_id TEXT,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);
```

用途：

1. 项目详情页展示历史对话。
2. AI 执行时可读取最近 N 条消息作为上下文。
3. 管理员排查用户反馈时有依据。

注意：不要无限塞入模型上下文。数据库可以长期保存，模型调用时只取最近或摘要后的内容。

### 15.7 密钥保存策略

API Key、管理员 token、签名证书密码不能散落在项目工作区里。

初期建议：

1. 服务端全局密钥放 `.env` 或服务器环境变量。
2. 用户自定义 API Key 存数据库前加密。
3. Android 签名密钥放服务器安全目录，不提交 Git。
4. 管理后台只显示脱敏后的 key。

推荐新增环境变量：

```text
DATA_DIR=/opt/elon/data
WORKSPACE_ROOT=/opt/elon/workspaces
BACKUP_DIR=/opt/elon/backups
SECRET_KEY=用于加密用户密钥的长随机字符串
```

### 15.8 备份策略

采用简单可执行的 3 层备份：

1. 本机每日备份。
2. 异地对象存储备份。
3. 每周完整归档。

每日备份内容：

```text
elon.db
agents.json
workspaces/projects/*/.git
workspaces/projects/*/artifacts
```

备份文件命名：

```text
backup-2026-05-22.tar.gz
backup-2026-05-22.sha256
```

备份完成后必须生成校验文件：

```bash
sha256sum backup-2026-05-22.tar.gz > backup-2026-05-22.sha256
```

保留策略：

| 备份类型 | 保留时间 |
| --- | --- |
| 每日备份 | 14 天 |
| 每周备份 | 8 周 |
| 每月备份 | 12 个月 |

### 15.9 恢复流程

备份没有验证恢复，就不算真正可用。

标准恢复流程：

1. 停止服务端。
2. 解压备份到新的数据目录。
3. 校验 SHA-256。
4. 恢复数据库。
5. 恢复 workspaces。
6. 启动服务端。
7. 登录测试账号。
8. 检查项目列表是否完整。
9. 打开一个历史项目。
10. 检查 Git log、任务记录、APK 下载是否正常。

每个月至少做一次恢复演练。

### 15.10 数据安全底线

1. 不允许客户端直接传路径。
2. 不允许用 user_id 拼接下载权限。
3. 所有项目访问都必须查 `project_members`。
4. 所有文件读写必须限制在项目 workspace 内。
5. 所有密钥必须脱敏显示。
6. 数据库和备份文件必须限制系统权限。
7. 删除用户或项目优先归档，不立即物理删除。
8. 线上升级前先备份数据库和 workspaces。

### 15.11 推荐落地顺序

1. 先把 `DATA_DIR`、`WORKSPACE_ROOT`、`BACKUP_DIR` 固定下来。
2. 接入 SQLite，并把用户、项目、任务写入数据库。
3. 新建项目时创建 Git 仓库。
4. 每次任务完成后写入 task、message、artifact、git commit。
5. 做一个 `scripts/backup.ps1` 或 `scripts/backup.sh`。
6. 做一个 `scripts/restore-check.sh`，用于测试备份能否恢复。
7. 再做异地备份。

系统真正可靠，不是因为“代码能跑”，而是因为账号、项目、代码、构建产物和备份之间能互相对得上。

## 16. 用户绑定自己的 GitHub 仓库做云备份

每个项目都可以绑定一个用户自己的 GitHub 仓库。服务端本地仍然保留项目工作区和 Git 仓库，GitHub 作为用户可见、可迁移、可恢复的云端备份。

### 16.1 推荐方案

有三种实现方式：

| 方案 | 适合阶段 | 优点 | 缺点 |
| --- | --- | --- | --- |
| 用户填写 GitHub 仓库 URL + Fine-grained PAT | MVP | 最快实现，适合少量朋友使用 | 用户要自己创建 token，体验略复杂 |
| GitHub OAuth App | 中期 | 用户体验好，可以代表用户访问 GitHub | 需要维护 OAuth 流程和 token 刷新 |
| GitHub App | 产品化 | 权限最小化，适合安装到指定仓库，安全性最好 | 开发和配置稍复杂 |

建议落地顺序：

1. 第一版用 **仓库 URL + Fine-grained PAT**。
2. 稳定后升级为 **GitHub App**。

### 16.2 数据模型

新增 `git_remotes` 表，用于保存项目绑定的 GitHub 仓库。

```sql
CREATE TABLE git_remotes (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  provider TEXT NOT NULL DEFAULT 'github',
  repo_full_name TEXT NOT NULL,
  repo_url TEXT NOT NULL,
  branch TEXT NOT NULL DEFAULT 'main',
  auth_type TEXT NOT NULL,
  credential_ref TEXT,
  github_installation_id TEXT,
  github_repository_id TEXT,
  last_push_commit TEXT,
  last_push_status TEXT,
  last_push_error TEXT,
  last_pushed_at TEXT,
  created_by TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (created_by) REFERENCES users(id)
);
```

如果先用 PAT，`auth_type = 'pat'`，`credential_ref` 指向加密后的密钥记录。

建议新增通用密钥表：

```sql
CREATE TABLE user_secrets (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  secret_type TEXT NOT NULL,
  encrypted_value TEXT NOT NULL,
  label TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(id)
);
```

注意：GitHub token 不能明文存库，必须用 `SECRET_KEY` 加密后保存。

### 16.3 用户绑定 GitHub 仓库流程

MVP 流程：

1. 用户在 GitHub 新建一个空仓库，例如 `friend/accounting-app`。
2. 用户生成 Fine-grained PAT，只授权该仓库。
3. 在 Web/APK 的项目设置里点击“绑定 GitHub 仓库”。
4. 输入仓库地址、默认分支、PAT。
5. 服务端校验 token 是否能访问仓库。
6. 服务端保存加密 token。
7. 服务端执行首次 push。

绑定接口：

```http
POST /api/projects/{project_id}/git-remotes
Authorization: Bearer <token>
```

请求：

```json
{
  "provider": "github",
  "repo_url": "https://github.com/friend/accounting-app.git",
  "branch": "main",
  "auth_type": "pat",
  "token": "github_pat_xxx"
}
```

响应：

```json
{
  "remote": {
    "id": "grm_123",
    "repo_full_name": "friend/accounting-app",
    "branch": "main",
    "last_push_status": "success"
  }
}
```

### 16.4 GitHub token 权限建议

Fine-grained PAT 只给最小权限：

| 权限 | 用途 |
| --- | --- |
| Repository access | 只选择绑定的仓库 |
| Contents: Read and write | `git push` 项目代码 |
| Metadata: Read-only | GitHub 默认需要 |

不需要给全部仓库权限，也不需要给删除仓库、管理组织等权限。

### 16.5 服务端如何 push 到用户 GitHub

项目本地仓库仍然在：

```text
WORKSPACE_ROOT/projects/prj_xxxxx/
```

绑定 GitHub 后，服务端给该项目配置 remote：

```bash
git remote add github-backup https://github.com/friend/accounting-app.git
git branch -M main
git push github-backup main
```

为了避免 token 写进 `.git/config`，不要把 token 拼进 remote URL 持久保存。推荐在执行 push 时临时注入凭据。

可选做法：

1. remote URL 永远保存无 token 版本：

```text
https://github.com/friend/accounting-app.git
```

2. push 时通过临时 credential helper 或环境变量提供 token。

伪代码：

```rust
fn push_to_github(workspace: &Path, repo_url: &str, branch: &str, token: &str) -> Result<()> {
    ensure_remote(workspace, "github-backup", repo_url)?;

    run("git", ["add", "."], workspace)?;
    commit_if_dirty(workspace, "chore: backup latest project state")?;

    run_with_git_token(
        "git",
        ["push", "github-backup", &format!("HEAD:{}", branch)],
        workspace,
        token,
    )?;

    Ok(())
}
```

### 16.6 自动备份时机

建议提供三种触发方式：

1. **任务完成后自动 push**
   - AI 修改并成功 commit 后，自动推送到 GitHub。

2. **用户手动点击同步**
   - 项目设置页提供“立即同步到 GitHub”按钮。

3. **每日定时同步**
   - 后台任务扫描已绑定 GitHub 的项目，把本地最新 commit 推上去。

推荐第一版实现：

```text
AI 任务完成
  -> git commit
  -> 如果项目绑定了 GitHub
  -> git push
  -> 更新 git_remotes.last_push_status
```

### 16.7 冲突处理

第一版建议把 GitHub 仓库定义为“备份仓库”，不要让用户直接在 GitHub 上改代码。

规则：

1. 服务端是主仓库。
2. GitHub 是备份镜像。
3. 如果 push 被 rejected，提示用户 GitHub 远端有新提交。
4. 管理后台显示冲突状态。
5. 初期不自动 merge，避免误覆盖。

冲突状态：

```text
last_push_status = rejected
last_push_error = "remote contains work that you do not have locally"
```

后续可以增加“从 GitHub 拉取并合并”的高级功能，但第一版建议只做单向 push。

### 16.8 从 GitHub 恢复项目

云备份的价值在于能恢复。

恢复场景：

1. 服务器项目目录损坏。
2. 用户换服务器。
3. 用户希望把 GitHub 仓库导入成新项目。

恢复流程：

```text
用户选择“从 GitHub 导入项目”
  -> 输入 repo_url / 授权 GitHub
  -> 服务端 git clone
  -> 创建 projects 记录
  -> 创建 project_members owner
  -> 扫描项目类型
  -> 进入项目详情
```

接口：

```http
POST /api/projects/import-github
Authorization: Bearer <token>
```

请求：

```json
{
  "repo_url": "https://github.com/friend/accounting-app.git",
  "name": "记账小工具",
  "auth_type": "pat",
  "token": "github_pat_xxx"
}
```

### 16.9 Web 端 UI

项目设置页增加一个 GitHub 备份区域：

```text
GitHub 云备份

状态：未绑定 / 已同步 / 同步失败 / 有冲突
仓库：friend/accounting-app
分支：main
最后同步：2026-05-22 12:30
最后 commit：abc123

[绑定 GitHub 仓库]
[立即同步]
[解绑]
```

绑定弹窗字段：

- GitHub 仓库 URL
- 分支名，默认 `main`
- GitHub Token
- “测试连接”按钮
- “绑定并首次同步”按钮

项目列表页也可以显示一个小状态：

```text
GitHub: 已同步
GitHub: 同步失败
GitHub: 未绑定
```

### 16.10 APK 端 UI

APK 端可以做简化版本：

项目设置页：

```text
GitHub 云备份

未绑定
[绑定仓库]

已绑定 friend/accounting-app
最后同步 12:30
[立即同步]
```

移动端输入 token 比较麻烦，所以推荐：

1. APK 端显示状态和“立即同步”。
2. 绑定 GitHub 优先在 Web 端完成。
3. APK 端可提供“复制 Web 设置链接”或二维码。

如果必须在 APK 端绑定，也可以提供 URL 和 token 输入框，但体验会差一些。

### 16.11 管理后台 UI

管理员项目详情页增加：

- GitHub 绑定状态。
- 最近同步时间。
- 最近同步错误。
- 手动重试同步。
- 解绑远端。

管理员不应该能看到用户完整 GitHub token，只能看到脱敏信息。

### 16.12 安全边界

1. GitHub token 必须加密保存。
2. token 不写入 `.git/config`。
3. 日志不能打印 token。
4. 用户只能绑定自己有权限的项目。
5. 项目成员中只有 owner 可以绑定或解绑 GitHub。
6. editor 可以触发同步，但不能查看或更换 token。
7. viewer 只能看同步状态。
8. push 失败不能自动 force push。
9. 如果要支持 force push，必须只允许 owner 手动确认。

### 16.13 产品化升级：GitHub App

当用户变多后，推荐从 PAT 升级到 GitHub App。

GitHub App 流程：

1. 用户点击“连接 GitHub”。
2. 跳转 GitHub 安装 App。
3. 用户选择允许访问的仓库。
4. GitHub 回调服务端。
5. 服务端保存 `installation_id` 和 repo 信息。
6. 每次同步时服务端用 GitHub App 私钥换取短期 installation token。
7. 用短期 token 执行 push。

这种方式更安全，因为：

- 用户不用手动创建 PAT。
- token 是短期的。
- 用户可以随时在 GitHub 取消安装。
- 权限可以限制在指定仓库。

### 16.14 推荐落地顺序

1. 新增 `git_remotes` 和 `user_secrets` 表。
2. Web 端项目设置增加 GitHub 绑定 UI。
3. 服务端实现绑定、测试连接、首次 push。
4. AI 任务完成后自动 push。
5. 项目列表显示同步状态。
6. 增加手动“立即同步”。
7. 增加从 GitHub 导入项目。
8. 稳定后升级为 GitHub App 授权。
