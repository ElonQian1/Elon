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
                restorableUrl = "https://www.google.com/search?q=private&udm=50&csuir=thread-private",
                activityDates = setOf("2026-08-14"),
            ),
        )

        val decoded = GoogleWebConversationCodec.decodeCache(
            GoogleWebConversationCodec.encode(source, officialCachedAtMs = 123_456L),
        )

        assertEquals(source, decoded.records)
        assertEquals(123_456L, decoded.officialCachedAtMs)
    }

    @Test
    fun migratesLegacyDirectoryAsNeedingAnOfficialRefresh() {
        val id = "c".repeat(64)
        val decoded = GoogleWebConversationCodec.decodeCache(
            """{"schema":"elon.google_web.conversation_index.v1","conversations":[{"id":"$id","title":"legacy","path":"/google-ai-mode/conversation/$id","url":"https://www.google.com/search?q=legacy&udm=50&csuir=thread-legacy","activity_dates":[]}]}""",
        )

        assertEquals(1, decoded.records.size)
        assertEquals(0L, decoded.officialCachedAtMs)
    }

    @Test
    fun rejectsNonGoogleAndMalformedRecords() {
        val decoded = GoogleWebConversationCodec.decode(
            """{"schema":"elon.google_web.conversation_index.v1","conversations":[{"id":"${"b".repeat(64)}","title":"bad","path":"/google-ai-mode/conversation/${"b".repeat(64)}","url":"https://evil.example/search?udm=50","activity_dates":[]}]}""",
        )

        assertTrue(decoded.isEmpty())
    }

    @Test
    fun dropsLegacyPromptExecutionUrlsInsteadOfReplayingThemAsHistory() {
        val id = "d".repeat(64)
        val decoded = GoogleWebConversationCodec.decode(
            """{"schema":"elon.google_web.conversation_index.v2","conversations":[{"id":"$id","title":"transient","path":"/google-ai-mode/conversation/$id","url":"https://www.google.com/search?q=first&udm=50&aep=11","activity_dates":[]}]}""",
        )

        assertTrue(decoded.isEmpty())
    }

    @Test
    fun collapsesRecordsThatDifferOnlyByVolatileTrackingParameters() {
        val first = "a".repeat(64)
        val second = "b".repeat(64)
        val decoded = GoogleWebConversationCodec.decode(
            """{"schema":"elon.google_web.conversation_index.v1","conversations":[{"id":"$first","title":"same","path":"/google-ai-mode/conversation/$first","url":"https://www.google.com/search?q=same&udm=50&csuir=thread-same&sei=one","activity_dates":[]},{"id":"$second","title":"duplicate","path":"/google-ai-mode/conversation/$second","url":"https://www.google.com/search?ved=two&q=same&udm=50&csuir=thread-same","activity_dates":[]}]}""",
        )

        assertEquals(1, decoded.size)
        assertEquals(
            "https://www.google.com/search?q=same&udm=50&csuir=thread-same",
            decoded.single().restorableUrl,
        )
    }
}
