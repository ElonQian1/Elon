# fb2 语音 SDK 接入标准

fb2 想要主项目同款聊天操控感，应接主项目 `android/chat-voice-kit`，而不是复制主项目 App 内部页面代码。

## fb2 应使用的公共 API

- `VoiceComposerView`
- `VoiceComposerConfig`
- `VoiceComposerAsrConfig`
- `VoiceComposerCallbacks`
- `ChatVoiceRecordingOverlay`
- `ChatVoiceInteractionContract`
- `ChatVoiceEventSink`
- `ChatVoiceSpeaker`
- `ServerAsrClient`

不要复制或引用主 App 内部类：

- `MainSpeechInputActions`
- 主 App 内部 `VoiceRecordingOverlay`
- 主 App 页面里的好友/群聊实现细节

## 推荐配置

```kotlin
val composer = VoiceComposerView(context).apply {
    applyConfig(
        VoiceComposerConfig(
            chatMode = ChatVoiceMode.FRIEND_CHAT,
            releaseZone = ChatVoiceZone.SEND,
            recordingOverlayEnabled = true,
            languageTag = "zh-CN",
            preferOfflineAsr = false,
            asr = VoiceComposerAsrConfig(
                serverFallbackEnabled = true,
                serverConfig = chatVoiceConfig,
                serverOptions = ServerAsrOptions(language = "auto"),
                localResultTimeoutMs = 4_500L,
                localEngineFallbackEnabled = true,
                prewarmLocalEngine = true,
            ),
        )
    )
}
```

## 完整链路

```text
按住说话
  -> SDK 显示主项目同款浮层
  -> 启动系统 ASR
  -> 同时录制 m4a
  -> 松手
  -> 等系统 ASR final
  -> 系统 ASR 成功：按区域发送/转文字/AI回复
  -> 系统 ASR 失败/无结果/超时：上传录音到 /api/voice/asr
  -> 云端 ASR 成功：按区域发送/转文字/AI回复
  -> 云端 ASR 失败：错误态并恢复输入栏
```

## 常见问题判断

### 1. fb2 没有微信式语音按钮

优先检查：

- 是否用了 `VoiceComposerView`，还是只接了 ASR/TTS 能力。
- 是否把输入栏中间区域切换为整条 `按住 说话`。
- 是否关闭了 `recordingOverlayEnabled`。
- H5/WebView 页面是否按 `ChatVoiceInteractionContract` 自己还原了浮层。

这是接入层缺少 SDK UI 的问题，不应该让 fb2 重写一套临时浮层。

### 2. 一直显示“识别中...”

优先检查：

- `VoiceComposerAsrConfig.serverFallbackEnabled=true`。
- `serverConfig` 有主项目 `baseUrl` 和 bearer token。
- `/api/voice/asr` 服务端可用。
- 录音文件是否成功生成，大小是否超过 `minVoiceBytes`。
- 设备系统 ASR 是否不回 `onResults/onError`，例如部分小米/HyperOS。

SDK 正确状态应该是：系统 ASR 超时后进入 `SERVER_PROCESSING`，再走云端兜底，不应无限停在 `PROCESSING`。

### 3. 同一手机主项目 APK 正常，fb2 不正常

通常不是手机问题，而是链路差异：

- 主项目 APK 走完整 App 内部链路，有预热、重试、兜底、状态回收。
- fb2 如果只接了部分 SDK 或复制临时 UI，就可能缺少 `VoiceComposerAsrConfig`、录音兜底、浮层状态机或 token。
- fb2 必须按 `chat-bootstrap` 和 `VoiceComposerView` 接入，才能得到同款完整链路。

## 验收标准

- 输入栏可在文本/语音模式切换。
- 语音模式显示整条 `按住 说话`。
- 按住后显示主项目同款绿色气泡和底部选择区。
- 上滑可取消，底部区域可选 `AI回复`、`发送`、`转文字`。
- 系统 ASR 成功时能回调 final。
- 系统 ASR 无结果时自动云端 ASR。
- ASR/TTS 不因 AI 余额为 0 被阻断。
