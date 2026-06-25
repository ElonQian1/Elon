# Route C SDK MVP

最后更新：2026-06-25

## 定位

Route C SDK 的目标是把“一龙服务器调用模型、子项目本机执行工具”做成可复用接入层。

MVP 版本先不做账号、计费和安全审批；正式版再接 `accounts/session` token、额度、审批和更细的隐私脱敏。

## 与 `mvp-chat` 的区别

| 能力 | `mvp-chat` | Route C SDK MVP |
|---|---|---|
| 模型位置 | 一龙服务器 | 一龙服务器 |
| 本地工具 | 服务器返回 `suggested_tools`，客户端自己决定 | 服务器返回 `actions`，SDK 执行后自动回传 `tool_results` |
| 工具循环 | 客户端页面手动编排 | SDK 自动循环 |
| 审批 | 客户端自定 | MVP 暂不内置，正式版补 |
| 适合场景 | 简单 AI 问答 | AI + 本地诊断/文件/命令/业务工具 |

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
  "client": {
    "platform": "windows",
    "app_version": "1.1.230"
  },
  "local_context": {
    "source": "bb64a_windows_mcp",
    "debug_port": 17899
  },
  "tool_manifest": {
    "schema": "elon.route_c.tool_manifest.v0",
    "tools": [
      { "name": "bb64a_doctor", "description": "采集代理、路由、节点和日志诊断快照" },
      { "name": "test_google", "description": "通过当前代理链路测试 Google" },
      { "name": "detect_conflicts", "description": "检测本机代理冲突" }
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
  "route_c": {
    "mode": "server_model_local_tools",
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

## 子项目接入责任

MVP 阶段，子项目需要自己保证：

- 只注册当前用户可接受的本地工具。
- 不把订阅 URL、token、节点密码、隐私日志直接放进 `local_context` 或 `tool_results`。
- 不把危险动作注册进工具 manifest。
- 工具执行失败也要返回错误结果，交给 AI 继续解释。

正式版会在 SDK 内补：

- 本地审批弹窗。
- 工具 scope。
- 危险工具分级。
- token/订阅/路径脱敏。
- 审计日志和用量计费。
