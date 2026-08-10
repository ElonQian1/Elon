# ChatGPT Web APK Smoke

`scripts/smoke-chatgpt-web-apk.ps1` 用于验证 APK 内的「一龙 AI -> ChatGPT 网页模式」。它通过 APK MCP 和 UIAutomator 检查真机界面，不导出 Cookie、账号或官网请求凭证。

## 默认只读

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-chatgpt-web-apk.ps1 `
  -DeviceSerial "192.168.31.171:5555"
```

只读模式检查：

- ChatGPT Web Activity、bridge、登录态和输入框状态。
- 能力矩阵中的阻塞缺口、未知能力和未知语义。
- 会话、功能、模型、工具、听写、输入和发送控件的稳定 ADB 选择器。
- 官网会话列表的真实请求和 MCP 分页读取。

## 显式发送回归

只有在已获得可发送测试消息的授权时才使用：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-chatgpt-web-apk.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -SendProbe
```

`-SendProbe` 会新建会话，发送唯一 ASCII 标记，然后等待官网回复。验收同时要求：

- `new_conversation` 和 `send_prompt` 的设备事件时间新于操作前状态。
- 回复已停止流式生成，且最后一条消息角色为 `assistant`。
- 回复包含测试标记；仅对 Markdown 的 `\_` / `\-` 转义做归一化。
- MCP 能读回会话 URL、模型和消息数。

可以传入自定义标记：

```powershell
-SendProbe -ProbeMarker "ELON-CHATGPT-WEB-SMOKE-20260810"
```

如果不带 `-SendProbe` 却传入 `-ProbeMarker`，脚本会立即失败，防止默认只读模式意外发送消息。

## 通过标志

成功时最后输出：

```text
CHATGPT_WEB_SMOKE_STATUS=passed mode=read_only
```

或：

```text
CHATGPT_WEB_SMOKE_STATUS=passed mode=send_probe
```

任何检查失败、未登录、bridge 未就绪、未知语义或等待超时都必须保持非零退出码。
