package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatSnapshotWindowMergerTest {
    @Test
    fun partialOfficialWindowUpdatesCacheWithoutErasingOlderMessages() {
        val cached = snapshot(
            messages = listOf(message("u1", "旧问题"), message("a1", "旧回答"),
                message("u2", "新问题"), message("a2", "旧的部分回答")),
            observed = 4,
        )
        val live = snapshot(
            messages = listOf(message("u2", "新问题"), message("a2", "完整回答")),
            start = 2,
            observed = 4,
        )

        val merged = WebChatSnapshotWindowMerger.merge(cached, live, sameConversation = true)

        assertEquals(listOf("旧问题", "旧回答", "新问题", "完整回答"),
            merged.messages.map { it.content })
        assertEquals(0, merged.messageWindowStart)
        assertEquals(4, merged.observedMessageCount)
    }

    @Test
    fun emptyTransientSnapshotKeepsCachedConversationVisible() {
        val cached = snapshot(listOf(message("u1", "问题"), message("a1", "回答")), observed = 2)

        val merged = WebChatSnapshotWindowMerger.merge(
            cached,
            snapshot(emptyList(), observed = 0),
            sameConversation = true,
        )

        assertEquals(listOf("问题", "回答"), merged.messages.map { it.content })
    }

    @Test
    fun differentConversationNeverInheritsCachedMessages() {
        val merged = WebChatSnapshotWindowMerger.merge(
            snapshot(listOf(message("old", "私人旧内容")), observed = 1),
            snapshot(listOf(message("new", "新会话")), observed = 1),
            sameConversation = false,
        )

        assertEquals(listOf("新会话"), merged.messages.map { it.content })
    }

    @Test
    fun mergedWindowRemainsBoundedToTheAdapterLimit() {
        val cached = snapshot((0 until 80).map { message("m$it", "内容$it") }, observed = 80)
        val live = snapshot((70 until 90).map { message("m$it", "更新$it") }, start = 70, observed = 90)

        val merged = WebChatSnapshotWindowMerger.merge(cached, live, sameConversation = true)

        assertEquals(80, merged.messages.size)
        assertEquals(10, merged.messageWindowStart)
        assertEquals("内容10", merged.messages.first().content)
        assertEquals("更新89", merged.messages.last().content)
    }

    private fun snapshot(
        messages: List<ChatGptWebMessage>,
        start: Int = 0,
        observed: Int,
    ) = ChatGptWebSnapshot(
        title = "会话",
        url = "https://chatgpt.com/c/example",
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "自动",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = "conversation",
        messageWindowStart = start,
        observedMessageCount = observed,
    )

    private fun message(id: String, content: String) = ChatGptWebMessage(
        id = id,
        role = if (id.startsWith("u")) "user" else "assistant",
        content = content,
        state = "completed",
        parts = emptyList(),
    )
}
