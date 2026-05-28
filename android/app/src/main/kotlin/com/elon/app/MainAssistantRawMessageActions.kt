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
    // 追踪当前 turn 是否已通过流式 assistant_message 推送过 AI 回复。
    // 若是，done 事件里的 message 字段是相同内容的冗余副本，传 "" 给 handleDone 避免重复气泡。
    // background 任务走 MainBackgroundTaskMessageActions，此标志不影响那条路径。
    private var receivedStreamingReplyThisTurn = false

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
                "assistant_message" -> {
                    receivedStreamingReplyThisTurn = true
                    assistantStreamEvents().assistantMessage(json) ?: return
                }
                "done" -> {
                    val rawContent = jsonStringOrNull(json, "message") ?: ""
                    val apkUrl = jsonStringOrNull(json, "apk_url")
                    val imageUrl = jsonStringOrNull(json, "image_url")
                    // 若已收到流式 AI 回复，done.message 与最后一条 assistant_message 相同，
                    // 传空串给 handleDone 避免重复气泡
                    val content = if (receivedStreamingReplyThisTurn) "" else rawContent
                    receivedStreamingReplyThisTurn = false
                    assistantTerminalActions().handleDone(content, apkUrl, imageUrl) ?: return
                }
                "error" -> {
                    receivedStreamingReplyThisTurn = false
                    assistantTerminalActions().handleError(
                        jsonStringOrNull(json, "message") ?: "未知错误",
                        jsonStringOrNull(json, "code"),
                        jsonBooleanOrNull(json, "retryable")
                    )
                }
                else -> return
            }
            appendMessage(msg)
        } catch (_: Exception) {
            assistantTerminalActions().handleMalformedResponse()
        }
    }
}
