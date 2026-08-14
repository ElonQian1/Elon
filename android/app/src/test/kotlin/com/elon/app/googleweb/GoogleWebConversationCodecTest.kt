package com.elon.app.googleweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebConversationCodecTest {
    @Test
    fun roundTripsOpaqueLocalPathsAndRestorableOfficialUrls() {
        val id = "a".repeat(64)
        val source = listOf(
            GoogleWebConversationRecord(
                id = id,
                title = "测试搜索",
                path = "/google-ai-mode/conversation/$id",
                restorableUrl = "https://www.google.com/search?udm=50&q=private",
                activityDates = setOf("2026-08-14"),
            ),
        )

        assertEquals(source, GoogleWebConversationCodec.decode(GoogleWebConversationCodec.encode(source)))
    }

    @Test
    fun rejectsNonGoogleAndMalformedRecords() {
        val decoded = GoogleWebConversationCodec.decode(
            """{"schema":"elon.google_web.conversation_index.v1","conversations":[{"id":"${"b".repeat(64)}","title":"bad","path":"/google-ai-mode/conversation/${"b".repeat(64)}","url":"https://evil.example/search?udm=50","activity_dates":[]}]}""",
        )

        assertTrue(decoded.isEmpty())
    }
}
