package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSessionRecoveryCoordinatorTest {
    @Test
    fun aNavigationFailureReloadsAtMostOnceAndExhaustsExactlyOnce() {
        val scheduler = FakeScheduler()
        var reloads = 0
        var exhausted = 0
        val coordinator = coordinator(
            scheduler = scheduler,
            retry = { reloads += 1; true },
            onExhausted = { exhausted += 1 },
        )

        coordinator.activate()
        coordinator.onFailure()

        assertEquals(2_000L, scheduler.nextDelay())
        scheduler.runNext()
        assertEquals(1, reloads)
        assertEquals(30_000L, scheduler.nextDelay())

        scheduler.runNext()
        assertEquals(1, reloads)
        assertEquals(1, exhausted)
        assertEquals(0, scheduler.size)

        coordinator.onFailure()
        assertEquals(1, exhausted)
    }

    @Test
    fun aFinishedPageRepairsTheBridgeBeforeReloading() {
        val scheduler = FakeScheduler()
        var repairs = 0
        var reloads = 0
        val coordinator = coordinator(
            scheduler = scheduler,
            repair = { repairs += 1; true },
            retry = { reloads += 1; true },
        )

        coordinator.activate()
        coordinator.onPageFinished()
        assertEquals(10_000L, scheduler.nextDelay())

        scheduler.runNext()
        assertEquals(1, repairs)
        assertEquals(0, reloads)
        assertEquals(10_000L, scheduler.nextDelay())

        coordinator.onReady()
        assertEquals(0, scheduler.size)
    }

    @Test
    fun failedBridgeRepairFallsThroughToOneDelayedReload() {
        val scheduler = FakeScheduler()
        var repairs = 0
        var reloads = 0
        val coordinator = coordinator(
            scheduler = scheduler,
            repair = { repairs += 1; false },
            retry = { reloads += 1; true },
        )

        coordinator.activate()
        coordinator.onPageFinished()
        scheduler.runNext()

        assertEquals(1, repairs)
        assertEquals(2_000L, scheduler.nextDelay())
        scheduler.runNext()
        assertEquals(1, reloads)
        assertEquals(30_000L, scheduler.nextDelay())
    }

    @Test
    fun navigationProgressExtendsTheStallDeadlineWithoutReloading() {
        val scheduler = FakeScheduler()
        var reloads = 0
        val coordinator = coordinator(scheduler, retry = { reloads += 1; true })

        coordinator.activate()
        coordinator.onNavigationStarted()
        val initialWatchdog = scheduler.nextRunnable()

        coordinator.onNavigationProgress(10)
        val firstProgressWatchdog = scheduler.nextRunnable()
        coordinator.onNavigationProgress(10)

        assertFalse(initialWatchdog === firstProgressWatchdog)
        assertTrue(firstProgressWatchdog === scheduler.nextRunnable())
        assertEquals(1, scheduler.size)
        assertEquals(30_000L, scheduler.nextDelay())
        assertEquals(0, reloads)
    }

    @Test
    fun readyCancelsPendingWorkAndResetsRecoveryBudget() {
        val scheduler = FakeScheduler()
        var reloads = 0
        val coordinator = coordinator(scheduler, retry = { reloads += 1; true })

        coordinator.activate()
        coordinator.onFailure()
        coordinator.onReady()

        assertEquals(0, scheduler.size)
        assertFalse(scheduler.runNext())

        coordinator.onFailure()
        scheduler.runNext()
        assertEquals(1, reloads)
    }

    @Test
    fun deactivationCancelsWatchdogsAndSuppressesRecovery() {
        val scheduler = FakeScheduler()
        var reloads = 0
        val coordinator = coordinator(scheduler, retry = { reloads += 1; true })

        coordinator.activate()
        coordinator.onNavigationStarted()
        assertEquals(30_000L, scheduler.nextDelay())

        coordinator.deactivate()
        coordinator.onFailure()

        assertEquals(0, scheduler.size)
        assertEquals(0, reloads)
    }

    @Test
    fun manualRetryIsImmediateAndUsesAFreshBudget() {
        val scheduler = FakeScheduler()
        var reloads = 0
        val coordinator = coordinator(scheduler, retry = { reloads += 1; true })

        assertFalse(coordinator.retryNow())
        coordinator.activate()
        coordinator.onFailure()

        assertTrue(coordinator.retryNow())
        assertEquals(1, reloads)
        assertEquals(30_000L, scheduler.nextDelay())
    }

    private fun coordinator(
        scheduler: FakeScheduler,
        retry: () -> Boolean,
        repair: () -> Boolean = { false },
        onExhausted: () -> Unit = {},
    ) = WebChatSessionRecoveryCoordinator(
        schedule = scheduler::schedule,
        cancel = scheduler::cancel,
        retry = retry,
        repair = repair,
        onExhausted = onExhausted,
        retryDelaysMs = listOf(2_000L),
        navigationStallTimeoutMs = 30_000L,
        bridgeReadinessTimeoutMs = 10_000L,
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

        fun nextRunnable(): Runnable? = tasks.minByOrNull(Task::delayMs)?.runnable

        fun runNext(): Boolean {
            val task = tasks.minByOrNull(Task::delayMs) ?: return false
            tasks.remove(task)
            task.runnable.run()
            return true
        }
    }
}
