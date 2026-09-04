package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptConversationHistoryCodecTest {
    @Test
    fun roundTripsOnlySafeBoundedConversationMetadata() {
        val encoded = ChatGptConversationHistoryCodec.encode(
            ChatGptConversationHistoryCache(
                conversations = listOf(
                    ChatGptWebConversation(
                        "one",
                        "第一场会话",
                        "/c/one",
                        active = true,
                        groupLabel = "今天",
                        activityDates = setOf("2026-08-14"),
                        pinned = true,
                    ),
                    ChatGptWebConversation("two", "第二场会话", "/c/two", active = false),
                ),
                savedAtMs = 1234L,
                projectCachedAtMs = mapOf("g-p-demo" to 1200L),
            ),
        )

        val decoded = ChatGptConversationHistoryCodec.decode(encoded)!!

        assertEquals(1234L, decoded.savedAtMs)
        assertEquals(listOf("/c/one", "/c/two"), decoded.conversations.map { it.path })
        assertFalse(decoded.conversations.any { it.active })
        assertEquals("今天", decoded.conversations.first().groupLabel)
        assertEquals(setOf("2026-08-14"), decoded.conversations.first().activityDates)
        assertEquals(true, decoded.conversations.first().pinned)
        assertEquals(mapOf("g-p-demo" to 1200L), decoded.projectCachedAtMs)
    }

    @Test
    fun decodeCleansLegacyNullMetadata() {
        val decoded = ChatGptConversationHistoryCodec.decode(
            """{"schema":"elon.chatgpt_web.conversation_index.v2","saved_at_ms":1,"conversations":[{"id":"one","title":"Greeting exchange","path":"/c/one","group_label":"null","project_id":null,"project_title":"null","project_path":null,"activity_dates":["2026-08-14"]}],"projects":[]}""",
        )!!

        assertEquals("", decoded.conversations.single().groupLabel)
        assertNull(decoded.conversations.single().projectTitle)
        assertNull(decoded.conversations.single().pinned)
    }

    @Test
    fun decodeCollapsesCachedRecentAndProjectRoutesWithoutClearingData() {
        val decoded = ChatGptConversationHistoryCodec.decode(
            """{"schema":"elon.chatgpt_web.conversation_index.v2","saved_at_ms":1,"conversations":[{"id":"shared","title":"Recent","path":"/c/shared","group_label":"昨天","activity_dates":["2026-08-13"]},{"id":"shared","title":"Project","path":"/g/g-p-demo/c/shared","group_label":"今天","project_id":"g-p-demo","project_title":"Mobile","project_path":"/g/g-p-demo/project","activity_dates":["2026-08-14"]}],"projects":[]}""",
        )!!

        assertEquals(1, decoded.conversations.size)
        assertEquals("/g/g-p-demo/c/shared", decoded.conversations.single().path)
        assertEquals(setOf("2026-08-13", "2026-08-14"), decoded.conversations.single().activityDates)
    }

    @Test
    fun decodeCanonicalizesDuplicateProductionProjectRoutes() {
        val id = "g-p-6916e3fec8d8819195edffedd2f6e08e"
        val decoded = ChatGptConversationHistoryCodec.decode(
            """{"schema":"elon.chatgpt_web.conversation_index.v2","saved_at_ms":1,"conversations":[],"projects":[{"id":"$id","title":"启动语音功能","path":"/g/$id/project"},{"id":"$id-tou-zi-jia-mi-huo-bi","title":"投资加密货币","path":"/g/$id-tou-zi-jia-mi-huo-bi/project"}]}""",
        )!!

        assertEquals(1, decoded.projects.size)
        assertEquals(id, decoded.projects.single().id)
        assertEquals("投资加密货币", decoded.projects.single().title)
        assertEquals("/g/$id/project", decoded.projects.single().path)
    }

    @Test
    fun keepsLargeAccountsAvailableForImmediateSidebarRestore() {
        val conversations = (0 until 180).map { index ->
            ChatGptWebConversation(
                id = "conversation-$index",
                title = "历史会话 $index",
                path = "/c/conversation-$index",
                active = false,
            )
        }

        val decoded = ChatGptConversationHistoryCodec.decode(
            ChatGptConversationHistoryCodec.encode(
                ChatGptConversationHistoryCache(conversations, savedAtMs = 1L),
            ),
        )!!

        assertEquals(180, decoded.conversations.size)
        assertEquals("/c/conversation-179", decoded.conversations.last().path)
    }

    @Test
    fun rejectsUnknownSchemaAndUnsafeOrEmptyIndexes() {
        assertNull(ChatGptConversationHistoryCodec.decode("{}"))
        assertNull(ChatGptConversationHistoryCodec.decode(
            """{"schema":"elon.chatgpt_web.conversation_index.v1","saved_at_ms":1,"conversations":[{"id":"bad","title":"越界","path":"https://example.com/c/bad"}]}""",
        ))
    }
}
