# ChatGPT 实时语音悬浮控制

capability_id: android_chatgpt_realtime_voice_floating_control_v1
status: implementation_completed
verification: targeted_tests_passed_device_core_actions
production_default: enabled_after_release

The current floating overlay supersedes the earlier full-screen compact-layout branch.
Do not merge title-height or full-screen padding patches from that obsolete surface;
the active overlay intentionally has no full-screen title and does not block chat.

## 交互边界

实时语音继续由官方网页会话执行，APK 的 Activity 全局层显示一个可拖动、可折叠的控制球。控制球不遮挡聊天，不拦截返回键，也不要求用户停留在官网全屏语音页；离开“一龙 AI”聊天页后仍保持可见并可结束语音。

悬浮球采用两层状态模型：

- 生命周期：`CONNECTING`、`ACTIVE`、`ENDING`、`HANGUP_UNCONFIRMED`、`FAILED`。
- 对话回合：`IDLE`、`LISTENING`、`THINKING`、`SPEAKING`、`UNKNOWN`。

当前代码能够可靠确认连接、活动、结束和失败。活动后默认显示“待机中”；只有收到官方可验证的回合信号时，才显示“正在聆听”“思考中”或“回答中”，不使用计时器猜测官网状态。

## 用户可见状态

- 连接中：恢复本机会话、同步输入状态或启动官方语音连接。
- 待机中：官方语音已启动，但还没有可靠的当前回合信号。
- 正在聆听、思考中、回答中：为后续版本化 adapter 信号预留的正式状态。
- 结束中：已经接受挂断命令，正在等待官网退出并保留原生对话。
- 仍在通话：官网尚未确认挂断；APK 自动再尝试一次，不停止仍在工作的语音，也不误报连接异常。该状态自动收回非阻塞悬浮球，点开后仍可再次挂断或进入官网确认。
- 连接异常：可在原生浮层重试，或显式打开官网完整语音作为兜底。

展开悬浮球会显示 `记录到：项目 / 会话`，点击可回到语音绑定的会话。未归项目的会话只显示会话名；新会话先显示“发送后自动归档”，官方页面生成正式路径后通过低频内存态检查自动更新为真实归属并停止检查，不重复读取 DOM；临时聊天明确显示“不保存到历史”，不伪造归档位置，也不向日志输出标题或路径。

结束期间挂断按钮禁用，重复点击不会重复发送挂断命令。状态没有变化时不会重复触发无障碍播报，也不使用持续动画，避免额外唤醒和发热。
挂断未确认不是连接失败，不强制保持大卡片遮挡聊天；只有真实失败才自动展开操作卡片。

## 已有能力复用

- 复用 `WebChatRealtimeVoiceLaunchCache` 的会话启动提示，不重复建立语音缓存。
- 复用 `ChatGptRealtimeVoiceBackingController` 的正常退出增量快照恢复，不在正常挂断后强制刷新 WebView。
- 复用官方 WebView、Cookie、登录态和完整功能兜底，不导出凭证或私人会话内容。

## 验证

已通过状态模型、全局恢复、会话归属、动作分发和协调器定向测试。`v1.1.1278 (1288)` 小米真机已验证两条结束路径：在原生聊天页展开并挂断，以及离开“一龙 AI”回到普通消息首页后展开并挂断；两次均在结束后移除悬浮层并保留当前原生页面。悬浮球在语音期间跨页面保持可见。

本轮从新空会话启动，官方尚未生成可归档的会话路径，所以“返回会话”入口按设计禁用；已有正式会话路径时的点击返回仍需独立真机样本。新会话生成路径后的低频归属更新已通过协调器定向测试，尚未把离线证据写成真机通过。
