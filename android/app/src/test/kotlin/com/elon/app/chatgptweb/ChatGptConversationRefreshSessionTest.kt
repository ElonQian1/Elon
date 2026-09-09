package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class ChatGptConversationRefreshSessionTest {
    @Test fun wireSnapshotParsesRequestOwnershipAndDefaultsPassiveSnapshotsToUnowned() {
        val id = ChatGptConversationRefreshOwner().begin(null)
        val raw = """{"schema":"${ChatGptWebProtocol.SCHEMA}","event":{"type":"conversation_snapshot",
            "conversations":[],"requestId":"$id","continueRefresh":true}}"""
        val event = ChatGptWebProtocol.parse(raw) as ChatGptWebEvent.ConversationList
        assertEquals(id, event.requestId)
        assertTrue(event.continueRefresh)
        val passive = ChatGptWebProtocol.parse(raw.replace("\"$id\"", "null")) as ChatGptWebEvent.ConversationList
        assertNull(passive.requestId)
    }

    @Test fun partialReadContinuesSameScopeWithoutAnotherUserRequest() {
        val f = Fixture()
        f.session.request("g-p-one")
        f.session.onSucceeded(true, 4)
        assertEquals(250L, f.scheduled.single().second)
        f.runNext()
        assertEquals(listOf("g-p-one", "g-p-one"), f.dispatched)
        f.session.onSucceeded(false, 6)
        assertFalse(f.coordinator.isBusy)
        assertTrue(f.scheduled.isEmpty())
    }

    @Test fun aFailedProjectPageRetriesThatProjectRatherThanGlobalHistory() {
        val f = Fixture()
        f.session.request("g-p-one")
        f.session.onFailed()
        f.runNext()
        assertEquals(listOf("g-p-one", "g-p-one"), f.dispatched)
    }

    @Test fun implicitNavigationScopeIsAlsoRetainedForRetry() {
        val f = Fixture()
        f.session.request(null)
        f.session.bindDispatchedScope("g-p-navigation")
        f.session.onFailed()
        f.runNext()
        assertEquals("g-p-navigation", f.dispatched.last())
    }

    @Test fun newlyRequestedProjectWinsOverContinuationOrRetry() {
        for (failed in listOf(false, true)) {
            val f = Fixture()
            f.session.request("g-p-one")
            f.session.request("g-p-two")
            if (failed) f.session.onFailed() else f.session.onSucceeded(true, 4)
            assertEquals(listOf("g-p-one", "g-p-two"), f.dispatched)
            assertTrue(f.scheduled.isEmpty())
        }
    }

    @Test fun lackOfProgressAndElapsedBudgetStopAutomaticContinuation() {
        for (expired in listOf(false, true)) {
            val f = Fixture()
            f.session.request(null)
            f.session.onSucceeded(true, 4)
            f.runNext()
            if (expired) f.time = 60_000L
            f.session.onSucceeded(true, if (expired) 5 else 4)
            assertFalse(f.coordinator.isBusy)
            assertTrue(f.scheduled.isEmpty())
        }
    }

    @Test fun continuationCountIsBoundedEvenIfEveryReplyClaimsProgress() {
        val f = Fixture()
        f.session.request(null)
        repeat(10) {
            f.session.onSucceeded(true, it + 1)
            f.runNext()
        }
        f.session.onSucceeded(true, 11)
        assertFalse(f.coordinator.isBusy)
    }

    @Test fun composerSuspensionRetainsProjectAndGenericReadySignalDoesNotOverwriteIt() {
        val f = Fixture()
        f.session.request("g-p-one")
        f.session.suspend(ChatGptConversationRefreshSuspension.COMPOSER_OPTIONS, true) {}
        f.session.requestDefaultIfMissing()
        f.session.resume(ChatGptConversationRefreshSuspension.COMPOSER_OPTIONS)
        assertEquals(listOf("g-p-one", "g-p-one"), f.dispatched)
    }

    @Test fun navigationCancelsScheduledContinuationAndCannotReviveIt() {
        val f = Fixture()
        f.session.request("g-p-one")
        f.session.onSucceeded(true, 4)
        val stale = f.scheduled.single().first
        f.session.yieldToUserNavigation()
        stale.run()
        assertEquals(1, f.dispatched.size)
        f.session.request(null)
        assertNull(f.dispatched.last())
    }

    @Test fun lateInflightPartialAfterNavigationDoesNotStartAnotherPage() {
        val f = Fixture()
        f.session.request("g-p-one")
        f.session.yieldToUserNavigation()
        f.session.onSucceeded(true, 4)
        assertFalse(f.coordinator.isBusy)
        assertEquals(1, f.dispatched.size)
    }

    @Test fun onlyMatchingRequestAndScopeOwnsARefresh() {
        val owner = ChatGptConversationRefreshOwner()
        val id = owner.begin("g-p-one")
        assertTrue(owner.matches(ChatGptWebEvent.ConversationList(emptyList(), scopeProjectId = "g-p-one", requestId = id)))
        assertFalse(owner.matches(ChatGptWebEvent.ConversationList(emptyList(), requestId = id)))
        assertFalse(owner.matches(ChatGptWebEvent.ConversationList(emptyList(), scopeProjectId = "g-p-one")))
        owner.begin("g-p-two")
        assertTrue(owner.isStaleNative(id))
        assertFalse(owner.matches(id))
        assertFalse(owner.isStaleNative("mcp-directory-request"))
        owner.clear()
        assertFalse(owner.matches(null))
    }

    private class Fixture {
        var time = 0L
        val scheduled = mutableListOf<Pair<Runnable, Long>>()
        val dispatched = mutableListOf<String?>()
        val coordinator = ChatGptConversationRefreshCoordinator(
            dispatch = { dispatch() },
            schedule = { task, delay -> scheduled.add(task to delay) },
            cancel = { task -> scheduled.removeAll { it.first === task } },
        )
        val session = ChatGptConversationRefreshSession(coordinator) { time }
        fun dispatch(): Boolean {
            val value = session.beginDispatch() ?: return false
            dispatched += value.projectId
            return true
        }
        fun runNext() = scheduled.removeAt(0).first.run()
    }
}
