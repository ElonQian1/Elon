package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptConversationRefreshCoordinatorTest {
    @Test
    fun failedRefreshRetriesUntilTheOfficialCollectionSucceeds() {
        val scheduled = mutableListOf<Scheduled>()
        var dispatches = 0
        val coordinator = coordinator(scheduled) {
            dispatches += 1
            true
        }

        assertTrue(coordinator.requestIfIdle())
        assertEquals(1, dispatches)
        coordinator.onFailed()
        assertEquals(1_000L, scheduled.single().delayMs)

        scheduled.removeAt(0).task.run()
        assertEquals(2, dispatches)
        coordinator.onSucceeded()

        assertFalse(coordinator.isBusy)
        assertTrue(scheduled.isEmpty())
    }

    @Test
    fun unavailablePageUsesBoundedBackoffWithoutClaimingAnInflightRequest() {
        val scheduled = mutableListOf<Scheduled>()
        var ready = true
        val coordinator = coordinator(scheduled) { ready }

        assertTrue(coordinator.requestNow())
        ready = false
        coordinator.onFailed()
        repeat(3) { scheduled.removeAt(0).task.run() }

        assertTrue(scheduled.isEmpty())
        assertFalse(coordinator.isBusy)
    }

    @Test
    fun explicitRefreshQueuesUntilThePageBecomesReady() {
        val scheduled = mutableListOf<Scheduled>()
        var ready = false
        var dispatches = 0
        val coordinator = coordinator(scheduled) {
            dispatches += 1
            ready
        }

        assertTrue(coordinator.requestNow())
        assertEquals(1, dispatches)
        assertEquals(1_000L, scheduled.single().delayMs)
        assertTrue(coordinator.isBusy)

        ready = true
        scheduled.removeAt(0).task.run()

        assertEquals(2, dispatches)
        assertTrue(coordinator.isBusy)
        coordinator.onSucceeded()
        assertFalse(coordinator.isBusy)
    }

    @Test
    fun refreshAfterCurrentQueuesUntilThePageBecomesReady() {
        val scheduled = mutableListOf<Scheduled>()
        var ready = false
        val coordinator = coordinator(scheduled) { ready }

        assertTrue(coordinator.requestAfterCurrent())
        assertEquals(1_000L, scheduled.single().delayMs)

        ready = true
        scheduled.removeAt(0).task.run()
        assertTrue(coordinator.isBusy)
        coordinator.onSucceeded()
        assertFalse(coordinator.isBusy)
    }

    @Test
    fun explicitRefreshCancelsAWaitingRetryAndStartsImmediately() {
        val scheduled = mutableListOf<Scheduled>()
        var dispatches = 0
        val coordinator = coordinator(scheduled) {
            dispatches += 1
            true
        }

        coordinator.requestNow()
        coordinator.onFailed()
        val staleRetry = scheduled.single().task

        assertTrue(coordinator.requestNow())
        assertEquals(2, dispatches)
        assertTrue(scheduled.isEmpty())
        staleRetry.run()
        assertEquals(2, dispatches)
    }

    @Test
    fun explicitRefreshJoinsAnInflightRequestWithoutDispatchingTwice() {
        val scheduled = mutableListOf<Scheduled>()
        var dispatches = 0
        val coordinator = coordinator(scheduled) {
            dispatches += 1
            true
        }

        assertTrue(coordinator.requestNow())
        assertTrue(coordinator.requestNow())

        assertEquals(1, dispatches)
        assertTrue(coordinator.isBusy)
    }

    @Test
    fun refreshAfterCurrentDispatchesAgainWhenTheInflightRequestCompletes() {
        val scheduled = mutableListOf<Scheduled>()
        var dispatches = 0
        val coordinator = coordinator(scheduled) {
            dispatches += 1
            true
        }

        assertTrue(coordinator.requestNow())
        assertTrue(coordinator.requestAfterCurrent())
        assertEquals(1, dispatches)

        coordinator.onSucceeded()

        assertEquals(2, dispatches)
        assertTrue(coordinator.isBusy)
        coordinator.onSucceeded()
        assertFalse(coordinator.isBusy)
    }

    @Test
    fun resetCancelsPendingWorkForANewDocument() {
        val scheduled = mutableListOf<Scheduled>()
        val coordinator = coordinator(scheduled) { true }

        coordinator.requestNow()
        coordinator.onFailed()
        coordinator.reset()

        assertFalse(coordinator.isBusy)
        assertTrue(scheduled.isEmpty())
    }

    @Test
    fun userNavigationDropsQueuedProjectRefreshAndItsStaleCompletion() {
        val scheduled = mutableListOf<Scheduled>()
        var dispatches = 0
        val coordinator = coordinator(scheduled) {
            dispatches += 1
            true
        }

        assertTrue(coordinator.requestNow())
        assertTrue(coordinator.requestAfterCurrent())
        coordinator.yieldToUserNavigation()
        coordinator.onSucceeded()

        assertEquals(1, dispatches)
        assertFalse(coordinator.isBusy)
        assertTrue(scheduled.isEmpty())
    }

    @Test
    fun userNavigationCancelsRetryWithoutSchedulingFromAStaleFailure() {
        val scheduled = mutableListOf<Scheduled>()
        val coordinator = coordinator(scheduled) { true }

        assertTrue(coordinator.requestNow())
        coordinator.yieldToUserNavigation()
        coordinator.onFailed()

        assertFalse(coordinator.isBusy)
        assertTrue(scheduled.isEmpty())
    }

    @Test
    fun scopedProjectRefreshSurvivesAConcurrentGenericRefresh() {
        assertEquals(
            "g-p-target",
            ChatGptConversationRefreshScopePolicy.select(
                pendingProjectId = "g-p-target",
                requestedProjectId = null,
                refreshBusy = true,
            ),
        )
        assertEquals(
            "g-p-latest",
            ChatGptConversationRefreshScopePolicy.select(
                pendingProjectId = "g-p-target",
                requestedProjectId = "g-p-latest",
                refreshBusy = true,
            ),
        )
        assertEquals(
            null,
            ChatGptConversationRefreshScopePolicy.select(
                pendingProjectId = "g-p-stale",
                requestedProjectId = null,
                refreshBusy = false,
            ),
        )
    }

    @Test
    fun suspendedSessionKeepsTheRequestWithoutDispatchingUntilExplicitlyResumed() {
        val scheduled = mutableListOf<Scheduled>()
        var dispatches = 0
        lateinit var session: ChatGptConversationRefreshSession
        val coordinator = coordinator(scheduled) {
            dispatches += 1
            session.beginDispatch() != null
        }
        session = ChatGptConversationRefreshSession(coordinator)

        session.suspend({})
        assertTrue(session.request("g-p-target"))
        assertEquals(0, dispatches)

        session.resume()
        assertTrue(session.request("g-p-target"))
        assertEquals(1, dispatches)
    }

    @Test
    fun autoRefreshDecisionSuppressesBackgroundWorkDuringAUserAction() {
        val session = ChatGptConversationRefreshSession(coordinator(mutableListOf()) { true })
        session.suspend({})

        assertEquals(
            ChatGptConversationAutoRefreshDecision.Action.NONE,
            session.autoRefreshDecision(
                postVoiceRefresh = true,
                supported = true,
                projectRefreshNeeded = true,
                officialRefreshNeeded = true,
            ).action,
        )
        session.resume()
        val decision = session.autoRefreshDecision(
            postVoiceRefresh = true,
            supported = true,
            projectRefreshNeeded = false,
            officialRefreshNeeded = false,
        )
        assertEquals(ChatGptConversationAutoRefreshDecision.Action.AFTER_CURRENT, decision.action)
        assertTrue(decision.consumePostVoiceRefresh)
    }

    private fun coordinator(
        scheduled: MutableList<Scheduled>,
        dispatch: () -> Boolean,
    ) = ChatGptConversationRefreshCoordinator(
        dispatch = dispatch,
        schedule = { task, delayMs -> scheduled += Scheduled(task, delayMs) },
        cancel = { task -> scheduled.removeAll { it.task === task } },
        retryDelaysMs = listOf(1_000L, 2_000L, 3_000L),
    )

    private data class Scheduled(val task: Runnable, val delayMs: Long)
}
