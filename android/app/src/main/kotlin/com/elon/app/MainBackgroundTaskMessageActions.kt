package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.update.AppUpdateManager
import org.json.JSONObject

internal class MainBackgroundTaskMessageActions(
    private val activity: AppCompatActivity,
    private val findConversationLocationByKey: (String) -> Pair<Int, Int>?,
    private val appendMessageToConversation: (Int, Int, ChatMessage) -> Unit
) {
    fun appendBackgroundTaskMessage(raw: String, key: String?, isDevelopment: Boolean) {
        val location = key?.let { findConversationLocationByKey(it) } ?: return
        val parsed = runCatching { JSONObject(raw) }.getOrNull() ?: return
        val type = parsed.optString("type").takeIf { it.isNotBlank() } ?: return
        if (type == "app_update_available") {
            AppUpdateManager(activity).realtimeCheck(parsed.optInt("versionCode", 0))
            return
        }
        val message = when (type) {
            "done" -> doneMessage(parsed, isDevelopment)
            "error" -> ChatMessage(
                "error",
                friendlyErrorMessage(parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务失败。")
            )
            "progress" -> progressMessage(parsed) ?: return
            else -> return
        }
        appendMessageToConversation(location.first, location.second, message)
    }

    private fun doneMessage(parsed: JSONObject, isDevelopment: Boolean): ChatMessage {
        val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务已完成。"
        val apkUrl = parsed.optString("apk_url").takeIf { it.isNotBlank() && it != "null" }
        val imageUrl = parsed.optString("image_url").takeIf { it.isNotBlank() && it != "null" }
        return ChatMessage(
            "ai",
            finalReplyMessage(content, if (isDevelopment) apkUrl else null, imageUrl, isDevelopment)
        )
    }

    private fun progressMessage(parsed: JSONObject): ChatMessage? {
        val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: return null
        val narrative = CodexProgressNarrative.fromWorkflowProgress(content)
        if (narrative == null && !shouldShowProgressBubble(content)) return null
        return narrative?.message ?: ChatMessage("ai-progress", workflowProgressMessage(content))
    }
}
