---
capability_id: android_chatgpt_webview_proxy_prepare_fail_open_v1
implementation_status: completed
verification_status: device_verified
production_default: true
repeat_research: not_required
---

# ChatGPT WebView 代理准备恢复

## 能力状态

- source commit: `7c16ba21abbfcae4d7c6c5e8769eee996250af74`
- verified APK: `v1.1.1240 (1250)`

## 问题与实现

部分 Android WebView 版本会让 `ProxyController.clearProxyOverride()` 长时间不回调。旧流程把首次 `loadUrl()` 严格串在该回调之后，导致原生聊天已打开但页面代数长期为 0，表现为一直连接、无法恢复输入框。

`ChatGptWebProxyPrepareGate` 现在把平台回调收敛为一次性结果：正常回调立即继续；超过 750ms 时按当前手机网络状态放行；迟到的平台回调被忽略，不会重复加载。官方 WebView、系统 VPN、保存的手动代理、Cookie、登录态和错误回退均保留。

## 验证证据

在小米真机上无损安装正式 APK，保留 Cookie 和应用数据，仅强停应用进程后重新进入原生 ChatGPT 聊天：

- 约 1068ms：`page_generation=2`、`bridge_state=connecting`。
- 约 2169ms：`bridge_state=ready`、`authenticated=true`、`composer_ready=true`。
- 后台停留 15 秒后返回约 1902ms：仍为 `ready`、已认证且输入框就绪。
- MCP 启动过程只保留一个 `MainActivity` 实例。

验收未发送消息、未读取会话正文、未清 Cookie 或应用数据。除非出现当前版本的明确回归证据，不再重复研究该能力。
