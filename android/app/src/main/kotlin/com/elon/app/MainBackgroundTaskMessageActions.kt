package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.update.AppUpdateManager
import org.json.JSONObject

internal class MainBackgroundTaskMessageActions(
    private val activity: AppCompatActivity,
    private val findConversationLocationByKey: (String) -> Pair<Int, Int>?,
    private val appendMessageToConversation: (Int, Int, ChatMessage) -> Unit,
    private val appendEvidenceToConversation: (Int, Int, EvidenceEntry, Boolean) -> Unit,
    private val stopEvidenceForConversation: (Int, Int) -> Unit,
    private val appendStreamChunkToConversation: (Int, Int, String, String) -> Unit
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
            "done" -> {
                recordEvidence(location, doneEvidence(parsed), working = false, enabled = isDevelopment)
                stopEvidenceForConversation(location.first, location.second)
                doneMessage(parsed, isDevelopment)
            }
            "error" -> ChatMessage(
                "error",
                friendlyErrorMessage(
                    parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务失败。",
                    jsonStringOrNull(parsed, "code"),
                    jsonBooleanOrNull(parsed, "retryable")
                )
            ).also {
                recordEvidence(location, errorEvidence(parsed), working = false, enabled = isDevelopment)
                stopEvidenceForConversation(location.first, location.second)
            }
            "progress" -> {
                recordEvidence(location, progressEvidence(parsed), working = true, enabled = isDevelopment)
                progressMessage(parsed) ?: return
            }
            "task_event" -> {
                recordEvidence(location, taskEventEvidence(parsed), working = true, enabled = isDevelopment)
                taskEventMessage(parsed) ?: return
            }
            "tool_call" -> {
                recordEvidence(location, toolCallEvidence(parsed), working = true, enabled = isDevelopment)
                return
            }
            "tool_result" -> {
                recordEvidence(location, toolResultEvidence(parsed), working = true, enabled = isDevelopment)
                toolResultMessage(parsed) ?: return
            }
            "pc_dispatch_started", "runtime_status", "runtime_summary" -> {
                recordEvidence(location, structuredProcessEvidence(parsed), working = true, enabled = isDevelopment)
                return
            }
            "assistant_message" -> assistantMessage(parsed) ?: return
            "assistant_chunk" -> {
                val streamId = jsonStringOrNull(parsed, "stream_id") ?: return
                val chunk = jsonStringOrNull(parsed, "text") ?: return
                appendStreamChunkToConversation(location.first, location.second, streamId, chunk)
                return
            }
            "usage" -> {
                recordEvidence(location, usageEvidenceEntry(parsed), working = true, enabled = isDevelopment)
                return
            }
            else -> return
        }
        appendMessageToConversation(location.first, location.second, message)
    }

    private fun doneMessage(parsed: JSONObject, isDevelopment: Boolean): ChatMessage {
        val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务已完成。"
        val apkUrl = parsed.optString("apk_url").takeIf { it.isNotBlank() && it != "null" }
        val imageUrl = parsed.optString("image_url").takeIf { it.isNotBlank() && it != "null" }
        val visibleApkUrl = if (isDevelopment) apkUrl else null
        return ChatMessage(
            "ai",
            finalReplyMessage(content, visibleApkUrl, imageUrl, isDevelopment),
            apkUrl = visibleApkUrl
        )
    }

    private fun progressMessage(parsed: JSONObject): ChatMessage? {
        val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: return null
        if (isCliOutputProgress(content)) {
            val line = cleanCliOutputLine(content)
            val category = cliOutputCategory(line)
            return CodexProgressNarrative.fromCliOutput(category, line)?.message
        }
        val narrative = CodexProgressNarrative.fromWorkflowProgress(content)
        if (narrative == null && !shouldShowProgressBubble(content)) return null
        return narrative?.message ?: ChatMessage("ai-progress", workflowProgressMessage(content))
    }

    private fun taskEventMessage(parsed: JSONObject): ChatMessage? {
        val event = parsed.optString("event").takeIf { it.isNotBlank() } ?: return null
        val content = parsed.optString("message").takeIf { it.isNotBlank() }.orEmpty()
        val narrative = CodexProgressNarrative.fromTaskEvent(event, content)
        if (narrative != null) return narrative.message
        return if ((event == "accepted" || event == "runtime_note_received") && shouldShowProgressBubble(content)) {
            ChatMessage("ai-progress", workflowProgressMessage(content))
        } else {
            null
        }
    }

    private fun toolResultMessage(parsed: JSONObject): ChatMessage? {
        val result = parsed.optString("result").takeIf { it.isNotBlank() } ?: return null
        val lower = result.lowercase()
        if (!lower.contains("failed") && !lower.contains("error") && !result.contains("失败")) return null
        return ChatMessage("ai-progress", workflowProgressMessage("工具执行遇到问题：${summarize(result, 80)}"))
    }

    private fun assistantMessage(parsed: JSONObject): ChatMessage? {
        val text = jsonStringOrNull(parsed, "text") ?: return null
        return ChatMessage(
            role = "ai",
            content = text,
            modelUsed = jsonStringOrNull(parsed, "model_used"),
            nodeId = jsonStringOrNull(parsed, "node_id"),
            streamId = jsonStringOrNull(parsed, "stream_id")
        )
    }

    private fun recordEvidence(
        location: Pair<Int, Int>,
        entry: EvidenceEntry?,
        working: Boolean,
        enabled: Boolean
    ) {
        if (!enabled || entry == null) return
        appendEvidenceToConversation(location.first, location.second, entry, working)
    }

    private fun progressEvidence(parsed: JSONObject): EvidenceEntry? {
        val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: return null
        if (isRoutineHeartbeatProgress(content)) return null
        if (isCliOutputProgress(content)) {
            val line = cleanCliOutputLine(content)
            val category = cliOutputCategory(line)
            return EvidenceEntry(evidenceKindForCliCategory(category), line)
        }
        return EvidenceEntry("progress", userFacingProgress(content))
    }

    private fun taskEventEvidence(parsed: JSONObject): EvidenceEntry? {
        val event = parsed.optString("event").takeIf { it.isNotBlank() } ?: return null
        val content = parsed.optString("message").takeIf { it.isNotBlank() }
        val text = if (content == null) {
            "任务事件：$event"
        } else {
            "任务事件：$event，${userFacingProgress(content)}"
        }
        return EvidenceEntry("progress", text)
    }

    private fun toolCallEvidence(parsed: JSONObject): EvidenceEntry {
        val tool = parsed.optString("tool").takeIf { it.isNotBlank() } ?: "工具"
        val args = parsed.optJSONObject("args")?.toString()?.let { summarize(it, 96) }
        val detail = if (args.isNullOrBlank()) {
            "开始：${toolLabel(tool)}"
        } else {
            "开始：${toolLabel(tool)}，$args"
        }
        return EvidenceEntry(toolEvidenceKind(tool), detail)
    }

    private fun toolResultEvidence(parsed: JSONObject): EvidenceEntry {
        val tool = parsed.optString("tool").takeIf { it.isNotBlank() } ?: "工具"
        val result = parsed.optString("result").takeIf { it.isNotBlank() }
        val detail = if (result.isNullOrBlank()) {
            "完成：${toolLabel(tool)}"
        } else {
            "完成：${toolLabel(tool)}，${summarize(result, 96)}"
        }
        return EvidenceEntry(toolEvidenceKind(tool), detail)
    }

    private fun doneEvidence(parsed: JSONObject): EvidenceEntry {
        val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务已完成。"
        return EvidenceEntry("result", summarize(content, 96))
    }

    private fun errorEvidence(parsed: JSONObject): EvidenceEntry {
        val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务失败。"
        return EvidenceEntry("result", summarize(content, 96))
    }

    private fun usageEvidenceEntry(parsed: JSONObject): EvidenceEntry? {
        return usageEvidence(parsed)
    }
}
