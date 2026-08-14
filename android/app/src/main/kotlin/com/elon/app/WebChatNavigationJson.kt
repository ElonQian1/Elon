package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebConversationIndex
import java.time.LocalDate
import org.json.JSONArray
import org.json.JSONObject

internal object WebChatNavigationJson {
    const val MAX_PAGE_SIZE = 50

    fun page(
        providerId: WebChatProviderId,
        state: ChatGptWebConversationIndexState,
        query: String,
        date: LocalDate?,
        offset: Int,
        limit: Int,
    ): JSONObject {
        val matchingConversations = state.conversations.filter { conversation ->
            query.isBlank() ||
                conversation.title.contains(query, ignoreCase = true) ||
                conversation.projectTitle.orEmpty().contains(query, ignoreCase = true)
        }
        val daily = date?.let { ChatGptWebConversationIndex.activeOn(matchingConversations, it) }
            ?: emptyList()
        val conversations = if (date == null) {
            matchingConversations
        } else {
            daily + ChatGptWebConversationIndex.unassignedExcluding(matchingConversations, daily)
        }
        val dailyIdentities = daily.mapTo(mutableSetOf(), ChatGptWebConversationIndex::identityOf)
        val projects = state.projects.filter { project ->
            query.isBlank() || project.title.contains(query, ignoreCase = true)
        }
        return JSONObject()
            .put("schema", "elon.web_chat.navigation.v1")
            .put("provider_id", providerId.wireValue)
            .put("offset", offset)
            .put("limit", limit)
            .put("query_applied", query.isNotBlank())
            .put("date", date?.toString() ?: JSONObject.NULL)
            .put("conversation_total", conversations.size)
            .put("project_total", projects.size)
            .put("conversation_has_more", offset + limit < conversations.size)
            .put("project_has_more", offset + limit < projects.size)
            .put("collection_state", state.collection.officialLoadState)
            .put("conversations", JSONArray().apply {
                conversations.drop(offset).take(limit).forEach { conversation ->
                    put(JSONObject()
                        .put("id", conversation.id)
                        .put("title", conversation.title)
                        .put("path", conversation.path)
                        .put("active", conversation.active)
                        .put(
                            "sidebar_group",
                            when {
                                ChatGptWebConversationIndex.identityOf(conversation) in dailyIdentities -> "daily_active"
                                conversation.projectId == null -> "unassigned"
                                else -> "project"
                            },
                        )
                        .put("project_id", conversation.projectId ?: JSONObject.NULL)
                        .put("project_title", conversation.projectTitle ?: JSONObject.NULL)
                        .put("activity_dates", JSONArray(conversation.activityDates.sorted())))
                }
            })
            .put("projects", JSONArray().apply {
                projects.drop(offset).take(limit).forEach { project ->
                    put(JSONObject()
                        .put("id", project.id)
                        .put("title", project.title)
                        .put("path", project.path)
                        .put("active", project.active))
                }
            })
    }
}
