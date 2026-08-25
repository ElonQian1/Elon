# ChatGPT 实时语音退出恢复

capability_id: android_chatgpt_realtime_voice_exit_snapshot_recovery_v1
status: implementation_completed
verification: targeted_tests_passed_device_pending
production_default: enabled_after_release

## 问题与修复

旧逻辑在实时语音退出后等待 2.5 到 3 秒；如果期间没有收到新快照，就强制刷新后台 WebView。即使官网已经正常返回当前会话，这个定时刷新仍可能清空原生消息窗口，造成先显示转写、随后黑屏、最后重新加载的重复过程。

当前逻辑把正常退出和异常中断分开：

- 用户点击挂断时，短暂缺少官网控件会进入有界等待并异步刷新 controls，不再立即按异常中断处理。
- 挂断命令成功后，只要连续确认已回到同一会话且结束控件消失，就按正常退出处理；不再强制等待语音入口重新渲染。
- 正常退出立即隐藏官网语音界面，保留原生消息，并主动请求当前会话快照；600 毫秒后仍属于同一恢复周期时再请求一次。
- 退出时同时异步复用已验证的同源会话 GET，只允许刷新当前 `/c/{id}`，不导航、不发送 POST；同一会话并发请求会合并为单飞。
- 私有刷新超时、熔断、缺少请求上下文或解析失败时不清空现有消息，也不误报官网无能力，官方 DOM 快照继续兜底。
- 正常退出禁止强制刷新 WebView。快照到达后只增量更新原生消息。
- 异常中断保留 3 秒兜底；只有恢复周期仍有效且没有新会话快照时才重载。
- 新一次语音周期会使旧恢复回调失效，避免延迟任务影响后续会话。

## 已有能力边界

每个厂商和会话的语音入口状态已经由 `WebChatRealtimeVoiceLaunchCache` 有界保存，协调器已有直接启动、刷新控制和恢复会话三种计划。本能力只修复退出后的消息恢复，不重复实现语音入口缓存或官方控制发现。

## 验证

已通过：

- `test-chatgpt-web-private-transport.js`
- `ChatGptRealtimeVoiceRecoveryGateTest`
- `WebChatRealtimeVoiceCoordinatorTest`
- `WebChatProductionVoiceEntryContractTest`

真机仍需用户监督完成一次“进入实时语音、说话、正常挂断、立即看到原生消息、再次进入”的敏感验收。验收不得清理 Cookie 或应用数据，也不得记录音频或会话正文。
