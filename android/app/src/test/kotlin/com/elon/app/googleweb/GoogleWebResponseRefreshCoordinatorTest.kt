package com.elon.app.googleweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebResponseRefreshCoordinatorTest {
    @Test
    fun confirmedSendPollsUntilACompletedAssistantReplyAppears() {
        val harness = Harness(listOf(10L, 20L, 30L))

        harness.start("new question")
        assertTrue(harness.coordinator.isActive)
        harness.runNext()
        assertEquals(1, harness.snapshots)

        harness.coordinator.onSnapshot(
            latestUserPrompt = "new question",
            assistantObserved = true,
            streaming = false,
        )

        assertFalse(harness.coordinator.isActive)
        assertTrue(harness.scheduled.isEmpty())
    }

    @Test
    fun streamingAssistantKeepsTheBoundedPollingActive() {
        val harness = Harness(listOf(10L, 20L))

        harness.start("new question")
        harness.runNext()
        harness.coordinator.onSnapshot(
            latestUserPrompt = "new question",
            assistantObserved = true,
            streaming = true,
        )
        harness.runNext()

        assertEquals(2, harness.snapshots)
        assertFalse(harness.coordinator.isActive)
    }

    @Test
    fun pollingStopsAfterTheConfiguredDelaysWithoutAnAnswer() {
        val harness = Harness(listOf(10L, 20L, 30L))

        harness.start("new question")
        while (harness.scheduled.isNotEmpty()) harness.runNext()

        assertEquals(3, harness.snapshots)
        assertFalse(harness.coordinator.isActive)
    }

    @Test
    fun aNewSendCancelsThePreviousGeneration() {
        val harness = Harness(listOf(10L, 20L))

        harness.start("old question")
        val stale = harness.scheduled.single().task
        harness.start("new question")
        stale.run()

        assertEquals(0, harness.snapshots)
        assertEquals(1, harness.scheduled.size)
        harness.runNext()
        assertEquals(1, harness.snapshots)
    }

    @Test
    fun explicitStopCancelsPendingPageReads() {
        val harness = Harness(listOf(10L))

        harness.start("new question")
        harness.coordinator.stop()

        assertFalse(harness.coordinator.isActive)
        assertTrue(harness.scheduled.isEmpty())
        assertEquals(0, harness.snapshots)
    }

    @Test
    fun aCompletedPreviousTurnCannotStopPollingForTheNewPrompt() {
        val harness = Harness(listOf(10L, 20L))

        harness.start("new question")
        harness.coordinator.onSnapshot(
            latestUserPrompt = "old question",
            assistantObserved = true,
            streaming = false,
        )

        assertTrue(harness.coordinator.isActive)
        harness.runNext()
        assertEquals(1, harness.snapshots)
    }

    private class Harness(delays: List<Long>) {
        var snapshots = 0
        val scheduled = mutableListOf<Scheduled>()
        val coordinator = GoogleWebResponseRefreshCoordinator(
            requestSnapshot = { snapshots += 1 },
            schedule = { task, delayMs -> scheduled += Scheduled(task, delayMs) },
            cancel = { task -> scheduled.removeAll { it.task === task } },
            delaysMs = delays,
        )

        fun start(prompt: String) {
            coordinator.onSendStarted(prompt)
            coordinator.onSendConfirmed()
        }

        fun runNext() {
            val next = scheduled.removeAt(0)
            next.task.run()
        }
    }

    private data class Scheduled(val task: Runnable, val delayMs: Long)
}
