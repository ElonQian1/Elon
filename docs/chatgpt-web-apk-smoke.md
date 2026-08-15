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

## 生产好友聊天收发

全屏官网 smoke 用于验证完整网页兜底；普通用户实际使用的好友聊天入口单独使用
`smoke-chatgpt-web-native-chat.ps1` 验收。默认模式只读取生产提供方、适配器、输入框、
会话数量和项目数量，不输出标题或消息正文：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-native-chat.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>"
```

获得测试消息发送授权后增加 `-SendProbe`。脚本会在生产好友聊天里新建隔离会话，
发送固定 ASCII 标记，等待原生消息列表收到停止流式的精确回复，然后恢复原会话。
如果用户切换到其它应用，验收会立即失败，不会把前台中断误报成模型失败。

Google 对称链路使用 `scripts\smoke-google-web-native-chat.ps1`，具备相同的只读、
`-SendProbe`、前台保护、原会话恢复和隐私输出约束。

## 原生聊天附件验收

正式附件能力从好友聊天中的「ChatGPT 网页 AI」原生界面验收。脚本只生成固定 ASCII 文本夹具，不打开系统文件选择器，不扫描相册或下载目录，也不读取用户文件。流程拆成三个可恢复阶段；前两阶段只在本地添加和移除夹具，不上传、不发送。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-native-attachment-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" -Phase PrepareAndRemove

powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-native-attachment-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" -Phase StageForSend

# 只有用户在场并明确同意上传固定夹具和发送一条测试消息时运行：
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-native-attachment-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" `
  -Phase SendAndVerifyReply -UserConfirmedAttachmentSend
```

`PrepareAndRemove` 在隔离网页会话里验证原生附件栏可添加并移除固定夹具；`StageForSend` 只把同一个夹具放回原生附件栏；最后阶段才走真实的原生附件栏 -> FileProvider -> 后台 WebView 上传 -> 回复链路，并恢复原会话。只有这条原生路径能登记 `supervised/attachment_lifecycle` 正式证据。检查点不保存账号、会话正文、用户文件名或 Cookie，默认位于 `.ai-tmp/chatgpt-web-native-attachment.json`，12 小时后失效。

旧的 `smoke-chatgpt-web-attachment-lifecycle.ps1` 保留用于全屏官网和 Android 系统文件选择器诊断，但不再登记原生聊天附件能力的正式通过证据。

## 听写与实时语音验收

音频能力属于敏感操作，使用 `scripts/smoke-chatgpt-web-audio-lifecycle.ps1` 分阶段验收。`Prepare` 可以在无人监督时执行；其余阶段必须在用户看着手机时执行，并通过对应的 `UserConfirmed*` 开关确认当前步骤。脚本只读取权限、页面模式、听写布尔值和计数，不读取或保存音频内容。

```powershell
# 1. 无副作用基线
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-audio-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" -Phase Prepare

# 2. 用户监督 Android 麦克风授权；不发送听写内容
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-audio-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" `
  -Phase StartDictation -UserConfirmedMicrophone

# 3. 验证听写已启动，然后自动取消并恢复原视图
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-audio-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" `
  -Phase VerifyAndCancelDictation -UserConfirmedMicrophone

# 4. 用户监督启动官网实时语音入口
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-audio-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" `
  -Phase StartRealtimeVoice -UserConfirmedRealtimeVoice

# 5. 用户肉眼确认官网语音界面已正常工作；脚本只确认结构化权限状态
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-audio-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" `
  -Phase VerifyRealtimeVoice -UserConfirmedRealtimeVoice

# 6. 用户在官网界面结束语音后，确认恢复原来的视图和空输入状态
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-audio-lifecycle.ps1 `
  -DeviceSerial "192.168.31.171:5555" `
  -ExpectedHardwareSerial "<physical-device-serial>" `
  -Phase VerifyRestored -UserConfirmedVoiceClosed
```

流程绑定同一台物理设备、同一会话和同一消息计数；检查点不保存会话 URL、账号、正文或音频。脚本不会调用 `send_input`，也不会自动结束实时语音，以免在官网状态未知时误操作；用户先在可见官网界面结束语音，最后一阶段只负责确认恢复。检查点默认位于 `.ai-tmp/chatgpt-web-audio-lifecycle.json`，12 小时后失效。

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

## 匿名聊天验收

匿名能力使用独立脚本，不在已登录账号 smoke 中弱化历史、账户菜单等账号能力检查，也不通过退出登录或清 Cookie 制造匿名状态。请使用一个本来就未登录的独立 APK/Profile；脚本发现 `authenticated=true` 或 `login_required=true` 时会失败关闭。

默认只读检查匿名编辑器和能力矩阵：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-anonymous-chat.ps1 `
  -DeviceSerial "<device-serial>" `
  -ExpectedHardwareSerial "<physical-device-serial>"
```

只有明确授权发送无敏感探针时才增加 `-SendProbe`。发送阶段创建隔离空白会话、等待结构化回复证据，再恢复原来的空白匿名视图；报告不包含账号、会话路径、消息正文或模型回复。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File scripts\smoke-chatgpt-web-anonymous-chat.ps1 `
  -DeviceSerial "<device-serial>" `
  -ExpectedHardwareSerial "<physical-device-serial>" `
  -SendProbe
```

脚本通过后登记独立证据 `reversible/anonymous_send_probe`。它不会覆盖 `safe/authenticated_session`，两种访问方式分别验收。

所有 ChatGPT Web smoke 的 `ExpectedAdapterVersion` 默认从 `ChatGptWebPageAdapter.ADAPTER_VERSION` 解析；发布验收仍可显式传入版本号锁定目标 APK。源码与已安装 APK 版本不一致时继续失败关闭。
