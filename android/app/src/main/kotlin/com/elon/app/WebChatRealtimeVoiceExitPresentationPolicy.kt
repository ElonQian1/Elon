package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebSnapshot

internal object WebChatRealtimeVoiceExitPresentationPolicy {
    fun shouldHoldCurrentTranscript(
        backingActive: Boolean,
        recoveryActive: Boolean,
        originConversationPath: String?,
        hadTranscriptBeforeVoice: Boolean,
        incoming: ChatGptWebSnapshot,
    ): Boolean {
        if (backingActive) return true
        if (!recoveryActive || incoming.messages.isNotEmpty()) return false

        if (hadTranscriptBeforeVoice) return true

        val incomingPath = ChatGptWebConversationPath.fromUrl(incoming.url)
        if (incomingPath == null || !incoming.composerReady || incoming.pageKind != "conversation") {
            return true
        }
        return originConversationPath != null && incomingPath == originConversationPath
    }
}
