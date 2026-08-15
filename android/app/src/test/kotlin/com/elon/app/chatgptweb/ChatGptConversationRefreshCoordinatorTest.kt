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
    fun resetCancelsPendingWorkForANewDocument() {
        val scheduled = mutableListOf<Scheduled>()
        val coordinator = coordinator(scheduled) { true }

        coordinator.requestNow()
        coordinator.onFailed()
        coordinator.reset()

        assertFalse(coordinator.isBusy)
        assertTrue(scheduled.isEmpty())
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
