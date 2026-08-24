package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatRealtimeVoiceTranscriptContinuityTest {
    @Test
    fun voiceSnapshotsAreCapturedWithoutReplacingTheVisibleConversation() {
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        val beforeVoice = snapshot("/c/origin", "old question", "old answer")
        val duringVoice = snapshot(
            "/c/origin",
            "old question",
            "old answer",
            "voice question",
            "voice answer",
        )

        continuity.begin(beforeVoice)

        assertNull(continuity.resolve(duringVoice))
        assertEquals(duringVoice, continuity.end(duringVoice))
    }

    @Test
    fun exitImmediatelyPresentsCapturedVoiceTranscriptAndIgnoresTransientEmptyDom() {
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        val beforeVoice = snapshot("/c/origin", "old question", "old answer")
        val voiceTranscript = snapshot(
            "/c/origin",
            "old question",
            "old answer",
            "voice question",
            "voice answer",
        )
        continuity.begin(beforeVoice)
        continuity.resolve(voiceTranscript)
        continuity.end(voiceTranscript)

        assertEquals(
            voiceTranscript.messages,
            continuity.resolve(emptySnapshot("/"))?.messages,
        )
        assertEquals(
            voiceTranscript.messages,
            continuity.resolve(emptySnapshot("/c/origin"))?.messages,
        )
    }

    @Test
    fun validTranscriptUpdatesDoNotEndProtectionAgainstALaterEmptySnapshot() {
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        val beforeVoice = snapshot("/c/origin", "old question", "old answer")
        val firstFinal = snapshot("/c/origin", "voice question", "draft answer")
        val settled = snapshot("/c/origin", "voice question", "final answer")
        continuity.begin(beforeVoice)
        continuity.end(firstFinal)

        assertEquals(settled, continuity.resolve(settled))
        assertEquals(settled.messages, continuity.resolve(emptySnapshot("/"))?.messages)
    }

    @Test
    fun firstVoiceConversationPromotesItsNewConversationPath() {
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        val voiceTranscript = snapshot("/c/new", "voice question", "voice answer")
        continuity.begin(emptySnapshot("/"))
        continuity.resolve(voiceTranscript)

        assertEquals(voiceTranscript, continuity.end(voiceTranscript))
        assertEquals(
            voiceTranscript.messages,
            continuity.resolve(emptySnapshot("/c/new"))?.messages,
        )
    }

    @Test
    fun explicitResetAllowsANewEmptyConversation() {
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        continuity.begin(snapshot("/c/origin", "question", "answer"))
        continuity.end(null)
        continuity.reset()

        val empty = emptySnapshot("/")
        assertEquals(empty, continuity.resolve(empty))
    }

    @Test
    fun smallerPartialSnapshotCannotEraseAlreadyObservedVoiceTurns() {
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        val complete = snapshot(
            "/c/origin",
            "old question",
            "old answer",
            "voice question",
            "voice answer",
        )
        val partial = snapshot("/c/origin", "voice question", "voice answer")
        continuity.begin(complete)
        continuity.end(complete)

        assertEquals(complete.messages, continuity.resolve(partial)?.messages)
    }

    @Test
    fun sameMessageCountCannotReplaceACompleteVoiceAnswerWithATruncatedBubble() {
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        val beforeVoice = snapshot("/c/origin", "old question", "old answer")
        val complete = snapshot(
            "/c/origin",
            "voice question",
            "the complete voice answer remains visible after voice closes",
        )
        val truncated = snapshot(
            "/c/origin",
            "voice question",
            "the complete voice answer",
        )
        continuity.begin(beforeVoice)
        continuity.resolve(complete)
        continuity.end(complete)

        val restored = continuity.resolve(truncated)

        assertEquals(complete.messages, restored?.messages)
    }

    private fun snapshot(path: String, vararg content: String): ChatGptWebSnapshot {
        val messages = content.mapIndexed { index, value ->
            ChatGptWebMessage(
                id = "message-$index",
                role = if (index % 2 == 0) "user" else "assistant",
                content = value,
                state = "completed",
                parts = emptyList(),
            )
        }
        return base(path, messages)
    }

    private fun emptySnapshot(path: String): ChatGptWebSnapshot = base(path, emptyList())

    private fun base(path: String, messages: List<ChatGptWebMessage>) = ChatGptWebSnapshot(
        title = "voice conversation",
        url = "https://chatgpt.com$path",
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = path.startsWith("/c/"),
        streaming = false,
        currentModel = "GPT-5",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = if (path.startsWith("/c/")) "conversation" else "home",
    )
}
