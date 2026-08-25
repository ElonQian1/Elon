package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceActivationMonitorTest {
    @Test
    fun lateEvidenceConfirmsWithoutWaitingForTheNextWatchdog() {
        val fixture = Fixture()
        fixture.monitor.begin()

        fixture.decision = WebChatRealtimeVoiceActivationDecision.Active
        fixture.monitor.observeEvent()

        assertEquals(1, fixture.confirmed)
        assertTrue(fixture.scheduler.hasTasks())
        fixture.scheduler.runAll()
        assertEquals(1, fixture.confirmed)
    }

    @Test
    fun exhaustedWatchdogKeepsAcceptingFutureEvidence() {
        val fixture = Fixture()
        fixture.monitor.begin()

        fixture.scheduler.runAll()
        assertEquals(1, fixture.exhausted)
        assertEquals(2, fixture.controlRefreshes)
        assertEquals(0, fixture.rejected)

        fixture.decision = WebChatRealtimeVoiceActivationDecision.Active
        fixture.monitor.observeEvent()
        assertEquals(1, fixture.confirmed)
    }

    @Test
    fun explicitPermissionRejectionStillFailsClosed() {
        val fixture = Fixture()
        fixture.monitor.begin()
        fixture.decision = WebChatRealtimeVoiceActivationDecision.Rejected("denied")

        fixture.monitor.observeEvent()

        assertEquals(1, fixture.rejected)
        assertEquals("denied", fixture.rejectionDetail)
        assertEquals(0, fixture.confirmed)
    }

    private class Fixture {
        val scheduler = Scheduler()
        var decision: WebChatRealtimeVoiceActivationDecision =
            WebChatRealtimeVoiceActivationDecision.Wait("pending")
        var confirmed = 0
        var rejected = 0
        var rejectionDetail = ""
        var exhausted = 0
        var controlRefreshes = 0
        val monitor = WebChatRealtimeVoiceActivationMonitor(
            schedule = scheduler::schedule,
            observeActivation = { decision },
            requestControls = { controlRefreshes += 1 },
            onConfirmed = { confirmed += 1 },
            onRejected = { detail ->
                rejected += 1
                rejectionDetail = detail
            },
            onWatchdogExhausted = { exhausted += 1 },
            watchdogDelaysMs = longArrayOf(1L, 2L, 3L, 4L),
            controlRefreshInterval = 3,
        )
    }

    private class Scheduler {
        private val tasks = ArrayDeque<Runnable>()

        fun schedule(task: Runnable, delayMs: Long) {
            require(delayMs > 0L)
            tasks.addLast(task)
        }

        fun hasTasks(): Boolean = tasks.isNotEmpty()

        fun runAll() {
            var guard = 0
            while (tasks.isNotEmpty() && guard < 100) {
                tasks.removeFirst().run()
                guard += 1
            }
            assertTrue(guard < 100)
        }
    }
}
