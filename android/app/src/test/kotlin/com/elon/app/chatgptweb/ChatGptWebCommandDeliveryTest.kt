package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebCommandDeliveryTest {
    @Test fun warmDeliveryDoesNotRepairOrSynthesizeProviderSuccess() {
        val f = Fixture()
        f.send()
        f.reply("entered")
        assertEquals(1, f.calls.size)
        assertTrue(f.repairs.isEmpty())
        assertTrue(f.failures.isEmpty())
        assertTrue(f.timers.isEmpty())
        assertTrue(f.events.isEmpty())
    }

    @Test fun missingBridgeRepairsOnceThenInvokesTheSameCommand() {
        val f = Fixture()
        f.send("first")
        f.reply("missing")
        assertEquals(1, f.repairs.size)
        f.repairs.single().result(true)
        assertEquals(listOf("first", "first"), f.calls.map { it.command })
        f.reply("entered")
        assertTrue(f.failures.isEmpty())
        assertEquals(listOf("repair_started", "repair_delivered"), f.events)
    }

    @Test fun concurrentMissingCallsJoinOneRepair() {
        val f = Fixture()
        f.send("read"); f.reply("missing")
        f.send("send"); f.reply("missing")
        assertEquals(1, f.repairs.size)
        f.repairs.single().result(true)
        assertEquals(listOf("read", "send", "read", "send"), f.calls.map { it.command })
        f.calls[2].result("\"entered\""); f.calls[3].result("\"entered\"")
        assertTrue(f.timers.isEmpty())
    }

    @Test fun duplicateOrLateCallbacksCannotDispatchAgain() {
        val f = Fixture()
        f.send(); val first = f.calls.single()
        first.result("\"missing\""); first.result("\"missing\"")
        assertEquals(1, f.repairs.size)
        val repair = f.repairs.single()
        repair.result(true); repair.result(true); first.result("\"missing\"")
        assertEquals(2, f.calls.size)
        f.reply("entered"); f.reply("missing")
        assertEquals(2, f.calls.size)
    }

    @Test fun secondMissingResultIsKnownUnsentAndDoesNotRepairForever() {
        val f = Fixture()
        f.send(); f.reply("missing"); f.repairs.single().result(true); f.reply("missing")
        assertEquals(listOf("missing"), f.failures)
        assertEquals(1, f.repairs.size)
        assertTrue(f.timers.isEmpty())
    }

    @Test fun ambiguousResultsNeverRepairOrReleaseAsKnownUnsent() {
        for (value in listOf(null, "null", "\"unknown\"", "unexpected", "\"entered\"")) {
            val f = Fixture()
            f.send(); f.calls.single().result(value)
            assertTrue(f.repairs.isEmpty()); assertTrue(f.failures.isEmpty())
            assertTrue(f.timers.isEmpty())
        }
    }

    @Test fun evaluationTimeoutForbidsLateInvocationAndReplay() {
        val f = Fixture()
        f.send(); val call = f.calls.single()
        f.fireTimers()
        assertFalse(call.allowed())
        call.result("\"missing\"")
        assertTrue(f.repairs.isEmpty()); assertTrue(f.failures.isEmpty())
    }

    @Test fun repairTimeoutReportsUnsentButLateRepairNeverSends() {
        val f = Fixture()
        f.send(); f.reply("missing")
        val repair = f.repairs.single()
        f.fireTimers()
        assertEquals(listOf("repair_timeout"), f.failures)
        assertFalse(repair.allowed())
        repair.result(true)
        assertEquals(1, f.calls.size)
    }

    @Test fun navigationDuringRepairDoesNotInvokeOrRestoreOverTheNewConversation() {
        val f = Fixture()
        f.send(); f.reply("missing")
        val repair = f.repairs.single()
        f.owner = f.owner!!.copy(href = "https://chatgpt.com/c/other")
        assertFalse(repair.allowed()); repair.result(true)
        assertEquals(1, f.calls.size); assertTrue(f.failures.isEmpty())
    }

    @Test fun pauseInvalidatesQueuedDeliveryAndRepairEvenOnTheSameUrl() {
        val f = Fixture()
        f.send(); f.reply("missing")
        val repair = f.repairs.single()
        f.delivery.invalidate()
        assertFalse(repair.allowed()); repair.result(true)
        assertEquals(1, f.calls.size); assertTrue(f.timers.isEmpty())
        f.send("after_resume"); f.reply("entered")
        assertEquals(2, f.calls.size)
    }

    @Test fun failureToRepairIsKnownUnsentAndExplicitRetryCanStartFresh() {
        val f = Fixture()
        f.send(); f.reply("missing"); f.repairs.single().result(false)
        assertEquals(listOf("repair_failed"), f.failures)
        f.send(); f.reply("entered")
        assertEquals(2, f.calls.size)
    }

    @Test fun pendingCommandsAreBoundedAndDoNotKeepPromptBuffersForever() {
        val f = Fixture()
        repeat(33) { f.send() }
        assertEquals(32, f.calls.size); assertEquals(listOf("busy"), f.failures)
        f.fireTimers(); f.send()
        assertEquals(33, f.calls.size)
    }

    @Test fun absentBindingDoesNotQueueOrInjectIntoAnotherPage() {
        val f = Fixture()
        f.owner = null
        f.send()
        assertEquals(listOf("unavailable"), f.failures)
        assertTrue(f.calls.isEmpty()); assertTrue(f.repairs.isEmpty()); assertTrue(f.timers.isEmpty())
    }

    @Test fun evaluatorThrowsNeverAllowReplayEvenAfterSynchronousAcknowledgement() {
        for (ack in listOf(null, "\"entered\"")) {
            val f = Fixture()
            f.invokeResultThenThrow = ack
            f.throwOnInvoke = true
            f.send()
            assertEquals(1, f.calls.size)
            assertTrue(f.failures.isEmpty()); assertTrue(f.repairs.isEmpty()); assertTrue(f.timers.isEmpty())
        }
    }

    @Test fun expiredWaitersForbidQueuedBridgeRepairBeforeItsOwnDeadline() {
        val f = Fixture()
        f.send(); f.reply("missing")
        val deadline = f.timers.first()
        f.timers.remove(deadline); deadline.run()
        assertFalse(f.repairs.single().allowed())
        f.repairs.single().result(true)
        assertEquals(1, f.calls.size); assertTrue(f.timers.isEmpty())
    }

    private data class Call(val command: String, val allowed: () -> Boolean, val result: (String?) -> Unit)
    private data class Repair(val allowed: () -> Boolean, val result: (Boolean) -> Unit)
    private class Fixture {
        var owner: ChatGptWebCommandDelivery.Binding? = ChatGptWebCommandDelivery.Binding("doc_test", "https://chatgpt.com/c/fixture")
        val calls = mutableListOf<Call>()
        val repairs = mutableListOf<Repair>()
        val failures = mutableListOf<String>()
        val events = mutableListOf<String>()
        val timers = linkedSetOf<Runnable>()
        var throwOnInvoke = false
        var invokeResultThenThrow: String? = null
        val delivery = ChatGptWebCommandDelivery(
            binding = { owner },
            invoke = { command, _, allowed, callback ->
                calls += Call(command, allowed, callback)
                if (throwOnInvoke) {
                    invokeResultThenThrow?.let(callback)
                    throw IllegalStateException("synthetic")
                }
            },
            repair = { _, allowed, callback -> repairs += Repair(allowed, callback) },
            schedule = { task, _ -> timers += task }, cancel = { timers -= it },
            observe = { events += it },
        )
        fun send(value: String = "fixture") = delivery.send(value) { failures += it }
        fun reply(value: String) = calls.last().result("\"$value\"")
        fun fireTimers() = timers.toList().forEach { if (timers.remove(it)) it.run() }
    }
}
