package com.elon.app

import android.content.Intent
import android.content.SharedPreferences
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject

internal class MainTaskWorkServiceActions(
    private val activity: AppCompatActivity,
    private val prefs: SharedPreferences,
    private val appendTaskMessage: (String, String?, String?, String?, Boolean?) -> Unit,
    private val appendRawMessage: (String) -> Unit
) {
    fun startTaskWorkService(
        action: String,
        payload: String? = null,
        isDevelopment: Boolean,
        traceId: String? = null
    ): Boolean {
        val intent = Intent(activity, TaskWorkService::class.java).apply {
            this.action = action
            payload?.let { putExtra(TaskWorkService.EXTRA_PAYLOAD, it) }
            putExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, isDevelopment)
            traceId?.let { putExtra(TaskWorkService.EXTRA_TRACE_ID, it) }
        }
        return runCatching {
            if (action == TaskWorkService.ACTION_START_WORK || action == TaskWorkService.ACTION_RESUME_PENDING) {
                ContextCompat.startForegroundService(activity, intent)
            } else {
                activity.startService(intent)
            }
        }.recoverCatching {
            activity.startService(intent)
        }.isSuccess
    }

    fun setTaskAppForeground(foreground: Boolean) {
        prefs.edit().putBoolean(TaskWorkService.PREF_APP_IN_FOREGROUND, foreground).apply()
    }

    fun drainQueuedTaskEvents() {
        val queued = prefs.getString(TaskWorkService.PREF_QUEUED_TASK_EVENTS, null)?.takeIf { it.isNotBlank() }
            ?: return
        prefs.edit().remove(TaskWorkService.PREF_QUEUED_TASK_EVENTS).apply()
        runCatching {
            val array = JSONArray(queued)
            for (index in 0 until array.length()) {
                val item = array.opt(index)
                if (item is JSONObject) {
                    appendQueuedObject(item)
                } else {
                    array.optString(index).takeIf { it.isNotBlank() }?.let { appendRawMessage(it) }
                }
            }
        }
    }

    private fun appendQueuedObject(item: JSONObject) {
        val raw = item.optString("raw").takeIf { it.isNotBlank() } ?: return
        appendTaskMessage(
            raw,
            item.optString("trace_id").takeIf { it.isNotBlank() },
            item.optString("project_id").takeIf { it.isNotBlank() },
            item.optString("conversation_id").takeIf { it.isNotBlank() },
            if (item.has("is_development")) item.optBoolean("is_development", true) else null
        )
    }
}
