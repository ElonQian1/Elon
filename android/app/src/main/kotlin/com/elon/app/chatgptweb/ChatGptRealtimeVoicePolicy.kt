package com.elon.app.chatgptweb

internal object ChatGptRealtimeVoicePolicy {
    const val SEMANTIC = "voice_mode"

    fun resolve(manifest: ChatGptWebUiManifest?): ChatGptWebUiControl? =
        manifest?.controls?.firstOrNull { control ->
            control.semantic == SEMANTIC && control.enabled
        }
}
