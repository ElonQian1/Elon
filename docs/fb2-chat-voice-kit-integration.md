# fb2 语音聊天体验接入

主项目提供 `android/chat-voice-kit` 作为可复用 Android Library，fb2 不需要复制主项目聊天页或语音实现。

## 接入方式

fb2 Android 工程可以把主项目仓库作为 Git submodule、subtree，或在同一工作区中通过 Gradle `includeBuild`/`include` 引用 `android/chat-voice-kit`。

推荐依赖形态：

```gradle
dependencies {
    implementation project(":chat-voice-kit")
}
```

fb2 需要向库传入主项目登录后拿到的 token：

```kotlin
val config = ChatVoiceConfig(
    baseUrl = "http://43.139.149.158:8080",
    bearerTokenProvider = { mainProjectToken },
    defaultGroupId = ChatVoiceIds.FB2_DEFAULT_GROUP_ID,
    selectedTtsVoiceProvider = { ChatVoiceIds.ANDROID_SYSTEM_TTS }
)
```

## 能力边界

- `VoiceComposerView`：SDK 正式输出的微信式聊天输入栏，内置文本/语音模式切换、整条“按住 说话”按钮、上滑取消、松手识别和状态文案。
- `VoiceComposerConfig` / `VoiceComposerCallbacks`：输入栏样式、图标、文案、默认松手区域和宿主回调。
- `SystemSpeechTranscriber`：手机系统 ASR，本地识别，适合作为显式选项或网络失败回退。
- `ChatVoiceRecorder`：按住说话期间录制 m4a，给服务端 ASR 或语音消息复用。
- `ServerAsrClient`：上传音频到主项目 `/api/voice/asr`。
- `ChatVoiceSpeaker`：优先调用主项目 `/api/voice/tts`，失败或选择 `android_system` 时回退手机系统 TTS。
- `HoldToTalkController`：按住说话、上滑取消、松开发送的手势状态。
- `ChatVoiceInteractionContract`：主项目语音浮层的状态文案、手势阈值、颜色 token 和区域判断。
- `ChatVoiceEventSink` / `ChatVoiceEvent`：统一事件流，供 fb2 原生 UI 或 H5 浮层订阅。

不要引用或复制主项目 `com.elon.app.MainSpeechInputActions`、`VoiceRecordingOverlay`、`VoiceSpeaker`、`VoiceServerTtsPlayer`。这些类属于主 App 内部实现，依赖主项目页面、附件、好友/群聊上下文。fb2 只引用 `com.elon.chatvoice.*`。

## 一键输入栏接入

fb2 Android 原生页面优先直接使用 `VoiceComposerView`。接入方不需要再重写“左侧语音/键盘切换按钮 + 中间文本框/整条按住说话按钮 + 右侧加号按钮”的 UI 逻辑。

```kotlin
val composer = VoiceComposerView(requireContext()).apply {
    applyConfig(
        VoiceComposerConfig(
            chatMode = ChatVoiceMode.FRIEND_CHAT,
            releaseZone = ChatVoiceZone.TRANSCRIBE,
            languageTag = "zh-CN",
            preferOfflineAsr = false,
            eventSink = sink,
        )
    )
    setCallbacks(object : VoiceComposerCallbacks {
        override fun onTextSubmit(text: String) {
            sendTextMessage(text)
        }

        override fun onVoiceRecognized(transcript: SpeechTranscript, zone: ChatVoiceZone) {
            when (zone) {
                ChatVoiceZone.TRANSCRIBE -> fillInput(transcript.text)
                ChatVoiceZone.AI_REPLY -> askGroupAi(transcript.text)
                ChatVoiceZone.SEND -> sendTextMessage(transcript.text)
                ChatVoiceZone.CANCEL -> Unit
            }
        }

        override fun onVoiceCanceled() {
            hideTemporaryVoiceUi()
        }

        override fun onPermissionRequired() {
            requestRecordAudioPermission()
        }

        override fun onPlusClick() {
            openAttachmentPanel()
        }
    })
}
```

`VoiceComposerView` 默认用手机系统 ASR 完成“按住说话、松手识别”，这就是主项目本地回退机制的可复用入口。需要服务端 ASR 或语音消息时，fb2 仍可以在 `onVoicePressStart` / `onVoiceReleased` 中组合 `ChatVoiceRecorder` 和 `ServerAsrClient`，但输入栏 UI 和手势状态不再复制。

常用配置：

| 配置 | 说明 |
|---|---|
| `chatMode` | `FRIEND_CHAT` 用于群聊/好友；`AGENT` 用于 AI 对话 |
| `releaseZone` | 默认松手区域；群聊可用 `SEND`，转文字输入可用 `TRANSCRIBE`，直接问 AI 可用 `AI_REPLY` |
| `copy` | 文本框 hint、`按住 说话`、权限失败、识别中、语音/键盘/加号按钮文案 |
| `style` | 输入栏背景、按钮颜色、文字颜色、圆角、间距、左侧/右侧图标 Drawable |
| `eventSink` | 继续订阅 `Start`、`Volume`、`PartialResult`、`FinalResult`、`Cancel`、`Error` 等底层语音事件 |

宿主只需要处理业务回调：

| 回调 | 用途 |
|---|---|
| `onTextSubmit(text)` | 文本发送 |
| `onVoiceRecognized(transcript, zone)` | ASR final 后按区域决定填入输入框、发消息或问 AI |
| `onVoiceCanceled()` | 上滑取消、系统取消或太短时收尾 |
| `onPermissionRequired()` | 申请 `RECORD_AUDIO` |
| `onPlusClick()` | 打开附件/更多面板 |
| `onStateChanged(state, text)` | 如需同步自定义浮层或埋点，订阅输入栏状态 |

## 操作 UI 规范

主项目语音操作分两种模式：

| 模式 | 默认松手行为 | 可选区域 |
|---|---|---|
| `ChatVoiceMode.AGENT` | `AI_REPLY` | `AI_REPLY`、`TRANSCRIBE`、`CANCEL` |
| `ChatVoiceMode.FRIEND_CHAT` | `SEND` | `SEND`、`AI_REPLY`、`TRANSCRIBE`、`CANCEL` |

状态文案由 `ChatVoiceInteractionContract.copy` 固定：

| 状态 | 文案 |
|---|---|
| 未录音 | `按住 说话` |
| 准备中 | `准备中...` |
| 录音中 | `正在听...` |
| 已听到声音 | `听到了，松手发送` |
| 松手识别中 | `识别中...` |
| 无声 | `没有检测到声音` |
| 噪声 | `环境较嘈杂，请靠近手机说话` |
| 太短 | `语音太短，请轻触再试` |
| 识别失败 | `识别失败，请重试` |
| TTS 播放 | `语音播放中...` |

区域文案：

| 区域 | 文案 |
|---|---|
| `AI_REPLY` | `松开 AI回复`，好友/群聊上滑后显示 `滑到这 AI回复` |
| `TRANSCRIBE` | `松开 转文字` |
| `CANCEL` | `松开 取消` |
| `SEND` | `松开 发送` |

切换规则：

1. `VoiceComposerView` 在文本模式显示输入框；点击左侧语音按钮后，中间区域切换为整条 `按住 说话`。
2. `ACTION_DOWN` 进入 pending；`longPressStartDelayMs` 到达后发出 `Start`。
3. 录音开始后显示浮层，状态先是 `PREPARING`，系统 ASR ready 后进入 `LISTENING`。
4. `onRmsChanged` 归一化为 0-1 音量，驱动波形。
5. partial 结果实时显示；final 结果作为最终文本。
6. 松手时，当前区域决定后续动作：`SEND` 发语音，`AI_REPLY` 用转写文本触发 AI，`TRANSCRIBE` 只转文字，`CANCEL` 丢弃。
7. 如果录音低于阈值，进入 `TOO_SHORT`，回到 `按住 说话`。
8. TTS 开始发 `TtsStart`，播放 UI 进入 `TTS_PLAYING`；结束发 `TtsEnd`。

## 手势和反馈阈值

阈值由 `ChatVoiceInteractionContract.holdOptions` 固定：

| 字段 | 默认值 | 说明 |
|---|---:|---|
| `longPressStartDelayMs` | `0` | 主项目按下即启动；fb2 不要额外加长按延迟 |
| `minRecordDurationMs` | `600` | SDK 判定录音太短的最小时长 |
| `minVoiceBytes` | `256` | SDK 判定录音太短的最小音频字节数 |
| `cancelDragUpDp` | `56dp` | 好友/群聊模式上滑进入选择区的距离 |
| `horizontalChoiceDp` | `80dp` | 兼容旧水平拖动：左取消，右转文字 |
| `touchChoiceHeightDp` | `118dp` | 底部选择托盘高度 |
| `heardVibrationMs` | `30ms` | VAD/ASR 确认听到声音时短震动 |
| `heardPulseMs` | `220ms` | 听到声音时气泡脉冲动画时长 |
| `countdownWarningRatio` | `0.75` | 倒计时超过 75% 时变黄提醒 |

视觉 token 由 `ChatVoiceInteractionContract.tokens` 固定，H5 可直接使用这些十六进制颜色：

| token | 值 |
|---|---|
| `overlayScrim` | `#CC000000` |
| `bubbleNormal` | `#58BE6A` |
| `bubbleCancel` | `#E65A5A` |
| `waveBar` | `#2C6C52` |
| `trayOuter` | `#70575757` |
| `trayInner` | `#8A707070` |
| `trayHighlight` | `#48FFFFFF` |
| `textDefault` | `#DDEDEDED` |
| `textTranscribe` | `#EAF7F0` |
| `textCancel` | `#FFE3E3` |
| `countdownNormal` | `#60FFFFFF` |
| `countdownWarning` | `#FFCC44` |

## SDK 事件

`ChatVoiceEventSink` 暴露这些事件：

| 事件 | 触发来源 |
|---|---|
| `Start` | `HoldToTalkController` 真正开始录音 |
| `Volume(value)` | `SystemSpeechTranscriber.onRmsChanged`，0-1 |
| `PartialResult(transcript)` | 手机系统 ASR partial |
| `FinalResult(transcript)` | 手机系统 ASR final |
| `Cancel` | 手势取消或主动取消 |
| `Error(error)` | ASR/TTS/播放错误 |
| `TooShort(minimumDurationMs, minimumBytes)` | `ChatVoiceRecorder.stop()` 判定录音太短 |
| `TtsStart` | 服务器或系统 TTS 开始播放 |
| `TtsEnd` | 服务器或系统 TTS 播放结束 |
| `StateChanged(state, text)` | ASR 状态变化 |
| `ZoneChanged(zone, text)` | 手势区域变化 |

示例：

```kotlin
val sink = ChatVoiceEventSink { event ->
    when (event) {
        is ChatVoiceEvent.Volume -> renderWave(event.value)
        is ChatVoiceEvent.PartialResult -> renderPartial(event.transcript.text)
        is ChatVoiceEvent.FinalResult -> submitText(event.transcript.text)
        is ChatVoiceEvent.ZoneChanged -> renderZone(event.zone, event.text)
        is ChatVoiceEvent.TtsStart -> showTtsPlaying()
        is ChatVoiceEvent.TtsEnd -> hideTtsPlaying()
        is ChatVoiceEvent.Error -> showToast(event.error.message)
        else -> Unit
    }
}
```

## 原生 UI 复用结论

`VoiceComposerView` 是 SDK 公共 API，fb2 可以直接调用；`VoiceRecordingOverlay` 仍然是主 App 内部 View：

- 依赖 `Activity.window.decorView`
- 依赖主项目 `MainSpeechInputActions` 驱动
- `internal class`，不是 SDK API
- 绑定主项目 AI回复/好友/附件行为

因此 fb2 **不能直接调用 `VoiceRecordingOverlay`，也不要复制源码**。Native Android 输入栏用 `VoiceComposerView`；如果 fb2 某些页面仍是 H5/WebView，只按 `ChatVoiceInteractionContract` 的状态机、token、文案和阈值还原浮层。

## 推荐体验策略

1. 文本、语音消息、实时转写都先使用主项目 `chat-bootstrap` 返回的协议。
2. ASR 默认走 `/api/voice/asr`；用户选择“手机系统识别”或网络失败时使用 `SystemSpeechTranscriber`。
3. TTS 默认走 `/api/voice/tts`；用户选择 `android_system` 或服务器失败时使用手机系统 TTS。
4. 群聊默认使用 `ext_fb2_official`；实时 WebSocket 使用 `target=external_group` 时传 `group_id=ext_fb2_official`。
5. fb2 UI 只实现自己的页面和主题，输入栏优先引用 `VoiceComposerView`，不复制主项目 `MainSpeechInputActions.kt`。
