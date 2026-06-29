package com.elon.app

import org.json.JSONObject

internal class MainTaskMessageRouterActions(
    private val keyForTrace: (String) -> String?,
    private val conversationTaskKey: (String, String) -> String,
    private val activeConversationTaskKey: () -> String?,
    private val taskIsDevelopment: (String) -> Boolean?,
    private val isProjectConversationVisible: () -> Boolean,
    private val appendActiveMessage: (String) -> Unit,
    private val appendBackgroundTaskMessage: (String, String?, Boolean) -> Unit,
    private val removeConversationTask: (String?, String?, String?) -> ConversationTaskState?,
    private val persistActiveWork: () -> Unit,
    private val updateConversationTaskFromService: (String?, String?, String?, Boolean?, Boolean?) -> ConversationTaskState?,
    private val drainNextQueuedMessage: (String?, String?) -> Unit,
    private val markProjectTaskCompleted: (String?) -> Unit = {}
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
        val activeVisible = key == activeConversationTaskKey() && isProjectConversationVisible()
        if (activeVisible) {
            appendActiveMessage(raw)
        } else {
            val effectiveIsDevelopment = isDevelopment
                ?: key?.let { taskIsDevelopment(it) }
                ?: false
            appendBackgroundTaskMessage(raw, key, effectiveIsDevelopment)
        }
        if (type == "done" || type == "error") {
            if (type == "done" && !activeVisible) {
                markProjectTaskCompleted(projectId ?: projectIdFromTaskKey(key))
            }
            removeConversationTask(traceId, projectId, conversationId)
            persistActiveWork()
            drainNextQueuedMessage(projectId, conversationId)
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

    private fun projectIdFromTaskKey(key: String?): String? {
        return key
            ?.substringBefore('\u001F', "")
            ?.takeIf { it.isNotBlank() }
    }
}
