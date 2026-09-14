---
version_status: current
reviewed_at: 2026-09-15
implementation_status: implemented
---

# 群聊 @ 选择与后台通知

## 用户操作

- 在群聊中长按对方头像，将 `@名字 ` 插入当前光标或选区，打开文字输入与键盘。
- 在开头、空白或标点后输入 `@` / `＠`，打开“选择提醒的人”。邮件地址不触发。
- 可搜索群友名字、EL 或“群 AI”；切换多选后跨搜索保留已选成员，完成后一次插入。
- 取消保留原草稿；加载失败提供重试。切换群或草稿已经改变时，不向新草稿写入旧选择。
- AI 目录目前只返回社会群聊实际支持的 EL，发送 `@EL` 沿用现有 AI 回复服务。

当前发送协议仍为文本提及，不引入“仅被 @ 的人响铃”或“强提醒”权限；普通消息通知策略不变。
不额外暴露成员电话，也不把项目执行代理列成可 @ 的群 AI。

## 实现与访问边界

- `GroupMentionController.kt` 负责输入触发、光标和会话生命周期；复用正式输入框焦点控制，支持从语音模式回到文字。
- `GroupMentionPicker.kt` 提供主题化底部面板、可回收列表、搜索、多选、加载与重试。
- `GroupMentionDirectory.kt` 从 `GET /api/me/groups/:group_id/members` 加载完整目录。
- 原群列表保留 9 人头像预览；新目录使用同一数据库连接校验成员资格并读取全体成员，不改变未读状态。
- API 返回 `members` 和 `ai_members`，Web 使用同一接口与交互；Android 后台服务开关没有 Web 对应运行能力。
- Web 入口挂载 `group_mentions.js` / `.css`，文本使用 DOM textContent，头像仅接受图片 data URL。

## 后台通知

- `ChatBackgroundService` 统一拥有 WebSocket 网络恢复和摘要轮询；旧 `ChatRealtimeService` 仅保留兼容入口。
- 登录且开启后台收消息时，仅一条静默、低优先级状态通知；重复 resume / worker 检查不重新发布正在运行的通知。
- 状态通知的“关闭后台收消息”和设置页开关写入同一偏好；关闭后 resume、boot、worker 与 sticky 重启均不得重新保活。
- 后台 MCP 调试使用新的明确开启偏好，默认关闭，不把旧版本自动写入的 active 状态当成授权。
- 设置页、通知结束按钮、MCP 工具和 ADB 控制接收器共用持久启停逻辑。前台调试端点沿用既有能力。
- Android 前台服务运行期间仍需系统状态通知，清除通知不等于停止服务；需要停止时使用通知按钮或设置开关。
- 柔和单音、实时/摘要去重与按会话合并沿用 [消息提醒策略](android-chat-notification-policy.md)。

## 验证入口与范围

- Android：`GroupMentionTest`、`GroupMentionPickerTest`、`BackgroundNotificationTest`、`ChatNotificationPolicyTest`、`ChatMessageNotificationsTest`，共 30 项通过。
- Web：`node --test scripts/test-group-mentions.js`；安装 Playwright 后运行 `node scripts/test-group-mentions-browser.cjs`。
- 数据访问：`store::groups::members::tests`，2 项通过，覆盖 115 人完整目录、非成员、其他群与移除成员后的拒绝。
- 浏览器 fixture 只证明交互，不代替 Android 真机/OEM通知验收或跨端截图的视觉一致性验收。

本次 Windows 测试遇到全量测试二进制 PDB 上限和编译器缓存包装器异常，使用临时 Cargo 配置
`build.rustc-wrapper = ""`、`profile.test.package.elon-server.debug = 0`，仍经
`scripts/validate-rust.ps1 -DisableSccache -- test --config <临时配置> -j 2 --manifest-path server/Cargo.toml --locked --bin elon-server store::groups::members::tests`
完成。该配置仅用于本地测试，未修改共享缓存配置和产品编译选项。
