---
version_status: current
reviewed_at: 2026-09-15
implementation_status: implemented
---

# Android 群聊与私聊消息提醒

## 问题与范围

旧 `ChatMessageNotifications` 在发送有声系统通知后，后台继续调用
`playFallbackMessageSound`，前台调用 `playForegroundHintIfNeeded`，形成两个声音来源。
前台 1.5 秒节流只约束手动播放，不能去重系统通知音。每条消息还使用不同通知 ID，
所以同一会话的多条消息会生成多张通知卡。截图中不同内容、不同时间的两张卡不能单独
证明同一条消息重复投递。

本次范围为 APK 人际私聊和群聊，提示音、通知卡与消息去重；任务完成提醒有独立实现。

## 行为合同

- 声音和震动只由 Android 系统通知触发，不调用 Ringtone 或 Vibrator 进行额外补播。
- 默认音为原创 280 毫秒、740 Hz 主音的单次柔和提示，单声道 PCM16，无连续冒泡。
- 正在查看的会话不提醒；先记住消息身份，再抑制显示，避免切走后轮询再次提醒。
- 同一账号、同一好友或同一群更新同一张卡；不同会话保留独立通知。
- 私聊 3 秒、群聊 10 秒内的新消息静默更新，跨会话另有 1.5 秒总间隔。
  静默更新不延长间隔，间隔后新的消息可以再次响铃。
- 已知消息 ID 优先去重；摘要轮询缺失 ID 时，以会话和规范化服务端时间戳与实时事件
  对账，兼容摘要对附件和正文的转换。去重有界、进程内有效，不宣称跨重启持久去重。
- 群列表中的名称缓存供未携带群名的实时通知使用，按一龙账号与会话隔离。
- 新通知通道继承旧通道的重要性、关闭状态、静音、自定义声音、震动等选择。
  Android 11+ 同时检查用户是否明确选择声音；更早版本只能比较声音 URI。
  已创建的新通道不反复重设。系统通知权限、勿扰和音量仍由 Android 控制。

## 实现与验证入口

- `android/app/src/main/kotlin/com/elon/app/ChatMessageNotifications.kt`：组装和投递通知。
- `android/app/src/main/kotlin/com/elon/app/ChatNotificationPolicy.kt`：去重、可见会话与提醒间隔。
- `android/app/src/main/kotlin/com/elon/app/ChatNotificationChannel.kt`：声音和通道迁移。
- `android/app/src/main/kotlin/com/elon/app/SocialSummaryPoller.kt`：摘要补偿及名称缓存。
- `scripts/generate-chat-message-soft-sound.ps1`：可重复生成原声音资源，无外部音频素材。
- `ChatNotificationPolicyTest`：双来源两种到达顺序、重复正文、相同时间不同 ID、
  可见会话、两种间隔、跨会话隔离和并发到达。
- `ChatMessageNotificationsTest`：Android 通知卡合并、静默更新、标题和通道设置继承。

验证执行结果与 APK 发布身份以本次构建、发布及统一收尾回执为准。
自动测试不替代用户手机的实际听感验收。

## 平台依据

Android 通知通道创建后，声音等行为由系统与用户设置控制，不能靠再次创建同 ID
通道修改默认行为；见 [官方通知通道文档](https://developer.android.com/develop/ui/compose/notifications/channels)。
连续消息通过通知更新和静默标志抑制打扰；见
[NotificationCompat.Builder](https://developer.android.com/reference/androidx/core/app/NotificationCompat.Builder)。
