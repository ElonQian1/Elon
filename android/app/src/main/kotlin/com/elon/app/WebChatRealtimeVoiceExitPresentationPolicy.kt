package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal object WebChatRealtimeVoiceExitPresentationPolicy {
    fun shouldHoldCurrentTranscript(
        recoveryActive: Boolean,
        originConversationPath: String?,
        hadTranscriptBeforeVoice: Boolean,
        incoming: ChatGptWebSnapshot,
    ): Boolean {
        if (!recoveryActive || incoming.messages.isNotEmpty()) return false

        val incomingPath = ChatGptWebConversationPath.fromUrl(incoming.url)
        if (incomingPath == null || !incoming.composerReady || incoming.pageKind != "conversation") {
            return true
        }
        return hadTranscriptBeforeVoice && incomingPath == originConversationPath
    }
}
