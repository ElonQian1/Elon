package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptSessionNavigationActionsTest {
    private val repository = FakeRepository()
    private val navigation = ChatGptConversationNavigationCoordinator(repository)
    private var ready = false
    private var loading = true
    private var bridgeReady = false
    private var current: ChatGptWebSnapshot? = snapshot("old", "/c/old")
    private val presented = mutableListOf<ChatGptWebSnapshot>()
    private val opened = mutableListOf<String>()
    private var loadingTransitions = 0
    private val actions = ChatGptSessionNavigationActions(
        sessionReady = { ready },
        sessionCanDefer = { loading },
        bridgeReady = { bridgeReady },
        commandAvailable = { true },
        startNewConversationCommand = {},
        openConversationCommand = opened::add,
        openProjectCommand = { true },
        latestSnapshot = { current },
        presentSnapshot = { value -> current = value; presented += value },
        updateLoading = { loadingTransitions += 1 },
        ensureInitialized = {},
        cancelNewConversationRecovery = {},
        scheduleNewConversationRecovery = {},
        conversationNavigation = navigation,
    )

    @Test
    fun cacheIsPresentedBeforeADeferredOfficialNavigation() {
        repository.values["/c/target"] = snapshot("cached", "/c/target")

        assertTrue(actions.openConversation("/c/target"))

        assertEquals(listOf("cached"), presented.last().messages.map { it.content })
        assertTrue(opened.isEmpty())
        assertEquals(0, loadingTransitions)

        ready = true
        loading = false
        bridgeReady = true
        actions.onSessionReady()

        assertEquals(listOf("/c/target"), opened)
        assertEquals(1, loadingTransitions)
    }

    @Test
    fun currentBridgeDispatchesImmediatelyDuringTransientSessionLoading() {
        bridgeReady = true

        assertTrue(actions.openConversation("/c/target"))

        assertEquals(listOf("/c/target"), opened)
        assertEquals(1, loadingTransitions)
    }

    private fun snapshot(content: String, path: String) = ChatGptWebSnapshot(
        title = "conversation",
        url = "https://chatgpt.com$path",
        draft = "",
        messages = listOf(ChatGptWebMessage("id", "user", content, "completed", emptyList())),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "auto",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = "conversation",
    )

    private class FakeRepository : WebChatConversationSnapshotRepository {
        val values = mutableMapOf<String, ChatGptWebSnapshot>()

        override fun restore(path: String): ChatGptWebSnapshot? = values[path]

        override fun save(path: String, snapshot: ChatGptWebSnapshot) {
            values[path] = snapshot
        }
    }
}
