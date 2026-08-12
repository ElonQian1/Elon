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
- MCP 必须报告官方 `web` 模式且 Activity 位于前台；一龙工具栏、状态栏和模式栏不应继续可见。
- 会话、功能、模型、工具、听写、输入和发送控件的稳定 ADB 选择器。
- 官网会话列表的真实请求，以及 `chatgpt_get_context` 当前游标重放和下一页读取。

上下文 smoke 只比较 schema、revision、offset 和游标，不把消息正文、Cookie 或凭证写入报告。

## 显式发送回归

只有在已获得可发送测试消息的授权时才使用：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-chatgpt-web-apk.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -SendProbe
```

`-SendProbe` 会新建会话，发送唯一 ASCII 标记，然后等待官网回复。验收同时要求：

- `new_conversation` 和 `send_prompt` 的设备事件时间新于操作前状态。
- 新建会话失败时立即停止，不允许把探针发送到已有会话。
- `send_input` 返回的 `request_id` 必须在 `ui_state.command_requests` 中终态成功；后续命令覆盖 `last_command` 不影响验收。
- 回复已停止流式生成，且最后一条消息角色为 `assistant`。
- 回复包含测试标记；仅对 Markdown 的 `\_` / `\-` 转义做归一化。
- MCP 能读回会话 URL、模型和消息数。

可以传入自定义标记：

```powershell
-SendProbe -ProbeMarker "ELON-CHATGPT-WEB-SMOKE-20260810"
```

如果不带 `-SendProbe` 却传入 `-ProbeMarker`，脚本会立即失败，防止默认只读模式意外发送消息。

## 附件选择与删除验收

附件完整链路需要真人在 Android 系统选择器中选择测试文件，因此拆成三个可中断阶段。阶段之间的检查点只保存设备与会话的 SHA-256 绑定、版本和计数，不保存会话 URL、消息正文或文件名。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-attachment-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" -Phase Prepare

powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-attachment-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" -Phase OpenPicker

# 此时由用户在系统选择器中选一个非敏感测试文件，返回 ChatGPT 后继续：
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-attachment-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" -Phase VerifyAndRemove
```

`Prepare` 要求当前输入为空、没有附件且未生成回复；`OpenPicker` 只按 `attachment_file` 语义打开系统选择器；`VerifyAndRemove` 要求 adapter 识别到唯一 `ready` 附件，再通过 `chatgpt_remove_attachment` 删除并确认附件数恢复为 0。整个流程不创建会话、不发送消息、不清 Cookie 或应用数据。检查点默认位于被忽略的 `.ai-tmp/chatgpt-web-attachment-lifecycle.json`，12 小时后失效。

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
