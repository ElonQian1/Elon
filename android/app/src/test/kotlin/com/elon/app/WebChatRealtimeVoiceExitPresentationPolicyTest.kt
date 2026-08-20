package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceExitPresentationPolicyTest {
    @Test
    fun holdsExistingTranscriptAcrossTransientReloadSnapshot() {
        assertTrue(policy(snapshot(url = "https://chatgpt.com/", messages = emptyList())))
    }

    @Test
    fun holdsExistingTranscriptWhenTheSameConversationTemporarilyLooksEmpty() {
        assertTrue(policy(snapshot(url = "https://chatgpt.com/c/origin", messages = emptyList())))
    }

    @Test
    fun releasesWhenRecoveredConversationContainsMessages() {
        assertFalse(policy(snapshot(
            url = "https://chatgpt.com/c/origin",
            messages = listOf(ChatGptWebMessage("answer", "assistant", "ok", "completed", emptyList())),
        )))
    }

    @Test
    fun emptyDifferentConversationCanReplaceThePreviousTranscript() {
        assertFalse(policy(snapshot(url = "https://chatgpt.com/c/next", messages = emptyList())))
    }

    @Test
    fun validEmptyConversationCanReplaceTheSyncPlaceholderWhenVoiceStartedFromHome() {
        assertFalse(
            WebChatRealtimeVoiceExitPresentationPolicy.shouldHoldCurrentTranscript(
                recoveryActive = true,
                originConversationPath = null,
                hadTranscriptBeforeVoice = false,
                incoming = snapshot(url = "https://chatgpt.com/c/new", messages = emptyList()),
            ),
        )
    }

    private fun policy(incoming: ChatGptWebSnapshot) =
        WebChatRealtimeVoiceExitPresentationPolicy.shouldHoldCurrentTranscript(
            recoveryActive = true,
            originConversationPath = "/c/origin",
            hadTranscriptBeforeVoice = true,
            incoming = incoming,
        )

    private fun snapshot(url: String, messages: List<ChatGptWebMessage>) = ChatGptWebSnapshot(
        title = "会话",
        url = url,
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(emptySet()),
        pageKind = if (url.contains("/c/")) "conversation" else "home",
    )
}
