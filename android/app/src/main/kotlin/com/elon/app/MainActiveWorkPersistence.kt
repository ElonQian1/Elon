package com.elon.app

import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

internal data class RestoredActiveWork(
    val tasks: List<ConversationTaskState>,
    val shouldRefreshUi: Boolean
)

internal fun persistActiveWorkTasks(
    prefs: SharedPreferences,
    tasks: Collection<ConversationTaskState>
) {
    val array = JSONArray()
    tasks.forEach { task ->
        array.put(
            JSONObject()
                .put("payload", task.payload)
                .put("is_development", task.isDevelopment)
                .put("started_at", task.startedAt)
        )
    }
    if (array.length() == 0) {
        clearPersistedActiveWorkTasks(prefs)
        return
    }
    prefs.edit()
        .putString(TaskWorkService.PREF_PENDING_WORK_TASKS, array.toString())
        .remove(TaskWorkService.PREF_PENDING_WORK_PAYLOAD)
        .remove(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT)
        .remove(TaskWorkService.PREF_PENDING_WORK_TIME)
        .apply()
}

internal fun clearPersistedActiveWorkTasks(prefs: SharedPreferences) {
    prefs.edit()
        .remove(TaskWorkService.PREF_PENDING_WORK_TASKS)
        .remove(TaskWorkService.PREF_PENDING_WORK_PAYLOAD)
        .remove(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT)
        .remove(TaskWorkService.PREF_PENDING_WORK_TIME)
        .apply()
}

internal fun restorePersistedActiveWorkTasks(
    prefs: SharedPreferences,
    now: Long,
    fallbackProjectId: String,
    fallbackConversationId: String
): RestoredActiveWork {
    val tasksJson = prefs.getString(TaskWorkService.PREF_PENDING_WORK_TASKS, null)?.takeIf { it.isNotBlank() }
    if (tasksJson != null) {
        val tasks = restoreTaskArray(tasksJson, now, fallbackProjectId)
        return RestoredActiveWork(tasks, shouldRefreshUi = true)
    }

    val legacy = restoreLegacyTask(prefs, now, fallbackProjectId, fallbackConversationId)
        ?: return RestoredActiveWork(emptyList(), shouldRefreshUi = false)
    return RestoredActiveWork(listOf(legacy), shouldRefreshUi = true)
}

private fun restoreTaskArray(
    tasksJson: String,
    now: Long,
    fallbackProjectId: String
): List<ConversationTaskState> {
    val array = runCatching { JSONArray(tasksJson) }.getOrNull() ?: return emptyList()
    val tasks = mutableListOf<ConversationTaskState>()
    for (index in 0 until array.length()) {
        val item = array.optJSONObject(index) ?: continue
        val payload = item.optString("payload").takeIf { it.isNotBlank() } ?: continue
        val savedAt = item.optLong("started_at", now)
        if (savedAt <= 0L || now - savedAt > TaskWorkService.PENDING_WORK_TTL_MS) continue
        val parsed = runCatching { JSONObject(payload) }.getOrNull() ?: continue
        val traceId = parsed.optString("trace_id").takeIf { it.isNotBlank() } ?: continue
        val projectId = parsed.optString("project_id").takeIf { it.isNotBlank() } ?: fallbackProjectId
        val conversationId = parsed.optString("conversation_id").takeIf { it.isNotBlank() } ?: "default"
        tasks.add(
            ConversationTaskState(
                traceId = traceId,
                projectId = projectId,
                conversationId = conversationId,
                payload = payload,
                isDevelopment = item.optBoolean("is_development", true),
                pendingReconnect = true,
                startedAt = savedAt
            )
        )
    }
    return tasks
}

private fun restoreLegacyTask(
    prefs: SharedPreferences,
    now: Long,
    fallbackProjectId: String,
    fallbackConversationId: String
): ConversationTaskState? {
    val payload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)?.takeIf { it.isNotBlank() }
        ?: return null
    val savedAt = prefs.getLong(TaskWorkService.PREF_PENDING_WORK_TIME, 0L)
    val tooOld = savedAt <= 0L || now - savedAt > TaskWorkService.PENDING_WORK_TTL_MS
    if (tooOld) {
        clearPersistedActiveWorkTasks(prefs)
        return null
    }
    val parsed = runCatching { JSONObject(payload) }.getOrNull() ?: return null
    val traceId = parsed.optString("trace_id").takeIf { it.isNotBlank() } ?: return null
    val projectId = parsed.optString("project_id").takeIf { it.isNotBlank() } ?: fallbackProjectId
    val conversationId = parsed.optString("conversation_id").takeIf { it.isNotBlank() } ?: fallbackConversationId
    return ConversationTaskState(
        traceId = traceId,
        projectId = projectId,
        conversationId = conversationId,
        payload = payload,
        isDevelopment = prefs.getBoolean(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT, true),
        pendingReconnect = true,
        startedAt = savedAt
    )
}
