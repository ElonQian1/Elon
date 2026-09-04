package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptConversationDirectoryTest {
    @Test
    fun restoresProjectRowsBeforeAnOfficialRefreshStarts() {
        val projectId = "g-p-invest"
        val directory = ChatGptConversationDirectory(
            restored = ChatGptConversationHistoryCache(
                conversations = listOf(conversation("cached", projectId)),
                savedAtMs = 500L,
                projects = listOf(project(projectId, "投资 加密货币")),
                projectCachedAtMs = mapOf(projectId to 700L),
            ),
            nowMs = { 900L },
        )

        val restored = directory.index()

        assertEquals(listOf("cached"), restored.conversations.map { it.id })
        assertEquals(
            ChatGptWebConversationCollection.SOURCE_CACHE,
            restored.projectCollections.getValue(projectId).source,
        )
        assertEquals(700L, restored.projectCollections.getValue(projectId).cachedAtMs)

        assertTrue(directory.requestProject("/g/$projectId/project"))
        val request = directory.beginRefresh(projectId)
        val refreshing = directory.index()

        assertEquals(projectId, request.scopeProjectId)
        assertEquals(listOf("cached"), refreshing.conversations.map { it.id })
        assertEquals(
            ChatGptWebConversationCollection.LOAD_LOADING,
            refreshing.projectCollections.getValue(projectId).officialLoadState,
        )
    }

    @Test
    fun completeProjectRefreshReplacesOnlyThatProjectsRows() {
        val targetId = "g-p-invest"
        val otherId = "g-p-health"
        val directory = ChatGptConversationDirectory(
            restored = ChatGptConversationHistoryCache(
                conversations = listOf(
                    conversation("global", null),
                    conversation("old-target", targetId),
                    conversation("other", otherId),
                ),
                savedAtMs = 500L,
                projects = listOf(
                    project(targetId, "投资 加密货币"),
                    project(otherId, "家庭成员健康"),
                ),
            ),
            nowMs = { 900L },
        )

        directory.accept(ChatGptWebEvent.ConversationList(
            conversations = listOf(conversation("fresh-target", targetId)),
            projects = listOf(project(targetId, "聊天")),
            collection = ChatGptWebConversationCollection(
                reachedEnd = true,
                observedCount = 1,
            ),
            scopeProjectId = targetId,
        ))

        val state = directory.index()
        assertEquals(
            setOf("global", "fresh-target", "other"),
            state.conversations.mapTo(linkedSetOf()) { it.id },
        )
        assertEquals("投资 加密货币", state.projects.first { it.id == targetId }.title)
        assertEquals(
            ChatGptWebConversationCollection.SOURCE_CACHE,
            state.collection.source,
        )
        assertEquals(
            ChatGptWebConversationCollection.LOAD_READY,
            state.projectCollections.getValue(targetId).officialLoadState,
        )
    }

    @Test
    fun explicitArchivedConversationRemovalIsAppliedToRestoredCache() {
        val directory = ChatGptConversationDirectory(
            restored = ChatGptConversationHistoryCache(
                conversations = listOf(
                    conversation("archived", null),
                    conversation("retained", null),
                ),
                savedAtMs = 500L,
            ),
            nowMs = { 900L },
        )

        directory.accept(ChatGptWebEvent.ConversationList(
            conversations = listOf(conversation("archived", null)),
            collection = ChatGptWebConversationCollection(
                reachedEnd = false,
                observedCount = 1,
            ),
            removedConversationIds = setOf("archived"),
        ))

        assertEquals(listOf("retained"), directory.index().conversations.map { it.id })
    }

    private fun project(id: String, title: String) =
        ChatGptWebProject(id, title, "/g/$id/project")

    private fun conversation(id: String, projectId: String?) = ChatGptWebConversation(
        id = id,
        title = "会话 $id",
        path = projectId?.let { "/g/$it/c/$id" } ?: "/c/$id",
        active = false,
        projectId = projectId,
        projectTitle = projectId?.let { "投资 加密货币" },
        projectPath = projectId?.let { "/g/$it/project" },
    )
}
