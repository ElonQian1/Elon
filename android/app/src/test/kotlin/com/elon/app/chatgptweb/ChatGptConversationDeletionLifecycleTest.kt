package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class ChatGptConversationDeletionLifecycleTest {
    private var now = 0L
    private val guard = ChatGptConversationDeletionGuard { now }
    private fun snapshot(path: String = "/c/target") = ChatGptWebSnapshot(
        title = "Fixture", url = "https://chatgpt.com$path", draft = "", messages = emptyList(),
        authenticated = true, composerReady = true, streaming = false, currentModel = "auto",
        attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = if (path == "/") "home" else "conversation",
    )
    private fun begin(current: String = "/c/other", active: Boolean = false) =
        guard.begin("delete-1", active, "/c/target", snapshot(current).url)
    private fun result(ok: Boolean, id: String = "delete-1") =
        ChatGptWebEvent.CommandResult("delete_conversation", ok, "fixture", requestId = id)

    @Test fun activeVoiceRejectsDeletionEvenWhenTheVisibleConversationHasChanged() {
        assertEquals("delete_voice_active", begin(current = "/c/unrelated-visible", active = true))
        assertFalse(guard.isBusy())
        assertNull(begin())
        assertTrue(guard.isBusy())
        assertEquals("delete_busy", begin())
    }

    @Test fun unrelatedCommandCannotReleaseTheWriteLease() {
        assertNull(begin())
        guard.accept(result(true, "old-delete"))
        guard.accept(ChatGptWebEvent.CommandResult("new_conversation", true, "fixture", requestId = "delete-1"))
        assertTrue(guard.isBusy())
        guard.accept(result(true))
        assertFalse(guard.isBusy())
    }

    @Test fun currentDeletionKeepsVoiceBlockedUntilARealDifferentReadyPageArrives() {
        assertNull(begin(current = "/g/g-p-fixture/c/target"))
        guard.accept(result(true))
        now = ChatGptConversationDeletionGuard.LEASE_MS * 2
        guard.accept(ChatGptWebEvent.Snapshot(snapshot()))
        assertTrue(guard.isBusy())
        guard.accept(ChatGptWebEvent.Snapshot(snapshot("/").copy(composerReady = false)))
        assertTrue(guard.isBusy())
        guard.accept(ChatGptWebEvent.Snapshot(snapshot("/")))
        assertFalse(guard.isBusy())
    }

    @Test fun failureReleasesVoiceAndMissingReceiptIsBoundedAfterTheEntireRequestBudget() {
        assertNull(begin(current = "/c/target"))
        guard.accept(result(false))
        assertFalse(guard.isBusy())
        assertNull(begin())
        now = 25_000L
        assertTrue(guard.isBusy())
        now = ChatGptConversationDeletionGuard.LEASE_MS
        assertFalse(guard.isBusy())
    }

    @Test fun currentDraftAttachmentsAndRecordingAreProtected() {
        fun reject(value: ChatGptWebSnapshot = snapshot(), nativeDraft: String = "") =
            ChatGptConversationDeletionGuard.rejection("/g/g-p-fixture/c/target", value, nativeDraft)
        assertNull(reject())
        assertEquals("delete_conversation_busy", ChatGptConversationDeletionGuard.rejection("/c/target", snapshot(), "", sendPending = true))
        assertEquals("delete_conversation_busy", ChatGptConversationDeletionGuard.rejection("/c/target", snapshot("/c/other"), "", sendPending = true))
        assertEquals("delete_draft_present", reject(nativeDraft = "unsent"))
        assertEquals("delete_draft_present", reject(snapshot().copy(draft = "unsent")))
        assertEquals("delete_draft_present", reject(snapshot().copy(attachments = listOf(ChatGptWebAttachment("a", "fixture", "ready", true)))))
        for (busy in listOf(snapshot().copy(streaming = true), snapshot().copy(composerReady = false),
            snapshot().copy(dictationActive = true), snapshot().copy(dictationCaptureActive = true),
            snapshot().copy(dictationCapturePending = true))) {
            assertEquals("delete_conversation_busy", reject(busy))
        }
        assertNull(reject(snapshot("/c/other").copy(draft = "keep me"), "keep me"))
    }

    @Test fun deletionOnlyClearsTheMatchingCurrentIdentity() {
        val caches = ChatGptConversationDeletionCaches({}, { null }, {}, { "https://chatgpt.com/" }, {})
        val current = snapshot("/g/g-p-fixture/c/target").copy(messages = listOf(
            ChatGptWebMessage("fixture", "assistant", "fixture", "completed", emptyList())))
        assertNull(caches.accept(emptySet(), current))
        assertNull(caches.accept(setOf("other"), current))
        val next = checkNotNull(caches.accept(setOf("target"), current))
        assertEquals("https://chatgpt.com/", next.url)
        assertTrue(next.messages.isEmpty())
        assertFalse(next.composerReady)
        assertNull(caches.accept(setOf("target"), next))
    }

    @Test fun deleteReceiptBudgetIncludesAuthenticationAndReadOnlyReconciliation() {
        val observed = ChatGptWebObservedState(nowMs = { now })
        val command = observed.beginCommand("delete_conversation")
        now = 25_001L
        assertEquals(ChatGptWebObservedState.CommandRequest.PENDING, observed.snapshot().commandRequests.last().status)
        observed.accept(result(true, command.id))
        assertEquals(ChatGptWebObservedState.CommandRequest.SUCCEEDED, observed.snapshot().commandRequests.last().status)
    }

    @Test fun sendReadinessUsesTheSameLeaseWithoutReservingOrDiscardingTheDraft() {
        var sends = 0
        val transport = com.elon.app.OfficialPageWebChatSendTransport(
            ready = { !guard.isBusy() }, sendPrompt = { _, _, _ -> sends += 1; true }, requestReconciliation = {},
        )
        val sender = com.elon.app.WebChatSendCoordinator(transport, { _, _ -> }, {}, {})
        assertNull(begin())
        assertEquals(com.elon.app.WebChatSendCoordinator.DispatchOutcome.NOT_READY,
            sender.dispatch("unsent fixture", snapshot()) {}.outcome)
        assertNull(sender.prompt())
        assertEquals(0, sends)
        guard.accept(result(false))
        assertEquals(com.elon.app.WebChatSendCoordinator.DispatchOutcome.DISPATCHED,
            sender.dispatch("unsent fixture", snapshot()) {}.outcome)
        assertEquals(1, sends)
    }
}
