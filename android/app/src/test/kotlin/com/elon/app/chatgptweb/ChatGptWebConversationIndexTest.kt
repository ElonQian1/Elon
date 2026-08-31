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
    fun dateViewKeepsEveryUnassignedConversationInItsOwnSection() {
        val selected = LocalDate.of(2026, 8, 14)
        val activeUnassigned = conversation("active", "今天", null).copy(
            activityDates = setOf(selected.toString()),
        )
        val oldUnassigned = conversation("old", "昨天", null)
        val oldProject = conversation("project", "昨天", "g-p-demo")
        assertEquals(
            listOf("active", "old"),
            ChatGptWebConversationIndex.unassigned(
                listOf(activeUnassigned, oldUnassigned, oldProject),
            ).map { it.id },
        )
    }

    @Test
    fun unassignedSectionExcludesEveryProjectConversation() {
        assertEquals(
            listOf("plain"),
            ChatGptWebConversationIndex.unassigned(
                listOf(
                    conversation("plain", "今天", null),
                    conversation("project", "今天", "g-p-demo"),
                ),
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
    fun completeGlobalRefreshDropsStaleUnassignedButPreservesProjectHistory() {
        val current = conversation("current", "今天", null)
        val staleUnassigned = conversation("stale", "昨天", null)
        val cachedProject = conversation("project", "昨天", "g-p-demo").copy(
            path = "/g/g-p-demo/c/project",
            projectTitle = "移动端项目",
            projectPath = "/g/g-p-demo/project",
        )

        val merged = ChatGptWebConversationIndex.mergeOfficialHistory(
            previous = listOf(current, staleUnassigned, cachedProject),
            observed = listOf(current),
            collectionComplete = true,
        )

        assertEquals(listOf("current", "project"), merged.map { it.id })
        assertEquals("g-p-demo", merged.last().projectId)
    }

    @Test
    fun partialGlobalRefreshPreservesEveryCachedConversationDomain() {
        val current = conversation("current", "今天", null)
        val staleUnassigned = conversation("stale", "昨天", null)
        val cachedProject = conversation("project", "昨天", "g-p-demo").copy(
            path = "/g/g-p-demo/c/project",
        )

        assertEquals(
            listOf("current", "stale", "project"),
            ChatGptWebConversationIndex.mergeOfficialHistory(
                previous = listOf(current, staleUnassigned, cachedProject),
                observed = listOf(current),
                collectionComplete = false,
            ).map { it.id },
        )
    }

    @Test
    fun projectRefreshReassignsAnObservedConversationWithoutLeavingTheOldMembership() {
        val original = conversation("shared", "昨天", "g-p-old").copy(
            path = "/g/g-p-old/c/shared",
            projectTitle = "旧项目",
            projectPath = "/g/g-p-old/project",
        )
        val moved = original.copy(
            path = "/g/g-p-target/c/shared",
            projectId = "g-p-target",
            projectTitle = "目标项目",
            projectPath = "/g/g-p-target/project",
        )

        val merged = ChatGptWebConversationIndex.mergeProjectHistory(
            previous = listOf(original),
            observed = listOf(moved),
            projectId = "g-p-target",
            collectionComplete = false,
        )

        assertEquals(1, merged.size)
        assertEquals("g-p-target", merged.single().projectId)
        assertEquals("/g/g-p-target/c/shared", merged.single().path)
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
    fun pinnedLabelsAreNotTreatedAsTemporalConversationGroups() {
        val cached = conversation("one", "已置顶", null)
        val observed = conversation("one", "", null)

        val merged = ChatGptWebConversationIndex.merge(listOf(cached), listOf(observed)).single()

        assertEquals("", merged.groupLabel)
        assertEquals("历史会话", ChatGptWebConversationIndex.sections(listOf(cached)).single().label)
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

    @Test
    fun canonicalizesProductionProjectRoutesWithReadableSuffixes() {
        val id = "g-p-6916e3fec8d8819195edffedd2f6e08e"
        val suffixed = "$id-tou-zi-jia-mi-huo-bi"

        assertEquals(id, ChatGptWebConversationPath.canonicalProjectId(suffixed))
        assertEquals(id, ChatGptWebConversationPath.projectId("/g/$suffixed/c/inside"))
        assertEquals("/g/$id/project", ChatGptWebConversationPath.normalizeProject("/g/$suffixed/project"))
    }

    @Test
    fun mergesLegacyAndReadableProjectRoutesIntoOneCanonicalProject() {
        val id = "g-p-6916e3fec8d8819195edffedd2f6e08e"
        val observed = ChatGptWebProject(
            "$id-tou-zi-jia-mi-huo-bi",
            "投资加密货币",
            "/g/$id-tou-zi-jia-mi-huo-bi/project",
        )
        val cached = ChatGptWebProject(id, "启动语音功能", "/g/$id/project")

        val projects = ChatGptWebConversationIndex.mergeObservedProjects(
            conversations = emptyList(),
            previous = listOf(cached),
            observed = listOf(observed),
        )

        assertEquals(1, projects.size)
        assertEquals(id, projects.single().id)
        assertEquals("投资加密货币", projects.single().title)
        assertEquals("/g/$id/project", projects.single().path)
    }

    @Test
    fun canonicalizesCachedConversationProjectMembership() {
        val id = "g-p-6916e3fec8d8819195edffedd2f6e08e"
        val suffixed = "$id-tou-zi-jia-mi-huo-bi"
        val sanitized = ChatGptWebConversationIndex.sanitize(
            conversation("inside", "今天", suffixed).copy(
                path = "/g/$suffixed/c/inside",
                projectTitle = "投资加密货币",
                projectPath = "/g/$suffixed/project",
            ),
        )

        assertEquals(id, sanitized.projectId)
        assertEquals("/g/$id/project", sanitized.projectPath)
    }

    @Test
    fun currentSnapshotImmediatelyAddsAConversationMissingFromCachedSidebar() {
        val observed = ChatGptWebConversationIndex.observeCurrent(
            previous = listOf(conversation("old", "昨天", null)),
            snapshot = snapshot("fresh", "语音创建的会话"),
            activityDate = LocalDate.of(2026, 8, 21),
        )

        assertEquals(listOf("fresh", "old"), observed.map { it.id })
        assertEquals("语音创建的会话", observed.first().title)
        assertEquals(setOf("2026-08-21"), observed.first().activityDates)
        assertEquals(true, observed.first().active)
        assertEquals(false, observed.last().active)
    }

    @Test
    fun currentProjectSnapshotUsesKnownProjectMetadata() {
        val project = ChatGptWebProject("g-p-demo", "家庭成员健康", "/g/g-p-demo/project")
        val observed = ChatGptWebConversationIndex.observeCurrent(
            previous = emptyList(),
            snapshot = snapshot("inside", "项目语音会话", "/g/g-p-demo/c/inside"),
            activityDate = LocalDate.of(2026, 8, 21),
            knownProjects = listOf(project),
        ).single()

        assertEquals("g-p-demo", observed.projectId)
        assertEquals("家庭成员健康", observed.projectTitle)
        assertEquals("/g/g-p-demo/project", observed.projectPath)
    }

    private fun conversation(id: String, group: String, projectId: String?) = ChatGptWebConversation(
        id = id,
        title = "会话 $id",
        path = "/c/$id",
        active = false,
        groupLabel = group,
        projectId = projectId,
    )

    private fun snapshot(id: String, title: String, path: String = "/c/$id") = ChatGptWebSnapshot(
        title = title,
        url = "https://chatgpt.com$path",
        draft = "",
        messages = emptyList(),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(emptySet()),
        pageKind = "conversation",
    )
}
