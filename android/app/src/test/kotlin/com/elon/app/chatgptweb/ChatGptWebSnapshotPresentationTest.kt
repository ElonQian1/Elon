package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebSnapshotPresentationTest {
    @Test
    fun loadingAConversationShowsOnlyThatConversationsCachedMessages() {
        val loading = ChatGptWebSnapshotPresentation.loadingConversation(
            cached = snapshot("目标缓存", "https://chatgpt.com/c/target", "快速"),
            previous = snapshot("上一会话", "https://chatgpt.com/c/previous", "自动"),
            path = "/c/target",
        )

        assertEquals(listOf("目标缓存"), loading.messages.map { it.content })
        assertEquals("https://chatgpt.com/c/target", loading.url)
        assertEquals("快速", loading.currentModel)
        assertPassive(loading)
    }

    @Test
    fun missingTargetCacheNeverLeaksThePreviousConversation() {
        val loading = ChatGptWebSnapshotPresentation.loadingConversation(
            cached = null,
            previous = snapshot("私人上一会话", "https://chatgpt.com/c/previous", "自动"),
            path = "/c/target",
        )

        assertTrue(loading.messages.isEmpty())
        assertEquals("自动", loading.currentModel)
        assertPassive(loading)
    }

    @Test
    fun newConversationImmediatelyPresentsAnEmptyPassiveComposer() {
        val loading = ChatGptWebSnapshotPresentation.newConversation(
            snapshot("旧消息", "https://chatgpt.com/c/previous", "自动"),
        )

        assertTrue(loading.messages.isEmpty())
        assertEquals(ChatGptWebNavigationPolicy.START_URL, loading.url)
        assertPassive(loading)
    }

    @Test
    fun revalidationKeepsMessagesButDropsLiveAuthority() {
        val cached = ChatGptWebSnapshotPresentation.revalidating(
            snapshot("本地内容", "https://chatgpt.com/c/target", "自动"),
        )

        assertEquals(listOf("本地内容"), cached.messages.map { it.content })
        assertEquals("https://chatgpt.com/c/target", cached.url)
        assertPassive(cached)
    }

    private fun assertPassive(value: ChatGptWebSnapshot) {
        assertFalse(value.authenticated)
        assertFalse(value.composerReady)
        assertFalse(value.streaming)
        assertFalse(value.loginRequired)
        assertTrue(value.capabilities.supported.isEmpty())
    }

    private fun snapshot(content: String, url: String, model: String) = ChatGptWebSnapshot(
        title = "会话",
        url = url,
        draft = "不应沿用",
        messages = listOf(ChatGptWebMessage("id", "user", content, "completed", emptyList())),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = model,
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = "conversation",
    )
}
