package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptComposerOptionRequestCoordinatorTest {
    @Test
    fun closesExistingMenuBeforeOpeningAndCollectingTheRequestedSection() {
        val fixture = Fixture()

        assertTrue(fixture.coordinator.request("model", "model-1"))
        assertEquals(listOf("prepare:model", "dismiss:null"), fixture.events)
        assertEquals(80L, fixture.scheduled.single().delayMs)

        fixture.runNext()
        assertEquals("dispatch:model:model-1", fixture.events.last())
        assertTrue(fixture.coordinator.scheduleCollection("model"))
        assertEquals(220L, fixture.scheduled.single().delayMs)

        fixture.runNext()
        assertEquals("collect:model", fixture.events.last())
    }

    @Test
    fun latestRequestWinsAndTheQueuedMcpRequestGetsAReceipt() {
        val fixture = Fixture()

        fixture.coordinator.request("model", "model-1")
        fixture.coordinator.request("tools", "tools-2")

        assertEquals(listOf("model-1:model"), fixture.superseded)
        assertEquals(1, fixture.scheduled.size)
        fixture.runNext()
        assertEquals("dispatch:tools:tools-2", fixture.events.last())
        assertFalse(fixture.events.contains("dispatch:model:model-1"))
    }

    @Test
    fun officialDismissReceiptOpensWithoutWaitingForTheFallbackDelay() {
        val fixture = Fixture()

        fixture.coordinator.request("model", "model-1")
        fixture.coordinator.onMenuDismissed()

        assertTrue(fixture.scheduled.isEmpty())
        assertEquals("dispatch:model:model-1", fixture.events.last())
    }

    @Test
    fun dismissCancelsQueuedAndCollectionWorkBeforeClosingTheOfficialMenu() {
        val fixture = Fixture()

        fixture.coordinator.request("tools", "tools-1")
        fixture.runNext()
        assertTrue(fixture.coordinator.scheduleCollection("tools"))
        fixture.coordinator.dismiss("dismiss-2")

        assertTrue(fixture.scheduled.isEmpty())
        assertEquals("dismiss:dismiss-2", fixture.events.last())
    }

    @Test
    fun completedSectionCannotScheduleAnotherStaleCollection() {
        val fixture = Fixture()

        fixture.coordinator.request("model")
        fixture.runNext()
        fixture.coordinator.complete("model")

        assertFalse(fixture.coordinator.scheduleCollection("model"))
        assertTrue(fixture.scheduled.isEmpty())
    }

    @Test
    fun unsupportedSectionHasNoSideEffects() {
        val fixture = Fixture()

        assertFalse(fixture.coordinator.request("unknown", "bad"))

        assertTrue(fixture.events.isEmpty())
        assertTrue(fixture.scheduled.isEmpty())
    }

    private class Fixture {
        val events = mutableListOf<String>()
        val superseded = mutableListOf<String>()
        val scheduled = mutableListOf<Scheduled>()
        val coordinator = ChatGptComposerOptionRequestCoordinator(
            dismissMenu = { requestId -> events += "dismiss:$requestId" },
            dispatchRequest = { section, requestId -> events += "dispatch:$section:$requestId" },
            collectOptions = { section -> events += "collect:$section" },
            schedule = { task, delayMs -> scheduled += Scheduled(task, delayMs) },
            cancel = { task -> scheduled.removeAll { it.task === task } },
            prepareSection = { section -> events += "prepare:$section" },
            failSuperseded = { requestId, section -> superseded += "$requestId:$section" },
            closeSettleMs = 80L,
            menuSettleMs = 220L,
        )

        fun runNext() {
            scheduled.removeAt(0).task.run()
        }
    }

    private data class Scheduled(val task: Runnable, val delayMs: Long)
}
