package com.elon.app

import android.content.SharedPreferences
import org.json.JSONArray

internal class MainConversationTaskRegistryActions(
    private val prefs: SharedPreferences,
    private val runningConversationTasks: MutableMap<String, ConversationTaskState>,
    private val runningTraceToConversation: MutableMap<String, String>,
    private val taskResponseTokens: MutableMap<String, Int>,
    private val activeProject: () -> AppProject,
    private val activeConversation: () -> AppConversation,
    private val setWaitingForReply: (Boolean) -> Unit,
    private val setActiveRequestIsDevelopment: (Boolean) -> Unit,
    private val setPendingRequestPayload: (String?) -> Unit,
    private val setPendingReconnectForActiveWork: (Boolean) -> Unit,
    private val resetReconnectAttempts: () -> Unit,
    private val getActiveRequestIsDevelopment: () -> Boolean,
    private val setSendEnabled: (Boolean) -> Unit,
    private val renderConversationList: () -> Unit,
    private val updateStage: (String, String) -> Unit,
    private val updateProjectViews: (String) -> Unit
) {
    fun conversationTaskKey(projectId: String, conversationId: String): String {
        return "$projectId\u001F$conversationId"
    }

    fun activeConversationTaskKey(): String {
        return conversationTaskKey(activeProject().id, activeConversation().id)
    }

    fun isActiveConversationWorking(): Boolean {
        return runningConversationTasks.containsKey(activeConversationTaskKey())
    }

    fun activeConversationTask(): ConversationTaskState? {
        return runningConversationTasks[activeConversationTaskKey()]
    }

    fun rememberConversationTask(
        target: SendTarget,
        traceId: String,
        payload: String,
        isDevelopment: Boolean
    ) {
        val key = conversationTaskKey(target.projectId, target.conversationId)
        runningConversationTasks[key] = ConversationTaskState(
            traceId = traceId,
            projectId = target.projectId,
            conversationId = target.conversationId,
            payload = payload,
            isDevelopment = isDevelopment
        )
        runningTraceToConversation[traceId] = key
        refreshActiveTaskState()
    }

    fun updateConversationTaskFromService(
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?,
        pendingReconnect: Boolean? = null
    ): ConversationTaskState? {
        val key = when {
            !traceId.isNullOrBlank() && runningTraceToConversation.containsKey(traceId) ->
                runningTraceToConversation[traceId]
            !projectId.isNullOrBlank() && !conversationId.isNullOrBlank() ->
                conversationTaskKey(projectId, conversationId)
            else -> null
        } ?: return null
        val existing = runningConversationTasks[key] ?: createConversationTaskFromService(
            key = key,
            traceId = traceId,
            projectId = projectId,
            conversationId = conversationId,
            isDevelopment = isDevelopment,
            pendingReconnect = pendingReconnect
        ) ?: return null
        if (!traceId.isNullOrBlank()) runningTraceToConversation[traceId] = key
        isDevelopment?.let { existing.isDevelopment = it }
        pendingReconnect?.let { existing.pendingReconnect = it }
        refreshActiveTaskState()
        return existing
    }

    fun removeConversationTask(
        traceId: String?,
        projectId: String?,
        conversationId: String?
    ): ConversationTaskState? {
        val key = when {
            !traceId.isNullOrBlank() -> runningTraceToConversation.remove(traceId)
            !projectId.isNullOrBlank() && !conversationId.isNullOrBlank() ->
                conversationTaskKey(projectId, conversationId)
            else -> null
        } ?: return null
        val removed = runningConversationTasks.remove(key)
        removed?.let {
            runningTraceToConversation.entries.removeAll { entry -> entry.value == key }
            taskResponseTokens.remove(it.traceId)
        }
        refreshActiveTaskState()
        return removed
    }

    fun refreshActiveTaskState() {
        setWaitingForReply(runningConversationTasks.isNotEmpty())
        val activeTask = activeConversationTask()
        setActiveRequestIsDevelopment(
            activeTask?.isDevelopment
                ?: runningConversationTasks.values.lastOrNull()?.isDevelopment
                ?: false
        )
        setPendingRequestPayload(activeTask?.payload)
        setPendingReconnectForActiveWork(activeTask?.pendingReconnect ?: false)
        setSendEnabled(!isActiveConversationWorking())
        renderConversationList()
    }

    fun persistActiveWork() {
        persistActiveWorkTasks(prefs, runningConversationTasks.values)
    }

    fun clearPersistedActiveWork() {
        clearPersistedActiveWorkTasks(prefs)
    }

    fun restorePendingActiveWork() {
        val restored = restorePersistedActiveWorkTasks(
            prefs = prefs,
            now = System.currentTimeMillis(),
            fallbackProjectId = activeProject().id,
            fallbackConversationId = activeConversation().id
        )
        if (!restored.shouldRefreshUi) return

        restored.tasks.forEach { task ->
            val key = conversationTaskKey(task.projectId, task.conversationId)
            runningConversationTasks[key] = task
            runningTraceToConversation[task.traceId] = key
        }

        refreshActiveTaskState()
        resetReconnectAttempts()
        if (getActiveRequestIsDevelopment()) {
            updateStage("后台继续", "任务仍在服务器继续处理，连接恢复后会同步最新进度。")
        } else {
            updateProjectViews("上一条回复仍在处理，连接恢复后会同步结果。")
        }
    }

    fun syncActiveTasksFromServiceState(activeTasksJson: String?) {
        if (activeTasksJson.isNullOrBlank()) return
        val array = runCatching { JSONArray(activeTasksJson) }.getOrNull() ?: return
        for (index in 0 until array.length()) {
            val item = array.optJSONObject(index) ?: continue
            val traceId = item.optString("trace_id").takeIf { it.isNotBlank() } ?: continue
            val projectId = item.optString("project_id").takeIf { it.isNotBlank() } ?: continue
            val conversationId = item.optString("conversation_id").takeIf { it.isNotBlank() } ?: continue
            val key = conversationTaskKey(projectId, conversationId)
            val existing = runningConversationTasks[key] ?: ConversationTaskState(
                traceId = traceId,
                projectId = projectId,
                conversationId = conversationId,
                payload = "",
                isDevelopment = item.optBoolean("is_development", true),
                pendingReconnect = false,
                startedAt = item.optLong("started_at", System.currentTimeMillis())
            ).also {
                runningConversationTasks[key] = it
            }
            runningTraceToConversation[traceId] = key
            existing.pendingReconnect = false
            existing.isDevelopment = item.optBoolean("is_development", existing.isDevelopment)
        }
        refreshActiveTaskState()
    }

    private fun createConversationTaskFromService(
        key: String,
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?,
        pendingReconnect: Boolean?
    ): ConversationTaskState? {
        if (traceId.isNullOrBlank() || projectId.isNullOrBlank() || conversationId.isNullOrBlank()) {
            return null
        }
        return ConversationTaskState(
            traceId = traceId,
            projectId = projectId,
            conversationId = conversationId,
            payload = "",
            isDevelopment = isDevelopment ?: true,
            pendingReconnect = pendingReconnect ?: false
        ).also { runningConversationTasks[key] = it }
    }
}
