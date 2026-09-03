package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptBackgroundExecutionControllerTest {
    @Test
    fun pausesAfterIdleAndResumesForTheNextInteraction() {
        val fixture = Fixture()

        fixture.controller.hostResumed()
        assertEquals(1, fixture.resumeCount)

        fixture.scheduler.runNext()
        assertEquals(1, fixture.pauseCount)

        fixture.controller.interactionRequested()
        assertEquals(2, fixture.resumeCount)
        fixture.scheduler.runNext()
        assertEquals(2, fixture.pauseCount)
    }

    @Test
    fun busyWorkDefersPauseUntilTheSessionSettles() {
        val fixture = Fixture()
        fixture.busy = true
        fixture.controller.hostResumed()

        fixture.scheduler.runNext()
        assertEquals(0, fixture.pauseCount)
        assertTrue(fixture.scheduler.hasTasks())

        fixture.busy = false
        fixture.scheduler.runNext()
        assertEquals(1, fixture.pauseCount)
    }

    @Test
    fun explicitBackgroundInteractionGetsABoundedExecutionLease() {
        val fixture = Fixture()
        fixture.controller.hostResumed()
        fixture.controller.hostPaused()

        assertEquals(1, fixture.pauseCount)
        assertFalse(fixture.scheduler.hasTasks())
        fixture.controller.interactionRequested()
        assertEquals(2, fixture.resumeCount)
        assertTrue(fixture.scheduler.hasTasks())

        fixture.scheduler.runNext()
        assertEquals(2, fixture.pauseCount)
        assertFalse(fixture.scheduler.hasTasks())
    }

    private class Fixture {
        val scheduler = Scheduler()
        var busy = false
        var resumeCount = 0
        var pauseCount = 0
        val controller = ChatGptBackgroundExecutionController(
            resumeExecution = { resumeCount += 1; true },
            pauseExecution = { pauseCount += 1 },
            isBusy = { busy },
            schedule = scheduler::schedule,
            cancel = scheduler::cancel,
            idleDelayMs = 10,
            busyRetryMs = 5,
        )
    }

    private class Scheduler {
        private val tasks = mutableListOf<Runnable>()

        fun schedule(task: Runnable, @Suppress("UNUSED_PARAMETER") delayMs: Long) {
            tasks += task
        }

        fun cancel(task: Runnable) {
            tasks.remove(task)
        }

        fun runNext() {
            tasks.removeFirstOrNull()?.run()
        }

        fun hasTasks(): Boolean = tasks.isNotEmpty()
    }
}
