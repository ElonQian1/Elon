# fb2 语音聊天体验接入

长期协作入口：`docs/fb2-ai-center/`。本文件保留语音 SDK 接入细节，整体聊天、AI 回复、计费和验收以新工作台为准。

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
- `ChatVoiceRecordingOverlay`：SDK 正式输出的主项目同款按住说话浮层，内置深色遮罩、绿色波形气泡、实时转写、底部弧形 `取消 / AI回复 / 发送 / 转文字` 选择区。
- `VoiceComposerConfig` / `VoiceComposerCallbacks`：输入栏样式、图标、文案、默认松手区域和宿主回调。
- `SystemSpeechTranscriber`：主项目同款手机系统 ASR 链路，内置引擎枚举、系统默认解析、预热、busy/client/cold-start 重试、厂商引擎回退和 stop 后超时保护。
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
            asr = VoiceComposerAsrConfig(
                serverFallbackEnabled = true,
                serverConfig = config,
                serverOptions = ServerAsrOptions(language = "auto"),
                localResultTimeoutMs = 4_500L,
            ),
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

`VoiceComposerView` 默认用主项目同款手机系统 ASR 链路完成“按住说话、松手识别”：SDK 会先预热当前最优系统识别引擎，按下后优先使用手机本地 ASR；遇到 `ERROR_RECOGNIZER_BUSY`、瞬时 `ERROR_CLIENT`、冷启动 `SERVER_DISCONNECTED` 等厂商引擎问题时，会在本次会话内重试或切换到下一个可用引擎。fb2 不需要复制主项目 `MainSpeechInputActions` / `AgentVoiceBridge` 的内部逻辑。

`VoiceComposerView` 默认开启 `recordingOverlayEnabled = true`，按住说话时自动把主项目同款浮层挂到当前页面根 View 上：准备中显示绿色波形气泡，移动手指会高亮 `取消 / AI回复 / 发送 / 转文字` 区域，partial 结果实时显示，松手后按当前区域回调 `onVoiceRecognized(transcript, zone)`。fb2 不需要再写临时 Web 浮层。

如果某个宿主页面已经有完全自定义的浮层，可以设置 `VoiceComposerConfig(recordingOverlayEnabled = false)`，再通过 `ChatVoiceEventSink` 和 `onStateChanged` 自己渲染。但 fb2 常规聊天页应保持默认开启，以获得主项目同款操控感。

如果 fb2 需要完整稳定链路，必须传 `VoiceComposerAsrConfig(serverFallbackEnabled = true, serverConfig = config)`：SDK 会在按下时同时保存一份录音，松手后先等系统 ASR；如果系统 ASR 报错、无结果、所有本地引擎失败，或超过 `localResultTimeoutMs` 仍未返回，就自动上传录音到主项目 `/api/voice/asr`。这样 UI 不会长期停在 `识别中...`。

如果 fb2 只传 `VoiceComposerView` 而不配置 `VoiceComposerAsrConfig.serverConfig`，SDK 会退化为“仅手机系统 ASR”，不会自动走云端兜底。

小米/HyperOS 的 `com.xiaomi.mibrain.speech/.asr.AsrService` 等厂商 ASR 可能在收到 `SpeechRecognizer.StopCapture` 后不稳定返回 `onResults` / `onError`。SDK 已在 `SystemSpeechTranscriber.stop()` 内置 stop 后 final 超时；超时会主动 cancel/destroy 当前 recognizer，并抛出 `system_asr_stop_timeout`。`VoiceComposerView` 收到该错误后，如果已启用 `serverFallbackEnabled`，会进入 `SERVER_PROCESSING` 并走云端 ASR；否则会回到错误态，不会无限停在 `识别中...`。

常用配置：

| 配置 | 说明 |
|---|---|
| `chatMode` | `FRIEND_CHAT` 用于群聊/好友；`AGENT` 用于 AI 对话 |
| `releaseZone` | 默认松手区域；群聊可用 `SEND`，转文字输入可用 `TRANSCRIBE`，直接问 AI 可用 `AI_REPLY` |
| `asr.serverFallbackEnabled` | 是否启用云端 ASR 兜底 |
| `asr.serverConfig` | 主项目 `ChatVoiceConfig`；包含 baseUrl 和 bearer token |
| `asr.localResultTimeoutMs` | 松手后等待系统 ASR final 的最长时间，默认 `4500ms` |
| `asr.localEngineFallbackEnabled` | 是否启用主项目同款本地识别引擎轮换，默认 `true` |
| `asr.prewarmLocalEngine` | 是否在输入栏初始化和每次识别结束后预热系统 ASR，默认 `true` |
| `copy` | 文本框 hint、`按住 说话`、权限失败、识别中、语音/键盘/加号按钮文案 |
| `recordingOverlayEnabled` | 是否使用 SDK 内置主项目同款按住说话浮层，默认 `true` |
| `style` | 输入栏背景、按钮颜色、文字颜色、圆角、间距、左侧/右侧图标 Drawable |
| `eventSink` | 继续订阅 `Start`、`Volume`、`PartialResult`、`FinalResult`、`Cancel`、`Error` 等底层语音事件 |

宿主只需要处理业务回调：

| 回调 | 用途 |
|---|---|
| `onTextSubmit(text)` | 文本发送 |
| `onVoiceRecognized(transcript, zone)` | ASR final 后按区域决定填入输入框、发消息或问 AI |
| `onVoiceServerFallbackStarted(reason)` | 系统 ASR 失败或超时后，SDK 开始上传录音走云端 ASR |
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

`VoiceComposerView` 和 `ChatVoiceRecordingOverlay` 都是 SDK 公共 API，fb2 可以直接调用。常规场景优先使用 `VoiceComposerView`，它会自动托管 `ChatVoiceRecordingOverlay`。

主 App 内部的 `com.elon.app.VoiceRecordingOverlay` 仍然不是 SDK API：

- 依赖 `Activity.window.decorView`
- 依赖主项目 `MainSpeechInputActions` 驱动
- `internal class`，不是 SDK API
- 绑定主项目 AI回复/好友/附件行为

因此 fb2 **不能直接调用主 App 的 `com.elon.app.VoiceRecordingOverlay`，也不要复制源码**。Native Android 输入栏用 SDK `VoiceComposerView`；如果 fb2 某些页面仍是 H5/WebView，只按 `ChatVoiceInteractionContract` 的状态机、token、文案和阈值还原浮层。

## 推荐体验策略

1. 文本、语音消息、实时转写都先使用主项目 `chat-bootstrap` 返回的协议。
2. ASR 默认走 `/api/voice/asr`；用户选择“手机系统识别”或网络失败时使用 `SystemSpeechTranscriber`。
3. TTS 默认走 `/api/voice/tts`；用户选择 `android_system` 或服务器失败时使用手机系统 TTS。
4. 群聊默认使用 `ext_fb2_official`；实时 WebSocket 使用 `target=external_group` 时传 `group_id=ext_fb2_official`。
5. fb2 UI 只实现自己的页面和主题，输入栏优先引用 `VoiceComposerView`，不复制主项目 `MainSpeechInputActions.kt`。
