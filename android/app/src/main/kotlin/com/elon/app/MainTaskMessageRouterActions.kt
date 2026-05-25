package com.elon.app

import org.json.JSONObject

internal class MainTaskMessageRouterActions(
    private val keyForTrace: (String) -> String?,
    private val conversationTaskKey: (String, String) -> String,
    private val activeConversationTaskKey: () -> String?,
    private val taskIsDevelopment: (String) -> Boolean?,
    private val appendActiveMessage: (String) -> Unit,
    private val appendBackgroundTaskMessage: (String, String?, Boolean) -> Unit,
    private val removeConversationTask: (String?, String?, String?) -> ConversationTaskState?,
    private val persistActiveWork: () -> Unit,
    private val updateConversationTaskFromService: (String?, String?, String?, Boolean?, Boolean?) -> ConversationTaskState?
) {
    fun appendTaskMessage(
        raw: String,
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?
    ) {
        val parsed = runCatching { JSONObject(raw) }.getOrNull()
        val type = parsed?.optString("type")?.takeIf { it.isNotBlank() }
        val key = taskKey(traceId, projectId, conversationId)
        if (key == activeConversationTaskKey()) {
            appendActiveMessage(raw)
        } else {
            val effectiveIsDevelopment = isDevelopment
                ?: key?.let { taskIsDevelopment(it) }
                ?: false
            appendBackgroundTaskMessage(raw, key, effectiveIsDevelopment)
        }
        if (type == "done" || type == "error") {
            removeConversationTask(traceId, projectId, conversationId)
            persistActiveWork()
        } else {
            updateConversationTaskFromService(traceId, projectId, conversationId, isDevelopment, false)
        }
    }

    private fun taskKey(traceId: String?, projectId: String?, conversationId: String?): String? {
        return when {
            !traceId.isNullOrBlank() && keyForTrace(traceId) != null -> keyForTrace(traceId)
            !projectId.isNullOrBlank() && !conversationId.isNullOrBlank() ->
                conversationTaskKey(projectId, conversationId)
            else -> activeConversationTaskKey()
        }
    }
}
