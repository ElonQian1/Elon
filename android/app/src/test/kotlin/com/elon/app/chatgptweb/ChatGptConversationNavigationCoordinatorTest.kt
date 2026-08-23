package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptConversationNavigationCoordinatorTest {
    private val repository = FakeRepository()
    private val coordinator = ChatGptConversationNavigationCoordinator(repository)

    @Test
    fun openingHistoryUsesTargetCacheAndIgnoresThePreviousSnapshot() {
        val previous = snapshot("old", "/c/old")
        repository.values["/c/target"] = snapshot("cached target", "/c/target")

        val loading = coordinator.beginOpen("/c/target", previous)

        assertEquals(listOf("cached target"), loading.messages.map { it.content })
        assertTrue(coordinator.hasPending())
        assertFalse(coordinator.shouldAccept(previous))
        assertTrue(coordinator.shouldAccept(snapshot("fresh target", "/c/target")))
        assertFalse(coordinator.hasPending())
        assertTrue(coordinator.isNavigating())
        coordinator.complete()
        assertFalse(coordinator.isNavigating())
    }

    @Test
    fun previewingHistoryUsesTargetCacheWithoutStartingANavigationBoundary() {
        val previous = snapshot("old", "/c/old")
        repository.values["/c/target"] = snapshot("cached target", "/c/target")

        val preview = coordinator.previewOpen("/c/target", previous)

        assertEquals(listOf("cached target"), preview.messages.map { it.content })
        assertFalse(coordinator.hasPending())
        assertFalse(coordinator.isNavigating())
    }

    @Test
    fun openingHistoryWithoutCacheNeverDisplaysPreviousMessages() {
        val loading = coordinator.beginOpen("/c/target", snapshot("private old", "/c/old"))

        assertTrue(loading.messages.isEmpty())
    }

    @Test
    fun newConversationIgnoresTheOldTurnUntilAnEmptyBoundaryArrives() {
        val previous = snapshot("old", "/c/old")

        val loading = coordinator.beginNew(previous)

        assertTrue(loading.messages.isEmpty())
        assertFalse(coordinator.shouldAccept(previous))
        assertTrue(coordinator.shouldAccept(snapshot("", "/")))
        assertFalse(coordinator.hasPending())
        assertTrue(coordinator.isNavigating())
        coordinator.complete()
        assertFalse(coordinator.isNavigating())
    }

    @Test
    fun commandFailureRestoresThePreviousConversation() {
        val previous = snapshot("old", "/c/old")
        coordinator.beginOpen("/c/target", previous)

        assertEquals(previous, coordinator.restoreAfterFailure("open_conversation"))
        assertFalse(coordinator.hasPending())
        assertFalse(coordinator.isNavigating())
        assertNull(coordinator.restoreAfterFailure("open_conversation"))
    }

    @Test
    fun completedSnapshotsAreSavedThroughTheRepository() {
        val value = snapshot("answer", "/c/target")

        coordinator.save("/c/target", value)

        assertEquals(value, repository.values["/c/target"])
    }

    private fun snapshot(content: String, path: String) = ChatGptWebSnapshot(
        title = "会话",
        url = "https://chatgpt.com$path",
        draft = "",
        messages = content.takeIf(String::isNotBlank)?.let {
            listOf(ChatGptWebMessage("id", "user", it, "completed", emptyList()))
        }.orEmpty(),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "自动",
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
