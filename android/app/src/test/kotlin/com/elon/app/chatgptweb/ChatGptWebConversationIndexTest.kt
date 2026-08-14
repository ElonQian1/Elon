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
    fun dateViewAddsRemainingUnassignedConversationsWithoutDuplicates() {
        val selected = LocalDate.of(2026, 8, 14)
        val activeUnassigned = conversation("active", "今天", null).copy(
            activityDates = setOf(selected.toString()),
        )
        val oldUnassigned = conversation("old", "昨天", null)
        val oldProject = conversation("project", "昨天", "g-p-demo")
        val active = ChatGptWebConversationIndex.activeOn(
            listOf(activeUnassigned, oldUnassigned, oldProject),
            selected,
        )

        assertEquals(
            listOf("old"),
            ChatGptWebConversationIndex.unassignedExcluding(
                listOf(activeUnassigned, oldUnassigned, oldProject),
                active,
            ).map { it.id },
        )
    }

    @Test
    fun dateViewDeduplicatesAliasesByCanonicalConversationPath() {
        val selected = LocalDate.of(2026, 8, 14)
        val activeAlias = conversation("active-id", "今天", null).copy(
            path = "/c/shared-conversation",
            activityDates = setOf(selected.toString()),
        )
        val cachedAlias = conversation("cached-id", "旧缓存", null).copy(
            path = "/g/g-p-demo/c/shared-conversation",
        )

        assertEquals(
            emptyList<String>(),
            ChatGptWebConversationIndex.unassignedExcluding(
                listOf(activeAlias, cachedAlias),
                listOf(activeAlias),
            ).map { it.id },
        )
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

    @Test
    fun completeOfficialRefreshDropsConversationsMissingFromTheOfficialList() {
        val previous = listOf(
            conversation("one", "今天", null).copy(activityDates = setOf("2026-08-14")),
            conversation("stale", "昨天", null).copy(activityDates = setOf("2026-08-13")),
        )
        val observed = previous.first().copy(activityDates = setOf("2026-08-15"))

        val merged = ChatGptWebConversationIndex.merge(
            previous,
            listOf(observed),
            retainMissing = false,
        )

        assertEquals(listOf("one"), merged.map { it.id })
        assertEquals(setOf("2026-08-14", "2026-08-15"), merged.single().activityDates)
    }

    @Test
    fun completeOfficialRefreshDropsProjectsMissingFromTheOfficialList() {
        val current = ChatGptWebProject("g-p-current", "当前项目", "/g/g-p-current/project")
        val stale = ChatGptWebProject("g-p-stale", "过期项目", "/g/g-p-stale/project")

        val merged = ChatGptWebConversationIndex.mergeProjects(
            conversations = emptyList(),
            previous = listOf(current, stale),
            observed = listOf(current),
            retainMissing = false,
        )

        assertEquals(listOf("g-p-current"), merged.map { it.id })
    }

    @Test
    fun partialProjectObservationNeverErasesPreviouslyIndexedProjects() {
        val current = ChatGptWebProject("g-p-current", "当前项目", "/g/g-p-current/project")
        val temporarilyHidden = ChatGptWebProject("g-p-hidden", "暂时不可见", "/g/g-p-hidden/project")

        val merged = ChatGptWebConversationIndex.mergeObservedProjects(
            conversations = emptyList(),
            previous = listOf(current, temporarilyHidden),
            observed = listOf(current),
        )

        assertEquals(listOf("g-p-current", "g-p-hidden"), merged.map { it.id })
    }

    @Test
    fun mergeCleansLegacyNullMetadataWithoutDroppingActivity() {
        val cached = conversation("one", "null", null).copy(
            projectTitle = "null",
            activityDates = setOf("2026-08-14"),
        )
        val observed = conversation("one", "", null)

        val merged = ChatGptWebConversationIndex.merge(listOf(cached), listOf(observed)).single()

        assertEquals("", merged.groupLabel)
        assertNull(merged.projectTitle)
        assertEquals(setOf("2026-08-14"), merged.activityDates)
    }

    @Test
    fun mergeCollapsesRecentAndProjectRoutesForTheSameConversation() {
        val recent = conversation("shared", "昨天", null).copy(
            activityDates = setOf("2026-08-13"),
        )
        val project = recent.copy(
            path = "/g/g-p-demo/c/shared",
            groupLabel = "今天",
            projectId = "g-p-demo",
            projectTitle = "移动端项目",
            projectPath = "/g/g-p-demo/project",
            activityDates = setOf("2026-08-14"),
        )

        val merged = ChatGptWebConversationIndex.merge(listOf(recent), listOf(recent, project))

        assertEquals(1, merged.size)
        assertEquals("/g/g-p-demo/c/shared", merged.single().path)
        assertEquals("g-p-demo", merged.single().projectId)
        assertEquals(setOf("2026-08-13", "2026-08-14"), merged.single().activityDates)
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
