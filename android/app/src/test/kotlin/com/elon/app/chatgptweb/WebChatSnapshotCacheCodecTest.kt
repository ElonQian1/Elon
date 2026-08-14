package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSnapshotCacheCodecTest {
    @Test
    fun restoresRecentMessagesForDisplayWithoutRestoringLiveAuthority() {
        val snapshot = snapshot(messages = listOf(
            message("u1", "user", "问题"),
            message(
                "a1",
                "assistant",
                "回答",
                parts = listOf(ChatGptWebMessagePart(
                    type = "file",
                    label = "结果.csv",
                    metadata = ChatGptWebMessagePartMetadata(mediaType = "text/csv"),
                )),
            ),
        ))

        val decoded = WebChatSnapshotCacheCodec.decode(WebChatSnapshotCacheCodec.encode(
            WebChatSnapshotCache(snapshot, savedAtMs = 123L),
        ))!!

        assertEquals(123L, decoded.savedAtMs)
        assertEquals(listOf("问题", "回答"), decoded.snapshot.messages.map { it.content })
        assertEquals("text/csv", decoded.snapshot.messages.last().parts.single().metadata?.mediaType)
        assertEquals("自动", decoded.snapshot.currentModel)
        assertFalse(decoded.snapshot.authenticated)
        assertFalse(decoded.snapshot.composerReady)
        assertFalse(decoded.snapshot.streaming)
        assertTrue(decoded.snapshot.capabilities.supported.isEmpty())
    }

    @Test
    fun boundsTheCachedWindowAndDropsTransientStreamingState() {
        val messages = (0 until 40).map { index ->
            message("m$index", if (index % 2 == 0) "user" else "assistant", "内容$index", "streaming")
        }

        val decoded = WebChatSnapshotCacheCodec.decode(WebChatSnapshotCacheCodec.encode(
            WebChatSnapshotCache(snapshot(messages, messageWindowStart = 10), savedAtMs = 1L),
        ))!!.snapshot

        assertEquals(32, decoded.messages.size)
        assertEquals("内容8", decoded.messages.first().content)
        assertEquals(18, decoded.messageWindowStart)
        assertTrue(decoded.messages.all { it.state == "completed" })
    }

    @Test
    fun rejectsUnknownOrMalformedPayloads() {
        assertNull(WebChatSnapshotCacheCodec.decode("{}"))
        assertNull(WebChatSnapshotCacheCodec.decode(
            """{"schema":"elon.web_chat.snapshot_cache.v1","saved_at_ms":-1,"messages":[]}""",
        ))
    }

    @Test
    fun providerNamespacesProduceIndependentFiles() {
        assertEquals("web-chat-chatgpt-snapshot-v1.json", WebChatSnapshotStore.fileName("chatgpt"))
        assertEquals("web-chat-google-snapshot-v1.json", WebChatSnapshotStore.fileName("google"))
    }

    private fun snapshot(
        messages: List<ChatGptWebMessage>,
        messageWindowStart: Int = 0,
    ) = ChatGptWebSnapshot(
        title = "会话",
        url = "https://chatgpt.com/c/example",
        draft = "不应恢复",
        messages = messages,
        authenticated = true,
        composerReady = true,
        streaming = true,
        currentModel = "自动",
        attachments = listOf(ChatGptWebAttachment("a", "x", "ready", true)),
        dictationActive = true,
        capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = "conversation",
        messageWindowStart = messageWindowStart,
        observedMessageCount = messageWindowStart + messages.size,
    )

    private fun message(
        id: String,
        role: String,
        content: String,
        state: String = "completed",
        parts: List<ChatGptWebMessagePart> = emptyList(),
    ) = ChatGptWebMessage(id, role, content, state, parts)
}
