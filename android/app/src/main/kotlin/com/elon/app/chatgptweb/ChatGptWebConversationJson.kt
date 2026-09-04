package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebConversationJson {
    fun encode(conversation: ChatGptWebConversation): JSONObject = JSONObject()
        .put("id", conversation.id)
        .put("title", conversation.title)
        .put("path", conversation.path)
        .put("active", conversation.active)
        .put("group_label", conversation.groupLabel)
        .put("project_id", conversation.projectId ?: JSONObject.NULL)
        .put("project_title", conversation.projectTitle ?: JSONObject.NULL)
        .put("project_path", conversation.projectPath ?: JSONObject.NULL)
        .put("pinned", conversation.pinned ?: JSONObject.NULL)
        .put("activity_dates", JSONArray(conversation.activityDates.sorted()))
        .put("native_action", "chatgpt_open_conversation")
        .put(
            "native_adb_content_description",
            ChatGptNativeNavigationSelector.conversation(conversation),
        )
}
