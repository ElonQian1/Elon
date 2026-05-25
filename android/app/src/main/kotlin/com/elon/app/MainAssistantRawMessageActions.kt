package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.update.AppUpdateManager
import com.google.gson.JsonParser

internal class MainAssistantRawMessageActions(
    private val activity: AppCompatActivity,
    private val assistantStreamEvents: () -> MainAssistantStreamEvents,
    private val assistantTerminalActions: () -> MainAssistantTerminalActions,
    private val incrementServerResponseToken: () -> Unit,
    private val appendMessage: (ChatMessage) -> Unit
) {
    fun appendMessage(raw: String) {
        try {
            val json = JsonParser.parseString(raw).asJsonObject
            val type = jsonStringOrNull(json, "type") ?: return
            if (type == "app_update_available") {
                val remoteVersionCode = runCatching {
                    json.get("versionCode")?.asInt ?: 0
                }.getOrDefault(0)
                AppUpdateManager(activity).realtimeCheck(remoteVersionCode)
                return
            }

            incrementServerResponseToken()
            val msg = when (type) {
                "task_event" -> assistantStreamEvents().taskEventMessage(json) ?: return
                "progress" -> assistantStreamEvents().progressMessage(json) ?: return
                "tool_call" -> {
                    assistantStreamEvents().handleToolCall(json)
                    return
                }
                "tool_result" -> {
                    assistantStreamEvents().handleToolResult(json)
                    return
                }
                "assistant_message" -> assistantStreamEvents().assistantMessage(json) ?: return
                "done" -> {
                    val content = jsonStringOrNull(json, "message") ?: ""
                    val apkUrl = jsonStringOrNull(json, "apk_url")
                    val imageUrl = jsonStringOrNull(json, "image_url")
                    assistantTerminalActions().handleDone(content, apkUrl, imageUrl)
                }
                "error" -> {
                    assistantTerminalActions().handleError(jsonStringOrNull(json, "message") ?: "未知错误")
                }
                else -> return
            }
            appendMessage(msg)
        } catch (_: Exception) {
            assistantTerminalActions().handleMalformedResponse()
        }
    }
}
