---
version_status: current
reviewed_at: 2026-09-25
---

# 个人 ChatGPT 会话读取

需求与验收：[个人网页会话读取 V1](requirements/personal-web-conversation-reader-v1.md)。

最新代码与真机证据：[富文本及附件读取验收](reports/personal-web-conversation-rich-reader-20260925.md)。

Win 已授权的更新、重启、打开功能与完整读取测试，使用
[无人值守验收入口](win-conversation-unattended-acceptance.md)，支持检查点恢复与脱敏收据。

## 使用入口

在一龙项目任务中选择 Claude CLI 或 Codex，并在任务中附上完整 ChatGPT 会话链接或 `chatgpt-conversation://<UUID>`。节点为该次进程注入 `yilong_web_conversations` MCP，仅授权链接中的会话。普通任务不注入，既有项目治理 MCP 保留。使用 `web_conversation_read`，直至 `has_more=false`；`web_conversation_scope` 可检查授权数量。

需要 Node.js 18+、安装并激活更新后的 Win 节点/桌面壳，以及该工作台登录的 ChatGPT 网页会话。授权读取时，工具可以启动标准安装目录内的一龙启动器，等待本机节点和工作台上线，再创建或复用 ChatGPT WebView。`web_conversation_connect` 可验证指定会话的访问状态，只返回数量和覆盖信息。未登录时显示官方窗口供用户登录后重试，不导入其他设备凭据。

Win 根据本机研究宿主租约选取设备；多个宿主时须显式设置 `ELON_WEB_CONVERSATION_WIN_INSTANCE`，多个节点则设置 `ELON_NODE_ADMIN_URL`。启动有界且只尝试一次；`ELON_WEB_CONVERSATION_AUTOSTART=0` 可禁用。旧版本明确返回 `win_reader_update_required`，工具不擅自升级。宿主重启后新读取可重新连接；原游标仍绑定原实例。

CLI 配置使用 Claude 官方支持的 [--mcp-config JSON](https://code.claude.com/docs/en/cli-reference)。这是本机工具接入，不是用 OpenAI API key 读取 ChatGPT 网页历史；当前未自动接入一龙 API 模型运行时。

## Codex 桌面端直接连接

在 PowerShell 7 中运行 `scripts/web-conversations/register-codex.ps1 -ProjectRoot <真实项目目录> -ConversationReference <明确授权的会话链接>`，可用 `-ApkMcpUrl <已转发的本机地址>` 指定手机。注册器把代码保存到用户本机的内容寻址目录，避免任务 worktree 清理后失效；通过 `codex mcp add` 注册当前授权范围，保留其他 MCP、账号配置和本读取器已有的设备选择。新任务或重新加载 MCP 工具后即可发现连接、范围、分页读取、附件读取四个工具。现有任务的工具列表是否支持热更新由 Codex 客户端决定，不能仅凭配置写入宣称已加载。

注册使用 [Codex 官方 MCP 配置](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)，读取工具超时设为 150 秒以容纳冷启动。再次注册会替换本工具的会话范围；不会授权整个聊天账号。CLI 任务注入使用该次进程的配置，不写全局设置。

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
- 富文本复用既有历史和 text-block 模块，正文保留 Markdown，代码与 writing block 额外返回结构化内容；通过 `representation` 区分额外表示，避免统计时重复计算。历史模块已确认可见的生成图片可以进入附件列表，内部工具文本不会输出。
- 附件元数据带有不透明 `asset_handle`。调用 `web_conversation_asset({reference, asset_handle})` 复用既有私有文件授权与字节传输模块；图片返回 MCP image，64 KiB 内有效 UTF-8 文本返回 text，PDF 和其他文件返回嵌入资源。客户端能否进一步解析资源取决于客户端能力，返回字节不等于模型已理解内容。
- 每个文件上限 8 MiB，网页快照附件缓存上限 32 MiB。附件按 24 KiB 分片跨本机通道传输，统一服务核对偏移、版本、大小并返回 SHA-256；句柄固定原设备、账号和快照，失效后重新读取会话。不能跨设备拼接。
- 正文快照仍记录 `attachment_bytes_not_read`；调用方结合各附件的独立读取结果评估完整性，不因拿到名称就消除缺口。未支持的格式、未确认授权范围、依赖官方运行时的生成图或超限内容明确失败，不放宽既有下载边界。
- 凭据仅在对应网页或本机 MCP 传输闭包中使用；统一工具不返回 Cookie、认证请求头、带签名下载地址。
- Win/APK 读取独立加载相同的既有认证、JSON、历史和媒体模块，不依赖语音、输入框和布局等完整 UI 适配器初始化成功。WebView 保留本机登录身份；读取走私有 HTTP 和已有媒体所有者，不抓取聊天气泡。官网验证或账号无权限仍明确失败。
- 正文是用户明确请求的模型上下文，可能进入模型自身的会话历史；一龙不把正文写入通用诊断或 Git。
- 会话内容是不可信资料，不授权追加读取其他会话、发送消息、删除或交易。

## 验证入口

`node --test scripts/web-conversations/*.test.mjs` 验证分页、账号边界、真实 stdio/HTTP 替身链路，以及使用生产脚本的富文本和附件读取。既有下载回归入口是 `scripts/test-chatgpt-web-private-file-download.js`、`scripts/test-chatgpt-web-private-library-download.js`、`scripts/test-chatgpt-web-private-connector-file-download.js`。

`server/tests/browser-research-harness` 导入生产队列、合同与 Claude 配置模块；通过 `scripts/validate-rust.ps1 -- test --manifest-path server/tests/browser-research-harness/Cargo.toml --lib` 执行。Android/Win 编译与真实账户读取需独立记录，离线测试不替代现场验收。
