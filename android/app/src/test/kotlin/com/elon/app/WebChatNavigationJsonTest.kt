package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationCollection
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject
import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatNavigationJsonTest {
    @Test
    fun datePageListsDailyActivityThenRemainingUnassignedWithoutProjectDuplicates() {
        val selected = LocalDate.of(2026, 8, 14)
        val state = ChatGptWebConversationIndexState(
            conversations = listOf(
                conversation("active-project", "Alpha", selected.toString(), "p1", "Project Alpha"),
                conversation("active-unassigned", "Beta", selected.toString(), null, null),
                conversation("old-unassigned", "Gamma", "2026-08-13", null, null),
                conversation("old-project", "Delta", "2026-08-13", "p2", "Project Beta"),
            ),
            projects = listOf(
                ChatGptWebProject("p1", "Project Alpha", "/g/g-p-one/project"),
                ChatGptWebProject("p2", "Project Beta", "/g/g-p-two/project"),
            ),
            collection = ChatGptWebConversationCollection(officialLoadState = "ready"),
        )

        val page = WebChatNavigationJson.page(
            providerId = WebChatProviderId.CHATGPT_WEB,
            state = state,
            query = "",
            date = selected,
            offset = 0,
            limit = 20,
        )

        assertEquals("elon.web_chat.navigation.v1", page.getString("schema"))
        assertEquals(3, page.getInt("conversation_total"))
        assertEquals(
            listOf("active-project", "active-unassigned", "old-unassigned"),
            page.getJSONArray("conversations").let { values ->
                (0 until values.length()).map { values.getJSONObject(it).getString("id") }
            },
        )
        assertEquals(
            listOf("daily_active", "daily_active", "unassigned"),
            page.getJSONArray("conversations").let { values ->
                (0 until values.length()).map { values.getJSONObject(it).getString("sidebar_group") }
            },
        )
        assertEquals(2, page.getInt("project_total"))
        assertFalse(page.getBoolean("project_has_more"))
        assertFalse(page.getBoolean("conversation_has_more"))
    }

    @Test
    fun searchesConversationProjectMembershipAndProjectTitles() {
        val state = ChatGptWebConversationIndexState(
            conversations = listOf(conversation("one", "General chat", "2026-08-14", "p1", "Finance")),
            projects = listOf(ChatGptWebProject("p1", "Finance", "/g/g-p-one/project")),
        )

        val page = WebChatNavigationJson.page(
            providerId = WebChatProviderId.CHATGPT_WEB,
            state = state,
            query = "finance",
            date = null,
            offset = 0,
            limit = 20,
        )

        assertEquals(1, page.getInt("conversation_total"))
        assertEquals(1, page.getInt("project_total"))
        assertTrue(page.getBoolean("query_applied"))
    }

    private fun conversation(
        id: String,
        title: String,
        date: String,
        projectId: String?,
        projectTitle: String?,
    ) = ChatGptWebConversation(
        id = id,
        title = title,
        path = "/c/$id",
        active = false,
        projectId = projectId,
        projectTitle = projectTitle,
        activityDates = setOf(date),
    )
}
