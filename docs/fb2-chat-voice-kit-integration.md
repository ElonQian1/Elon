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

- `SystemSpeechTranscriber`：手机系统 ASR，本地识别，适合作为显式选项或网络失败回退。
- `ChatVoiceRecorder`：按住说话期间录制 m4a，给服务端 ASR 或语音消息复用。
- `ServerAsrClient`：上传音频到主项目 `/api/voice/asr`。
- `ChatVoiceSpeaker`：优先调用主项目 `/api/voice/tts`，失败或选择 `android_system` 时回退手机系统 TTS。
- `HoldToTalkController`：按住说话、上滑取消、松开发送的手势状态。

## 推荐体验策略

1. 文本、语音消息、实时转写都先使用主项目 `chat-bootstrap` 返回的协议。
2. ASR 默认走 `/api/voice/asr`；用户选择“手机系统识别”或网络失败时使用 `SystemSpeechTranscriber`。
3. TTS 默认走 `/api/voice/tts`；用户选择 `android_system` 或服务器失败时使用手机系统 TTS。
4. 群聊默认使用 `ext_fb2_official`；实时 WebSocket 使用 `target=external_group` 时传 `group_id=ext_fb2_official`。
5. fb2 UI 只实现自己的页面和主题，不复制主项目 `MainSpeechInputActions.kt`。
