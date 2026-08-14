package com.elon.app.googleweb

import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Test

class GoogleWebConversationIndexPolicyTest {
    @Test
    fun aFollowUpKeepsThePreferredConversationPath() {
        val initial = GoogleWebConversationIndexPolicy.upsert(
            records = emptyList(),
            restorableUrl = "https://www.google.com/search?q=first&udm=50",
            title = "first",
            date = LocalDate.parse("2026-08-14"),
            preferredPath = null,
        )

        val followUp = GoogleWebConversationIndexPolicy.upsert(
            records = initial.records,
            restorableUrl = "https://www.google.com/search?q=second&udm=50",
            title = "second",
            date = LocalDate.parse("2026-08-15"),
            preferredPath = initial.path,
        )

        assertEquals(initial.path, followUp.path)
        assertEquals(1, followUp.records.size)
        assertEquals("first", followUp.records.single().title)
        assertEquals(setOf("2026-08-14", "2026-08-15"), followUp.records.single().activityDates)
    }

    @Test
    fun aRestoredUrlResolvesToTheExistingOpaquePath() {
        val record = GoogleWebConversationRecord(
            id = "a".repeat(64),
            title = "query",
            path = "/google-ai-mode/conversation/${"a".repeat(64)}",
            restorableUrl = "https://www.google.com/search?q=query&udm=50",
            activityDates = emptySet(),
        )

        assertEquals(record.path, GoogleWebConversationIndexPolicy.currentPath(
            listOf(record),
            record.restorableUrl,
        ))
    }
}
