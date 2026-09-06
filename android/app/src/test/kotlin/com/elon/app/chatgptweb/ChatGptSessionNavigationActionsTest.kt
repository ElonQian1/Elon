package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
    private var initializationCount = 0
    private var newConversationCommands = 0
    private var navigationPriorities = 0
    private val actions = ChatGptSessionNavigationActions(
        sessionReady = { ready },
        sessionCanDefer = { loading },
        bridgeReady = { bridgeReady },
        commandAvailable = { true },
        startNewConversationCommand = { newConversationCommands += 1 },
        openConversationCommand = { path ->
            opened += path
            "open-${opened.size}"
        },
        openProjectCommand = { true },
        latestSnapshot = { current },
        presentSnapshot = { value -> current = value; presented += value },
        updateLoading = { loadingTransitions += 1 },
        ensureInitialized = { initializationCount += 1 },
        prioritizeUserNavigation = { navigationPriorities += 1 },
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
        assertEquals(1, navigationPriorities)

        ready = true
        loading = false
        bridgeReady = true
        actions.onSessionReady()

        assertEquals(listOf("/c/target"), opened)
        assertEquals(1, loadingTransitions)
        assertEquals("open-1", actions.lastOpenRequestId())
        val repeated = actions.openConversationTracked("/g/g-p-project/c/target")
        assertTrue(repeated.accepted)
        assertEquals("open-1", repeated.requestId)
        assertEquals(listOf("/c/target"), opened)
    }

    @Test
    fun currentBridgeDispatchesImmediatelyDuringTransientSessionLoading() {
        bridgeReady = true

        assertTrue(actions.openConversation("/c/target"))

        assertEquals(listOf("/c/target"), opened)
        assertEquals(1, loadingTransitions)
        assertEquals("open-1", actions.lastOpenRequestId())
        val repeated = actions.openConversationTracked("/c/target")
        assertTrue(repeated.accepted)
        assertEquals("open-1", repeated.requestId)
        assertEquals(listOf("/c/target"), opened)
    }

    @Test
    fun newConversationPreviewsImmediatelyAndDispatchesOnceWhenBridgeIsReady() {
        assertTrue(actions.startNewConversation())

        assertTrue(presented.last().messages.isEmpty())
        assertEquals("home", presented.last().pageKind)
        assertEquals(1, initializationCount)
        assertEquals(1, navigationPriorities)
        assertEquals(0, newConversationCommands)
        assertEquals(0, loadingTransitions)

        bridgeReady = true
        actions.onBridgeReady()
        actions.onBridgeReady()

        assertEquals(1, newConversationCommands)
        assertEquals(1, loadingTransitions)
        assertFalse(actions.startNewConversation())
    }

    @Test
    fun newConversationIsRejectedWhenTheSessionCannotRecover() {
        loading = false

        assertFalse(actions.startNewConversation())
        assertTrue(presented.isEmpty())
        assertEquals(0, initializationCount)
        assertEquals(0, navigationPriorities)
        assertEquals(0, newConversationCommands)
    }

    @Test
    fun confirmedDeletionPresentsEmptyContextBeforeLoadingHomeAndCancelsDeferredNavigation() {
        assertTrue(actions.openConversation("/c/old"))
        val next = ChatGptWebSnapshotPresentation.newConversation(current)
        var loads = 0
        actions.showAfterDeletion(next) {
            assertEquals("https://chatgpt.com/", current?.url)
            assertTrue(current?.messages?.isEmpty() == true)
            loads += 1
        }
        bridgeReady = true
        actions.onBridgeReady()
        actions.showAfterDeletion(null) { loads += 1 }
        assertEquals(1, loads)
        assertTrue(opened.isEmpty())
        assertFalse(navigation.hasPending())
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
