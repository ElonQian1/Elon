package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSessionRecoveryCoordinatorTest {
    @Test
    fun retriesAreBoundedAndExhaustExactlyOnce() {
        val scheduler = FakeScheduler()
        var retries = 0
        var exhausted = 0
        val coordinator = coordinator(
            scheduler = scheduler,
            retry = { retries += 1; true },
            onExhausted = { exhausted += 1 },
        )

        coordinator.activate()
        coordinator.onFailure()

        assertEquals(2_000L, scheduler.nextDelay())
        repeat(6) { scheduler.runNext() }

        assertEquals(3, retries)
        assertEquals(1, exhausted)
        assertEquals(0, scheduler.size)

        coordinator.onFailure()
        assertEquals(0, scheduler.size)
        assertEquals(1, exhausted)
    }

    @Test
    fun readyCancelsPendingWorkAndResetsRetryBudget() {
        val scheduler = FakeScheduler()
        var retries = 0
        val coordinator = coordinator(scheduler, retry = { retries += 1; true })

        coordinator.activate()
        coordinator.onFailure()
        coordinator.onReady()

        assertEquals(0, scheduler.size)
        assertFalse(scheduler.runNext())

        coordinator.onFailure()
        assertEquals(2_000L, scheduler.nextDelay())
        scheduler.runNext()
        assertEquals(1, retries)
    }

    @Test
    fun deactivationCancelsWatchdogsAndSuppressesRetries() {
        val scheduler = FakeScheduler()
        var retries = 0
        val coordinator = coordinator(scheduler, retry = { retries += 1; true })

        coordinator.activate()
        coordinator.onNavigationStarted()
        assertEquals(20_000L, scheduler.nextDelay())

        coordinator.deactivate()
        coordinator.onFailure()

        assertEquals(0, scheduler.size)
        assertEquals(0, retries)
    }

    @Test
    fun readinessTimeoutSchedulesRecoveryInsteadOfLoopingImmediately() {
        val scheduler = FakeScheduler()
        var retries = 0
        val coordinator = coordinator(scheduler, retry = { retries += 1; true })

        coordinator.activate()
        coordinator.onPageFinished()
        assertEquals(20_000L, scheduler.nextDelay())

        scheduler.runNext()
        assertEquals(2_000L, scheduler.nextDelay())
        assertEquals(0, retries)

        scheduler.runNext()
        assertEquals(1, retries)
        assertEquals(20_000L, scheduler.nextDelay())
    }

    @Test
    fun manualRetryIsImmediateAndUsesFreshBudget() {
        val scheduler = FakeScheduler()
        var retries = 0
        val coordinator = coordinator(scheduler, retry = { retries += 1; true })

        assertFalse(coordinator.retryNow())
        coordinator.activate()
        coordinator.onFailure()

        assertTrue(coordinator.retryNow())
        assertEquals(1, retries)
        assertEquals(20_000L, scheduler.nextDelay())

        scheduler.runNext()
        assertEquals(2_000L, scheduler.nextDelay())
    }

    private fun coordinator(
        scheduler: FakeScheduler,
        retry: () -> Boolean,
        onExhausted: () -> Unit = {},
    ) = WebChatSessionRecoveryCoordinator(
        schedule = scheduler::schedule,
        cancel = scheduler::cancel,
        retry = retry,
        onExhausted = onExhausted,
        retryDelaysMs = listOf(2_000L, 5_000L, 15_000L),
        readinessTimeoutMs = 20_000L,
    )

    private class FakeScheduler {
        private data class Task(val runnable: Runnable, val delayMs: Long)

        private val tasks = mutableListOf<Task>()
        val size: Int get() = tasks.size

        fun schedule(runnable: Runnable, delayMs: Long) {
            tasks += Task(runnable, delayMs)
        }

        fun cancel(runnable: Runnable) {
            tasks.removeAll { it.runnable === runnable }
        }

        fun nextDelay(): Long? = tasks.minByOrNull(Task::delayMs)?.delayMs

        fun runNext(): Boolean {
            val task = tasks.minByOrNull(Task::delayMs) ?: return false
            tasks.remove(task)
            task.runnable.run()
            return true
        }
    }
}
