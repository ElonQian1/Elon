# ChatGPT 实时语音悬浮控制

capability_id: android_chatgpt_realtime_voice_floating_control_v1
status: implementation_completed
verification: targeted_tests_passed_device_pending
production_default: enabled_after_release

## 交互边界

实时语音继续由官方网页会话执行，APK 的 Activity 全局层显示一个可拖动、可折叠的控制球。控制球不遮挡聊天，不拦截返回键，也不要求用户停留在官网全屏语音页；离开“一龙 AI”聊天页后仍保持可见并可结束语音。

悬浮球采用两层状态模型：

- 生命周期：`CONNECTING`、`ACTIVE`、`ENDING`、`FAILED`。
- 对话回合：`IDLE`、`LISTENING`、`THINKING`、`SPEAKING`、`UNKNOWN`。

当前代码能够可靠确认连接、活动、结束和失败。活动后默认显示“待机中”；只有收到官方可验证的回合信号时，才显示“正在聆听”“思考中”或“回答中”，不使用计时器猜测官网状态。

## 用户可见状态

- 连接中：恢复本机会话、同步输入状态或启动官方语音连接。
- 待机中：官方语音已启动，但还没有可靠的当前回合信号。
- 正在聆听、思考中、回答中：为后续版本化 adapter 信号预留的正式状态。
- 结束中：已经接受挂断命令，正在等待官网退出并保留原生对话。
- 连接异常：可在原生浮层重试，或显式打开官网完整语音作为兜底。

展开悬浮球会显示 `记录到：项目 / 会话`，点击可回到语音启动时绑定的会话。未归项目的会话只显示会话名；新会话显示“发送后自动归档”；临时聊天明确显示“不保存到历史”，不伪造归档位置，也不向日志输出标题或路径。

结束期间挂断按钮禁用，重复点击不会重复发送挂断命令。状态没有变化时不会重复触发无障碍播报，也不使用持续动画，避免额外唤醒和发热。

## 已有能力复用

- 复用 `WebChatRealtimeVoiceLaunchCache` 的会话启动提示，不重复建立语音缓存。
- 复用 `ChatGptRealtimeVoiceBackingController` 的正常退出增量快照恢复，不在正常挂断后强制刷新 WebView。
- 复用官方 WebView、Cookie、登录态和完整功能兜底，不导出凭证或私人会话内容。

## 验证

已通过状态模型、全局恢复、会话归属和协调器定向测试。真机仍需用户监督完成一次“进入语音、切到普通首页仍看到悬浮球、展开确认会话归属、正常挂断、观察结束状态、立即返回原生对话”的敏感验收。
