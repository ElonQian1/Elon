package com.elon.app

import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

private const val MAX_QUEUED_EVENTS = 120
private const val MAX_QUEUED_EVENT_LENGTH = 20_000

internal fun queueTaskRawEvent(prefs: SharedPreferences, task: RunningTask, raw: String) {
    val queue = runCatching {
        JSONArray(prefs.getString(TaskWorkService.PREF_QUEUED_TASK_EVENTS, "[]"))
    }.getOrElse { JSONArray() }
    queue.put(
        JSONObject()
            .put("raw", raw.take(MAX_QUEUED_EVENT_LENGTH))
            .put("trace_id", task.traceId)
            .put("project_id", task.projectId)
            .put("conversation_id", task.conversationId)
            .put("is_development", task.isDevelopment)
    )
    while (queue.length() > MAX_QUEUED_EVENTS) {
        queue.remove(0)
    }
    prefs.edit().putString(TaskWorkService.PREF_QUEUED_TASK_EVENTS, queue.toString()).apply()
}

internal fun isTaskAppInForeground(prefs: SharedPreferences): Boolean {
    return prefs.getBoolean(TaskWorkService.PREF_APP_IN_FOREGROUND, false)
}
