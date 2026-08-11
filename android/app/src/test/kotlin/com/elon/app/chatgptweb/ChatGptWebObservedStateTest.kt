package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebObservedStateTest {
    @Test
    fun retainsNavigationComposerAndCommandObservations() {
        val state = ChatGptWebObservedState()
        state.accept(ChatGptWebEvent.ConversationList(
            conversations = listOf(
                ChatGptWebConversation("demo", "桥接验证", "/c/demo", active = true),
            ),
            collection = ChatGptWebConversationCollection(
                scrollerFound = true,
                scrolled = true,
                scrollRestored = true,
                reachedEnd = true,
                observedCount = 1,
                steps = 3,
            ),
        ))
        state.accept(ChatGptWebEvent.FeatureNavigation(listOf(
            ChatGptWebFeature("feature_library", "文件库", "library", selected = false),
        )))
        state.accept(ChatGptWebEvent.ComposerControls(
            section = "model",
            currentModel = "5.6 Sol 轻度",
            options = listOf(ChatGptWebComposerOption("model_fast", "快速", false, "model")),
        ))
        state.accept(ChatGptWebEvent.CommandResult("list_conversations", true, ""))

        val snapshot = state.snapshot()
        assertEquals("/c/demo", snapshot.conversations.single().path)
        assertTrue(snapshot.conversationCollection.reachedEnd)
        assertEquals(3, snapshot.conversationCollection.steps)
        assertEquals("library", snapshot.features.single().kind)
        assertEquals("快速", snapshot.composerSections.getValue("model").single().label)
        assertTrue(snapshot.lastCommand?.ok == true)
        assertTrue(snapshot.updatedAtMs > 0)
    }

    @Test
    fun composerRequestClearsOnlyTheRequestedStaleSection() {
        val state = ChatGptWebObservedState()
        state.accept(composerEvent("model", "快速"))
        state.accept(composerEvent("tools", "网页搜索"))

        state.beginComposerRequest("model")

        assertTrue("model" !in state.snapshot().composerSections)
        assertEquals("网页搜索", state.snapshot().composerSections.getValue("tools").single().label)
    }

    @Test
    fun aNewRequestIgnoresStaleAndUnrelatedCommandResults() {
        var now = 1_000L
        val state = ChatGptWebObservedState { now }
        state.accept(ChatGptWebEvent.CommandResult("list_conversations", true, "旧结果"))

        val request = state.beginCommand("list_conversations")
        assertEquals(
            ChatGptWebObservedState.CommandRequest.PENDING,
            state.snapshot().commandRequests.single().status,
        )

        now += 10
        state.accept(ChatGptWebEvent.CommandResult("snapshot", true, ""))
        assertEquals(
            ChatGptWebObservedState.CommandRequest.PENDING,
            state.snapshot().commandRequests.single().status,
        )

        now += 10
        state.accept(ChatGptWebEvent.CommandResult(
            "list_conversations",
            false,
            "官网变化",
            requestId = request.id,
        ))
        val completed = state.snapshot().commandRequests.single()
        assertEquals(request.id, completed.id)
        assertEquals(ChatGptWebObservedState.CommandRequest.FAILED, completed.status)
        assertFalse(completed.result?.ok == true)
        assertEquals("官网变化", completed.result?.detail)
        assertEquals(now, completed.completedAtMs)
        assertEquals(now, state.snapshot().lastCommandObservedAtMs)
    }

    @Test
    fun concurrentCommandsCompleteOnlyTheirCorrelatedRequest() {
        var now = 2_000L
        val state = ChatGptWebObservedState { now }
        val first = state.beginCommand("invoke_ui_control")
        val second = state.beginCommand("invoke_ui_control")

        now += 10
        state.accept(ChatGptWebEvent.CommandResult(
            "invoke_ui_control",
            true,
            "second",
            requestId = second.id,
        ))
        var requests = state.snapshot().commandRequests
        assertEquals(ChatGptWebObservedState.CommandRequest.PENDING, requests[0].status)
        assertEquals(ChatGptWebObservedState.CommandRequest.SUCCEEDED, requests[1].status)

        now += 10
        state.accept(ChatGptWebEvent.CommandResult("invoke_ui_control", true, "native"))
        requests = state.snapshot().commandRequests
        assertEquals(ChatGptWebObservedState.CommandRequest.PENDING, requests[0].status)

        now += 10
        state.accept(ChatGptWebEvent.CommandResult(
            "invoke_ui_control",
            false,
            "first",
            requestId = first.id,
        ))
        requests = state.snapshot().commandRequests
        assertEquals(ChatGptWebObservedState.CommandRequest.FAILED, requests[0].status)
        assertEquals(ChatGptWebObservedState.CommandRequest.SUCCEEDED, requests[1].status)
    }

    @Test
    fun localPermissionFailureCompletesTheCorrelatedCommandImmediately() {
        var now = 5_000L
        val state = ChatGptWebObservedState { now }
        val request = state.beginCommand("start_dictation")

        now += 5
        state.failCommand(request.id, "start_dictation", "microphone_permission_denied")

        val completed = state.snapshot().commandRequests.single()
        assertEquals(ChatGptWebObservedState.CommandRequest.FAILED, completed.status)
        assertEquals("microphone_permission_denied", completed.result?.detail)
        assertEquals(now, completed.completedAtMs)
    }

    @Test
    fun pendingRequestsExpireAndHistoryStaysBounded() {
        var now = 10_000L
        val state = ChatGptWebObservedState { now }
        repeat(25) { state.beginCommand("invoke_ui_control") }

        assertEquals(20, state.snapshot().commandRequests.size)
        now += 20_000L
        val requests = state.snapshot().commandRequests

        assertTrue(requests.all { it.status == ChatGptWebObservedState.CommandRequest.TIMED_OUT })
        assertTrue(requests.all { it.completedAtMs == now })
    }

    @Test
    fun pageGenerationClearsDocumentObservationsAndFailsPendingCommands() {
        var now = 30_000L
        val state = ChatGptWebObservedState { now }
        state.updateDocument(document(page = 1, adapter = 1))
        state.accept(ChatGptWebEvent.ConversationList(listOf(
            ChatGptWebConversation("demo", "桥接验证", "/c/demo", active = true),
        )))
        state.accept(composerEvent("model", "快速"))
        val pending = state.beginCommand("list_model_options")

        now += 10
        state.updateDocument(document(page = 2, adapter = 0))

        val reloading = state.snapshot()
        assertTrue(reloading.conversations.isEmpty())
        assertEquals(0, reloading.conversationCollection.observedCount)
        assertTrue(reloading.composerSections.isEmpty())
        assertEquals(2L, reloading.pageGeneration)
        assertEquals(0L, reloading.adapterGeneration)
        assertFalse(reloading.adapterCurrent)
        val failed = reloading.commandRequests.single { it.id == pending.id }
        assertEquals(ChatGptWebObservedState.CommandRequest.FAILED, failed.status)
        assertEquals("page_generation_changed", failed.result?.detail)

        now += 10
        state.updateDocument(document(page = 2, adapter = 2))
        assertTrue(state.snapshot().adapterCurrent)
    }

    private fun document(page: Long, adapter: Long) = ChatGptWebDocumentSession.Snapshot(
        pageGeneration = page,
        adapterGeneration = adapter,
        documentToken = "doc_page_$page",
    )

    private fun composerEvent(section: String, label: String) = ChatGptWebEvent.ComposerControls(
        section = section,
        currentModel = "5.6 Sol 轻度",
        options = listOf(ChatGptWebComposerOption("${section}_option", label, false, "menuitem")),
    )
}
