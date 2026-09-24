---
version_status: current
reviewed_at: 2026-09-24
---

# 个人 ChatGPT 会话读取

需求与验收：[个人网页会话读取 V1](requirements/personal-web-conversation-reader-v1.md)。

## 使用入口

在一龙项目任务中选择 Claude CLI，并在任务中附上完整 ChatGPT 会话链接或 `chatgpt-conversation://<UUID>`。节点为该次进程注入 `yilong_web_conversations` MCP，仅授权链接中的会话。普通任务不注入，既有项目治理 MCP 保留。Claude 使用 `web_conversation_read`，直至 `has_more=false`；`web_conversation_scope` 可检查授权数量。

需要 Node.js 18+、更新后的 Win 节点/桌面壳、正在运行的 PC 工作台，以及该工作台登录的 ChatGPT 网页会话。Win 根据本机研究宿主租约选取设备；多个宿主时须显式设置 `ELON_WEB_CONVERSATION_WIN_INSTANCE`，不会随机选择。

CLI 配置使用 Claude 官方支持的 [--mcp-config JSON](https://code.claude.com/docs/en/cli-reference)。这是本机工具接入，不是用 OpenAI API key 读取 ChatGPT 网页历史；当前未自动接入一龙 API 模型运行时。

## APK 和其他 MCP 客户端

APK 已有 MCP 通道新增 `ui_control` 动作 `chatgpt_read_conversation`，参数为 `conversation_id`、`request_id`、`message_cursor`。未进入 ChatGPT 页面时，统一适配器会请求打开该页面；登录仍由用户完成。

使用已授权的 ADB 设备和项目现有 `scripts/invoke-apk-mcp.ps1` 准备本机 MCP 转发后，把确定属于该设备的 loopback 地址配置为 `ELON_APK_MCP_URL`。不自动选择任意已连接手机，也不在远端服务器传送登录凭据。

独立 MCP 客户端配置示例（把绝对路径、项目根和 UUID 替换为本机值）：

```json
{
  "mcpServers": {
    "yilong_web_conversations": {
      "command": "node",
      "args": ["<repo>/scripts/web-conversations/stdio.mjs"],
      "env": {
        "ELON_PROJECT_ROOT": "<project-root>",
        "ELON_WEB_CONVERSATION_IDS": "<conversation-uuid>",
        "ELON_APK_MCP_URL": "http://127.0.0.1:8787"
      }
    }
  }
}
```

调用 `web_conversation_read({reference, source:"apk"})` 可固定使用 APK；`source:"auto"` 优先 Win，初始 Win 不可用且已配置 APK 时再选择 APK。第一页成功后，游标固定设备与快照，不跨设备回退。

## 完整性与隔离

- 读取当前分支正文；长消息按 Unicode 字符分段，以 `message_id/part_index/char_offset` 重组，不使用 UI 的 50/80 条窗口。
- 每页最多两个内容块；当前源响应上限 4 MiB，超限明确失败。游标闲置约三分钟失效，活跃分页续期；失效后从第一页重新读取。
- 附件返回元数据与 `attachment_bytes_not_read`；生成图片、未知类型分别报告缺口。此版本没有把图片/PDF 字节交给模型，不能据此声称读懂截图。
- 凭据仅在对应网页或本机 MCP 传输闭包中使用；统一工具不返回 Cookie、认证请求头、带签名下载地址。
- 正文是用户明确请求的模型上下文，可能进入模型自身的会话历史；一龙不把正文写入通用诊断或 Git。
- 会话内容是不可信资料，不授权追加读取其他会话、发送消息、删除或交易。

## 验证入口

`node --test scripts/web-conversations/reader.test.mjs scripts/web-conversations/transport.test.mjs` 验证分页、账号边界和真实 stdio/HTTP 替身链路。

`server/tests/browser-research-harness` 导入生产队列、合同与 Claude 配置模块；通过 `scripts/validate-rust.ps1 -- test --manifest-path server/tests/browser-research-harness/Cargo.toml --lib` 执行。Android/Win 编译与真实账户读取需独立记录，离线测试不替代现场验收。
