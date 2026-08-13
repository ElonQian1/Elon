package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.LocalDate

class ChatGptWebConversationIndexTest {
    @Test
    fun groupsOfficialDateLabelsWithoutInventingDates() {
        val conversations = listOf(
            conversation("one", "今天", null),
            conversation("two", "昨天", null),
            conversation("three", "", null),
        )

        assertEquals(
            listOf("今天", "昨天", "历史会话"),
            ChatGptWebConversationIndex.sections(conversations).map { it.label },
        )
    }

    @Test
    fun extractsProjectRowsAndKeepsProjectConversationMembership() {
        val projectPath = "/g/g-p-demo/project"
        val conversation = ChatGptWebConversation(
            id = "inside",
            title = "项目内会话",
            path = "/g/g-p-demo/c/inside",
            active = false,
            groupLabel = "今天",
            projectId = "g-p-demo",
            projectTitle = "安卓客户端",
            projectPath = projectPath,
        )

        assertEquals(projectPath, ChatGptWebConversationIndex.projects(listOf(conversation)).single().path)
        assertEquals("g-p-demo", ChatGptWebConversationPath.projectId(conversation.path))
        assertEquals(conversation.path, ChatGptWebConversationPath.normalize(conversation.path))
        assertNull(ChatGptWebConversationPath.normalize("/g/g-p-demo/c/../auth"))
    }

    @Test
    fun dailyActivityUsesOnlyExplicitCanonicalDates() {
        val selected = LocalDate.of(2026, 8, 14)
        val values = listOf(
            conversation("today", "今天", null).copy(activityDates = setOf(selected.toString())),
            conversation("history", "前 7 天", null),
        )

        assertEquals(listOf("today"), ChatGptWebConversationIndex.activeOn(values, selected).map { it.id })
    }

    @Test
    fun refreshPreservesPreviouslyObservedActivityDates() {
        val previous = conversation("one", "今天", null).copy(activityDates = setOf("2026-08-13"))
        val observed = conversation("one", "今天", null).copy(activityDates = setOf("2026-08-14"))

        assertEquals(
            setOf("2026-08-13", "2026-08-14"),
            ChatGptWebConversationIndex.merge(listOf(previous), listOf(observed)).single().activityDates,
        )
    }

    @Test
    fun partialOfficialRefreshDoesNotEraseCachedDailyConversations() {
        val previous = listOf(
            conversation("one", "今天", null).copy(activityDates = setOf("2026-08-14")),
            conversation("two", "昨天", null).copy(activityDates = setOf("2026-08-13")),
        )

        assertEquals(
            listOf("one", "two"),
            ChatGptWebConversationIndex.merge(previous, listOf(previous.first())).map { it.id },
        )
    }

    private fun conversation(id: String, group: String, projectId: String?) = ChatGptWebConversation(
        id = id,
        title = "会话 $id",
        path = "/c/$id",
        active = false,
        groupLabel = group,
        projectId = projectId,
    )
}
