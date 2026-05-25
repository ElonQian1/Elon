package com.elon.app

import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

internal fun taskActiveTasksJson(tasks: Collection<RunningTask>): JSONArray {
    val array = JSONArray()
    tasks.forEach { task ->
        array.put(
            JSONObject()
                .put("trace_id", task.traceId)
                .put("project_id", task.projectId)
                .put("conversation_id", task.conversationId)
                .put("is_development", task.isDevelopment)
                .put("started_at", task.startedAtMs)
        )
    }
    return array
}

internal fun persistTaskWork(prefs: SharedPreferences, tasks: Collection<RunningTask>) {
    val array = JSONArray()
    tasks
        .filter { it.waitingForReply && it.payload.isNotBlank() }
        .forEach { task ->
            array.put(
                JSONObject()
                    .put("payload", task.payload)
                    .put("is_development", task.isDevelopment)
                    .put("started_at", task.startedAtMs)
            )
        }
    if (array.length() == 0) {
        clearPersistedTaskWork(prefs)
        return
    }
    prefs.edit()
        .putString(TaskWorkService.PREF_PENDING_WORK_TASKS, array.toString())
        .remove(TaskWorkService.PREF_PENDING_WORK_PAYLOAD)
        .remove(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT)
        .remove(TaskWorkService.PREF_PENDING_WORK_TIME)
        .apply()
}

internal fun clearPersistedTaskWork(prefs: SharedPreferences) {
    prefs.edit()
        .remove(TaskWorkService.PREF_PENDING_WORK_TASKS)
        .remove(TaskWorkService.PREF_PENDING_WORK_PAYLOAD)
        .remove(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT)
        .remove(TaskWorkService.PREF_PENDING_WORK_TIME)
        .apply()
}

internal fun restorePersistedTaskWork(
    prefs: SharedPreferences,
    activeTasks: MutableMap<String, RunningTask>
) {
    val restored = mutableListOf<RunningTask>()
    val now = System.currentTimeMillis()
    val tasksJson = prefs.getString(TaskWorkService.PREF_PENDING_WORK_TASKS, null)?.takeIf { it.isNotBlank() }
    if (tasksJson != null) {
        val array = runCatching { JSONArray(tasksJson) }.getOrNull()
        if (array != null) {
            for (index in 0 until array.length()) {
                val item = array.optJSONObject(index) ?: continue
                val payload = item.optString("payload").takeIf { it.isNotBlank() } ?: continue
                val savedAt = item.optLong("started_at", now)
                if (savedAt <= 0L || now - savedAt > TaskWorkService.PENDING_WORK_TTL_MS) continue
                val traceId = taskPayloadTraceId(payload) ?: continue
                if (activeTasks.containsKey(traceId)) continue
                restored += RunningTask(
                    traceId = traceId,
                    projectId = taskPayloadString(payload, "project_id"),
                    conversationId = taskPayloadString(payload, "conversation_id"),
                    payload = payload,
                    isDevelopment = item.optBoolean("is_development", true),
                    startedAtMs = savedAt
                )
            }
        }
    }

    if (tasksJson == null) {
        val payload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)?.takeIf { it.isNotBlank() }
        val savedAt = prefs.getLong(TaskWorkService.PREF_PENDING_WORK_TIME, 0L)
        val tooOld = savedAt <= 0L || now - savedAt > TaskWorkService.PENDING_WORK_TTL_MS
        if (payload != null && !tooOld) {
            val traceId = taskPayloadTraceId(payload)
            if (traceId != null && !activeTasks.containsKey(traceId)) {
                restored += RunningTask(
                    traceId = traceId,
                    projectId = taskPayloadString(payload, "project_id"),
                    conversationId = taskPayloadString(payload, "conversation_id"),
                    payload = payload,
                    isDevelopment = prefs.getBoolean(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT, true),
                    startedAtMs = savedAt
                )
            }
        }
    }

    restored.forEach { activeTasks[it.traceId] = it }
    persistTaskWork(prefs, activeTasks.values)
}

internal fun taskPendingWorkAgeMs(tasks: List<RunningTask>): Long? {
    val oldest = tasks.map { it.startedAtMs }.filter { it > 0L }.minOrNull() ?: return null
    return System.currentTimeMillis() - oldest
}
