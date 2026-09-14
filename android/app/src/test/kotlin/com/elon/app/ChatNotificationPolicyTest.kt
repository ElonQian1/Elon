package com.elon.app

import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.Callable
import java.util.concurrent.Executors

class ChatNotificationPolicyTest {
    private val policy = ChatNotificationPolicy()
    private fun receive(
        id: String = "m1", at: String = "2026-09-15T01:00:00Z", text: String = "hello",
        visible: Boolean = false, group: Boolean = false, now: Long = 0L,
        conversation: String = "account:friend:one",
    ) = policy.receive(conversation, id, at, text, visible, group, now)

    @Test fun realtimeAndAbbreviatedSummaryAlertOnlyOnceInEitherOrder() {
        assertTrue(receive(text = "a long attachment message").alert)
        assertFalse(receive(id = "", text = "[附件]", now = 15_000).show)
        val reverse = ChatNotificationPolicy()
        assertTrue(reverse.receive("g", "", "2026-09-15T00:00:00Z", "[图片]", false, true, 0).alert)
        assertFalse(reverse.receive("g", "m", "2026-09-15T08:00:00+08:00", "", false, true, 15_000).show)
        assertFalse(reverse.receive("g", "m", "2026-09-15T00:00:00Z", "", false, true, 30_000).show)
    }

    @Test fun distinctIdsAtTheSameTimestampRemainDistinctMessages() {
        assertTrue(receive().show)
        assertTrue(receive(id = "m2").show)
        assertFalse(receive().show)
    }

    @Test fun repeatedTextAtDifferentTimesIsNotLost() {
        assertTrue(receive(id = "").show)
        assertTrue(receive(id = "", at = "2026-09-15T01:01:00Z", now = 60_000).alert)
    }

    @Test fun messageReadInVisibleConversationDoesNotAlertOnLaterPoll() {
        assertFalse(receive(visible = true).show)
        assertFalse(receive(id = "", now = 15_000).show)
        assertTrue(receive(id = "m2", at = "2026-09-15T01:01:00Z", now = 60_000).alert)
    }

    @Test fun privateBurstUpdatesSilentlyAndNewMessageAlertsAfterThreeSeconds() {
        assertTrue(receive().alert)
        val second = receive(id = "m2", now = 2_999)
        assertTrue(second.show)
        assertFalse(second.alert)
        assertTrue(receive(id = "m3", now = 3_000).alert)
    }

    @Test fun groupBurstHasTenSecondCooldownWithoutPostponingIt() {
        assertTrue(receive(group = true).alert)
        assertFalse(receive(id = "m2", group = true, now = 9_999).alert)
        assertTrue(receive(id = "m3", group = true, now = 10_000).alert)
    }

    @Test fun anotherConversationIsIndependentExceptForShortGlobalCooldown() {
        assertTrue(receive().alert)
        assertFalse(receive(conversation = "account:group:two", now = 500).alert)
        assertTrue(receive(conversation = "account:friend:three", now = 1_500).alert)
        assertTrue(receive(conversation = "another-account:friend:one", now = 3_000).alert)
    }

    @Test fun racingPollAndRealtimeHaveOnlyOneWinner() {
        val pool = Executors.newFixedThreadPool(4)
        try {
            val results = pool.invokeAll((0..19).map { index -> Callable {
                receive(id = if (index % 2 == 0) "m1" else "")
            } }).map { it.get() }
            assertEquals(1, results.count { it.show })
            assertEquals(1, results.count { it.alert })
        } finally { pool.shutdownNow() }
    }

    @Test fun invalidOrMissingTimestampIsSafeAndMessageIdsStillDeduplicate() {
        assertTrue(receive(at = "").show)
        assertFalse(receive(at = "", text = "updated text").show)
        assertTrue(receive(id = "m2", at = "not-a-date").show)
    }
}
