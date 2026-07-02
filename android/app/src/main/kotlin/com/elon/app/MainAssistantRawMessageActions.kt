package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.update.AppUpdateManager
import com.google.gson.JsonParser

internal class MainAssistantRawMessageActions(
    private val activity: AppCompatActivity,
    private val assistantStreamEvents: () -> MainAssistantStreamEvents,
    private val assistantTerminalActions: () -> MainAssistantTerminalActions,
    private val incrementServerResponseToken: () -> Unit,
    private val appendMessage: (ChatMessage) -> Unit,
    private val isDevelopmentRequest: () -> Boolean,
    /** 流式追加块到已有气泡（打字机效果），找不到 streamId 对应气泡时忽略 */
    private val streamAppendChunk: (streamId: String, chunk: String) -> Unit = { _, _ -> }
) {
    // 追踪当前 turn 是否已通过流式 assistant_message 推送过普通聊天回复。
    // 普通聊天的 done.message 常是冗余副本；开发任务则把 done.message 作为最终答复气泡。
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
                "assistant_chunk" -> {
                    // 流式追加到已有气泡（打字机效果），不创建新消息
                    val sid = jsonStringOrNull(json, "stream_id") ?: return
                    val chunk = jsonStringOrNull(json, "text") ?: return
                    streamAppendChunk(sid, chunk)
                    return
                }
                "done" -> {
                    val rawContent = jsonStringOrNull(json, "message") ?: ""
                    val apkUrl = jsonStringOrNull(json, "apk_url")
                    val imageUrl = jsonStringOrNull(json, "image_url")
                    val modelUsed = jsonStringOrNull(json, "model_used")
                    val nodeId = jsonStringOrNull(json, "node_id")
                    val suppressDuplicateDone = receivedStreamingReplyThisTurn && !isDevelopmentRequest()
                    val content = if (suppressDuplicateDone) "" else rawContent
                    receivedStreamingReplyThisTurn = false
                    assistantTerminalActions().handleDone(content, apkUrl, imageUrl, modelUsed, nodeId) ?: return
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
