---
version_status: current
reviewed_at: 2026-09-28
---

# PWA 公开登录页捕获

## 用途与范围

`ui_capture_pwa_runtime` 默认把可见登录表单、登录路由及 HTTP 401/403 判为
`AUTHENTICATION_REQUIRED`。验收登录页自身时，可以显式声明 `expectedPage`。
它只证明声明的公开登录页面被真实浏览器捕获，不证明登录成功或账号可用。

此扩展需要包含本功能的节点版本。旧节点的严格输入 schema 会拒绝新字段；
源码推送不代表正在运行的节点已升级。

## 调用示例

保留原调用的 URL、视口、fixture 和真实源码证据，增加完整的声明：

```json
{
  "url": "http://127.0.0.1:55363/",
  "viewport": { "width": 411, "height": 842 },
  "waitFor": { "selector": "#loginView" },
  "fixtureProfile": "mobile-v2-light",
  "expectedPage": {
    "kind": "PUBLIC_LOGIN",
    "pageId": "main-login",
    "path": "/",
    "readySelector": "#loginView"
  },
  "evidence": {
    "sourceRevision": "<真实源码 revision>",
    "routeRevision": "<本轮路由 revision>"
  }
}
```

`pageId` 限 1–64 个英文字母、数字、下划线或连字符。`path` 必须等于请求 URL 的
精确路径，不含 query/fragment；完成捕获前再验证实际 origin 和 path。
哈希路由不能仅靠 `path` 区分页面，应以唯一 `readySelector` 约束目标；
本版本没有声明哈希路由身份的独立字段。

`readySelector` 必须命中恰好一个可见节点；该节点自身或子树中必须存在可见的
密码输入框，或 action 含 login/signin 的表单。不能用普通 dashboard 占位节点
冒充登录页。不符合此形态的无密码登录页暂不在支持范围。

## 保留的限制

- 不能携带显式或项目默认 `authProfile`，冲突在读取认证资料之前被拒绝。
- 项目仅配置 `authenticatedReadySelector` 而没有认证 profile 时，可验收公开页。
- 只允许 `waitFor`、`assertText`、`scrollIntoView`；禁止填表、点击、按键、
  更改勾选、选择选项和预览样式修改。
- HTTP 401/403 始终失败；原有 origin 白名单、请求拦截、URL 密钥拒绝及脱敏规则继续生效。
- 持久浏览器会绑定完整声明。复用时必须保持同一声明；不能直接将公开页会话改作认证后证据。

## 回执与排障

成功回执及磁盘 manifest 保存完整 `expectedPage`，认证模式仍为 `none`。
失败不保存成功截图 artifact，也不将公开页证据升级为认证成功。

| 诊断 | 含义 |
|---|---|
| `PUBLIC_LOGIN_AUTH_PROFILE_CONFLICT` | 存在显式或默认认证 profile |
| `PUBLIC_LOGIN_DECLARATION_INVALID` | pageId/path 声明不合法 |
| `PUBLIC_LOGIN_CAPTURE_ONLY` | 请求包含会改变页面或输入的步骤 |
| `PUBLIC_LOGIN_PAGE_MISMATCH` | 实际 origin/path 与声明不同 |
| `WAIT_TIMEOUT` | 唯一可见登录锚点等就绪条件未满足 |
| `AUTHENTICATION_REQUIRED` | HTTP 401/403，或未声明公开登录页的默认认证拦截 |
| `BROWSER_SESSION_BINDING_CHANGED` | 持久会话的用途声明等绑定发生变化 |

最小回归入口为 `scripts/validate-rust.ps1 -- test --manifest-path
server/tests/pwa-capture-harness/Cargo.toml -- --test-threads=2`，外层使用项目日志执行器。
测试导入生产捕获模块，使用隔离无头浏览器和本机 HTTP fixture；不重启节点或操作 Android。
