# Project AI / Route C SDK MVP

最后更新：2026-06-25

## 定位

本 MVP 先把子项目 AI 接入抽象成一套通用 Project AI SDK：Route A/B/C 都使用同一套工具 manifest、`actions` 和 `tool_results` 协议。

MVP 版本先不做账号、计费和安全审批；正式版再接 `accounts/session` token、额度、审批和更细的隐私脱敏。

## 三条路线

| 路线 | 模型调用位置 | 工具执行位置 | MVP 用法 |
|---|---|---|---|
| Route A | 用户本机已有 Codex / Copilot / Claude CLI | 用户本机 SDK/CLI | 子项目把工具协议和远程源码能力交给本机 AI CLI |
| Route B | 用户本机自己的 API key | 用户本机 SDK/CLI | 子项目自己跑模型调用，但复用一龙工具协议 |
| Route C | 一龙服务器 | 用户本机 SDK/CLI | 子项目把对话发到一龙服务器，SDK 自动执行工具循环 |

第一版服务端接口仍复用 `/route-c/chat`，但请求和响应已经带 `runtime_route`，子项目可以先接 UI 选择器。

## 与 `mvp-chat` 的区别

| 能力 | `mvp-chat` | Project AI SDK MVP |
|---|---|---|
| 模型位置 | 一龙服务器 | Route A/B/C 可选 |
| 本地工具 | 服务器返回 `suggested_tools`，客户端自己决定 | 服务器返回 `actions`，SDK 执行后自动回传 `tool_results` |
| 工具循环 | 客户端页面手动编排 | SDK 自动循环 |
| 审批 | 客户端自定 | MVP 暂不内置，正式版补 |
| 适合场景 | 简单 AI 问答 | AI + 本地诊断/文件/命令/业务工具 + 远程源码查询 |

## 服务端接口

```http
POST /api/external/apps/{app_id}/route-c/chat
```

启用条件：

- `ELON_EXTERNAL_APP_ROUTE_C_SDK_ENABLED=true`
- 或复用当前 MVP 开关：`ELON_EXTERNAL_APP_MVP_CHAT_ENABLED=true`

请求示例：

```json
{
  "conversation_id": "local-session-1",
  "message": "为什么我连上节点后 Google 打不开？",
  "history": [],
  "runtime_route": "route_c",
  "client": {
    "platform": "windows",
    "app_version": "1.1.230"
  },
  "local_context": {
    "source": "bb64a_windows_mcp",
    "debug_port": 17899
  },
  "runtime_permission": "danger_full_access",
  "tool_manifest": {
    "schema": "elon.route_c.tool_manifest.v0",
    "tools": [
      { "name": "bb64a_doctor", "description": "采集代理、路由、节点和日志诊断快照" },
      { "name": "test_google", "description": "通过当前代理链路测试 Google" },
      { "name": "detect_conflicts", "description": "检测本机代理冲突" },
      { "name": "run_command", "description": "执行本机 cmd/powershell 命令", "permission": "danger_full_access", "dangerous": true },
      { "name": "read_file", "description": "读取本机文本文件", "permission": "danger_full_access", "dangerous": true },
      { "name": "write_file", "description": "写入本机文本文件", "permission": "danger_full_access", "dangerous": true },
      { "name": "remote_source_ask", "description": "请求远程源码节点查阅子项目源码" },
      { "name": "create_feedback_post", "description": "创建子项目需求频道反馈帖子" }
    ]
  },
  "tool_results": []
}
```

响应示例：

```json
{
  "ok": true,
  "schema": "external_app.route_c_chat.v0",
  "conversation_id": "local-session-1",
  "reply": "我需要先测一下当前代理链路是否能访问 Google。",
  "done": false,
  "actions": [
    {
      "id": "tool_1",
      "tool": "test_google",
      "args": {},
      "reason": "验证当前代理链路是否可用",
      "dangerous": false
    }
  ],
  "project_ai": {
    "schema": "elon.project_ai_sdk.mvp.v0",
    "runtime_route": "route_c",
    "supported_routes": ["route_a", "route_b", "route_c"],
    "local_execution": "external_app_sdk",
    "remote_source_tools": ["remote_source_search", "remote_source_read_file", "remote_source_ask"],
    "feedback_tools": ["create_feedback_post"]
  },
  "route_c": {
    "mode": "server_model_local_tools",
    "runtime_route": "route_c",
    "local_execution": "external_app_sdk",
    "approval": "client_managed_mvp_disabled",
    "tool_filter": "manifest_allowlist"
  }
}
```

## JS SDK

浏览器或 WebView 可直接引用：

```html
<script src="https://your-elon-server/assets/elon_route_c_sdk.js"></script>
```

函数工具 provider：

```js
const provider = ElonRouteCSDK.createFunctionToolProvider({
  tools: {
    async test_google() {
      return await window.bb64aApi.testGoogle();
    },
    async detect_conflicts() {
      return await window.bb64aApi.detectConflicts();
    },
    async bb64a_doctor() {
      return await window.bb64aApi.doctor();
    },
  },
  async collectContext() {
    return await window.bb64aApi.lightContext();
  },
});

const client = new ElonRouteCSDK.ElonRouteCClient({
  appId: "bb64a",
  serverBaseUrl: "http://43.139.149.158:8080",
  toolProvider: provider,
  client: {
    platform: "windows",
    app_version: "1.1.230",
  },
});

const answer = await client.ask("为什么我连上节点后 Google 打不开？");
console.log(answer.reply);
```

通用 Project AI provider：

```js
const provider = ElonRouteCSDK.createProjectAiToolProvider({
  route: "route_c",
  appId: "bb64a",
  async executeCommand(args) {
    return await window.bb64aLocalCli.runCommand(args);
  },
  async readFile(args) {
    return await window.bb64aLocalCli.readTextFile(args.path);
  },
  async writeFile(args) {
    return await window.bb64aLocalCli.writeTextFile(args.path, args.content);
  },
  async listDir(args) {
    return await window.bb64aLocalCli.listDir(args.path || ".");
  },
  remoteSource: {
    async ask(args) {
      return await window.bb64aApi.askSourceNode(args);
    },
    async search(args) {
      return await window.bb64aApi.searchSourceNode(args);
    },
    async readFile(args) {
      return await window.bb64aApi.readSourceFile(args);
    },
  },
  feedback: {
    async createPost(args) {
      return await window.bb64aApi.createDemandPost(args);
    },
  },
  tools: {
    async bb64a_doctor() {
      return await window.bb64aApi.doctor();
    },
  },
});

const client = new ElonRouteCSDK.ElonRouteCClient({
  appId: "bb64a",
  serverBaseUrl: "http://43.139.149.158:8080",
  route: "route_c",
  toolProvider: provider,
});

const answer = await client.ask("连上节点以后浏览器打不开，帮我排查");
```

标准远程源码和反馈工具名：

| 工具 | 子项目实现 |
|---|---|
| `remote_source_search` | 让远程源码节点按关键词、路径或符号搜索 |
| `remote_source_read_file` | 让远程源码节点读取指定文件或片段 |
| `remote_source_ask` | 让远程源码节点围绕用户问题做源码判断 |
| `create_feedback_post` | 在子项目需求频道创建问题总结帖 |

`create_feedback_post` 建议接收：`title`、`user_problem`、`local_evidence`、`source_findings`、`issue_type`、`suggested_next_step`。

本地 HTTP provider：

```js
const provider = ElonRouteCSDK.createHttpToolProvider({
  baseUrl: "http://127.0.0.1:17899",
  tools: {
    bb64a_doctor: {
      method: "POST",
      path: "/debug/doctor",
      body: () => ({
        include_os_snapshot: false,
        include_network_tests: true,
        include_sensitive_subscriptions: false,
      }),
    },
    test_google: {
      method: "POST",
      path: "/debug/test/google",
    },
    detect_conflicts: {
      method: "GET",
      path: "/debug/conflicts",
    },
  },
});
```

完整本机 CLI provider：

```js
const provider = ElonRouteCSDK.createDangerFullAccessToolProvider({
  async executeCommand(args) {
    // Win 端实现：调用本地 Rust/Tauri/Node 桥接层执行命令。
    // args: { program, args, command, shell, cwd }
    return await window.bb64aLocalCli.runCommand(args);
  },
  async readFile(args) {
    return await window.bb64aLocalCli.readTextFile(args.path);
  },
  async writeFile(args) {
    return await window.bb64aLocalCli.writeTextFile(args.path, args.content);
  },
  async listDir(args) {
    return await window.bb64aLocalCli.listDir(args.path || ".");
  },
  async collectContext() {
    return await window.bb64aLocalCli.lightSystemContext();
  },
});

const client = new ElonRouteCSDK.ElonRouteCClient({
  appId: "bb64a",
  serverBaseUrl: "http://43.139.149.158:8080",
  toolProvider: provider,
  runtimePermission: "danger_full_access",
  client: {
    platform: "windows",
    app_version: "1.1.230",
  },
});

const answer = await client.ask("帮我检查为什么系统代理开了但网页打不开");
```

## 子项目接入责任

MVP 阶段，子项目需要自己保证：

- 只在确认要让 AI 接管本机排障时注册 `createDangerFullAccessToolProvider`。
- 不把订阅 URL、token、节点密码、隐私日志直接放进 `local_context` 或 `tool_results`。
- 普通问答只注册业务诊断工具；完整 CLI 问答显式声明 `runtime_permission=danger_full_access`。
- Route A/B 可只复用 `createProjectAiToolProvider` 的 manifest 和工具回调，模型调用由子项目本机完成。
- 远程源码节点不要直接泄露密钥、生产配置或用户隐私；只返回和问题相关的源码判断。
- 工具执行失败也要返回错误结果，交给 AI 继续解释。

正式版会在 SDK 内补：

- 本地审批弹窗。
- 工具 scope。
- 危险工具分级。
- token/订阅/路径脱敏。
- 审计日志和用量计费。
