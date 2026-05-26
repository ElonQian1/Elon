package com.elon.app.mcp

import com.elon.app.*
import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

internal fun pendingWorkAgeMs(prefs: SharedPreferences): Long? {
    val tasks = pendingTasksJson(prefs)
    if (tasks.length() > 0) {
        val now = System.currentTimeMillis()
        var oldest = Long.MAX_VALUE
        for (index in 0 until tasks.length()) {
            val savedAt = tasks.optJSONObject(index)?.optLong("started_at", 0L) ?: 0L
            if (savedAt > 0L && savedAt < oldest) oldest = savedAt
        }
        if (oldest != Long.MAX_VALUE) return now - oldest
    }
    val savedAt = prefs.getLong(TaskWorkService.PREF_PENDING_WORK_TIME, 0L)
    return if (savedAt > 0L) System.currentTimeMillis() - savedAt else null
}

internal fun isTaskBusy(prefs: SharedPreferences): Boolean {
    if (pendingTasksJson(prefs).length() > 0) return true
    val payload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
        ?.takeIf { it.isNotBlank() }
        ?: return false
    val savedAt = prefs.getLong(TaskWorkService.PREF_PENDING_WORK_TIME, 0L)
    val expired = savedAt > 0L && System.currentTimeMillis() - savedAt > TaskWorkService.PENDING_WORK_TTL_MS
    return !expired && payload.isNotBlank()
}

internal fun pendingTaskJson(prefs: SharedPreferences): JSONObject? {
    val tasks = pendingTasksJson(prefs)
    for (index in 0 until tasks.length()) {
        val payload = tasks.optJSONObject(index)?.optString("payload")?.takeIf { it.isNotBlank() }
        val parsed = runCatching { payload?.let { JSONObject(it) } }.getOrNull()
        if (parsed != null) return parsed
    }
    val payload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
        ?.takeIf { it.isNotBlank() }
        ?: return null
    return runCatching { JSONObject(payload) }.getOrNull()
}

internal fun isTracePending(prefs: SharedPreferences, traceId: String): Boolean {
    val tasks = pendingTasksJson(prefs)
    for (index in 0 until tasks.length()) {
        val payload = tasks.optJSONObject(index)?.optString("payload")
        if (traceIdFromPayload(payload) == traceId) return true
    }
    return traceIdFromPayload(prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)) == traceId
}

internal fun pendingTasksJson(prefs: SharedPreferences): JSONArray {
    val raw = prefs.getString(TaskWorkService.PREF_PENDING_WORK_TASKS, null)?.takeIf { it.isNotBlank() }
        ?: return JSONArray()
    val source = runCatching { JSONArray(raw) }.getOrElse { JSONArray() }
    val now = System.currentTimeMillis()
    val active = JSONArray()
    for (index in 0 until source.length()) {
        val item = source.optJSONObject(index) ?: continue
        val savedAt = item.optLong("started_at", 0L)
        val expired = savedAt > 0L && now - savedAt > TaskWorkService.PENDING_WORK_TTL_MS
        val payload = item.optString("payload").takeIf { it.isNotBlank() }
        if (!expired && payload != null) active.put(item)
    }
    return active
}

internal fun pendingTaskKind(prefs: SharedPreferences): String {
    if (!isTaskBusy(prefs)) return "idle"
    val tasks = pendingTasksJson(prefs)
    if (tasks.length() > 0) {
        return if (tasks.optJSONObject(0)?.optBoolean("is_development", true) != false) {
            "development"
        } else {
            "chat"
        }
    }
    return if (prefs.getBoolean(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT, true)) {
        "development"
    } else {
        "chat"
    }
}

internal fun reservePendingTask(prefs: SharedPreferences, payload: String, isDevelopment: Boolean) {
    val tasks = pendingTasksJson(prefs)
    tasks.put(
        JSONObject()
            .put("payload", payload)
            .put("is_development", isDevelopment)
            .put("started_at", System.currentTimeMillis())
    )
    prefs.edit()
        .putString(TaskWorkService.PREF_PENDING_WORK_TASKS, tasks.toString())
        .remove(TaskWorkService.PREF_PENDING_WORK_PAYLOAD)
        .remove(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT)
        .remove(TaskWorkService.PREF_PENDING_WORK_TIME)
        .apply()
}

internal fun clearPersistedTask(prefs: SharedPreferences) {
    prefs.edit()
        .remove(TaskWorkService.PREF_PENDING_WORK_TASKS)
        .remove(TaskWorkService.PREF_PENDING_WORK_PAYLOAD)
        .remove(TaskWorkService.PREF_PENDING_WORK_IS_DEVELOPMENT)
        .remove(TaskWorkService.PREF_PENDING_WORK_TIME)
        .apply()
}

internal fun clearReservedPendingTask(prefs: SharedPreferences, traceId: String) {
    val tasks = pendingTasksJson(prefs)
    if (tasks.length() > 0) {
        val kept = JSONArray()
        for (index in 0 until tasks.length()) {
            val item = tasks.optJSONObject(index) ?: continue
            val currentTraceId = traceIdFromPayload(item.optString("payload"))
            if (currentTraceId != traceId) kept.put(item)
        }
        prefs.edit()
            .putString(TaskWorkService.PREF_PENDING_WORK_TASKS, kept.toString())
            .apply()
        return
    }
    val currentTraceId = traceIdFromPayload(
        prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
    )
    if (currentTraceId == traceId) clearPersistedTask(prefs)
}

internal fun traceIdFromPayload(payload: String?): String? {
    return payload
        ?.let { runCatching { JSONObject(it).optString("trace_id") }.getOrNull() }
        ?.takeIf { it.isNotBlank() }
}

internal fun queuedTaskEvents(prefs: SharedPreferences): List<String> {
    val raw = prefs.getString(TaskWorkService.PREF_QUEUED_TASK_EVENTS, null)
        ?.takeIf { it.isNotBlank() }
        ?: return emptyList()
    val array = runCatching { JSONArray(raw) }.getOrElse { return emptyList() }
    return buildList {
        for (index in 0 until array.length()) {
            array.optString(index).takeIf { it.isNotBlank() }?.let { add(it) }
        }
    }
}

internal fun rawTaskEventJson(raw: String): JSONObject {
    val parsed = runCatching { JSONObject(raw) }.getOrNull()
    return JSONObject()
        .put("raw", raw.take(20_000))
        .put("json", parsed ?: JSONObject.NULL)
}
