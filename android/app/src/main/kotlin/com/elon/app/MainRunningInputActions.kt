package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.gson.JsonArray
import com.google.gson.JsonObject

internal class MainRunningInputActions(
    private val activity: AppCompatActivity,
    private val projects: () -> List<AppProject>,
    private val activeConversation: () -> AppConversation,
    private val activeConversationTask: () -> ConversationTaskState?,
    private val appendMessage: (ChatMessage) -> Unit,
    private val updateMessage: (ChatMessage) -> Unit,
    private val startPreparedMessageAfterUserBubble:
        (String, String, JsonArray, SendTarget, List<ChatAttachment>) -> Unit,
    private val startTaskWorkService: (String, String?, Boolean, String?) -> Boolean,
    private val forkForRunningInput: (String, String) -> ForkedConversation,
    private val expandOutgoing: (String, MutableList<ChatMessage>) -> String
) {
    fun handleRunningInput(
        mode: RunningInputMode,
        visibleText: String,
        outgoingText: String,
        hasAttachments: Boolean
    ): Boolean {
        if (hasAttachments) {
            Toast.makeText(activity, "运行中输入暂不支持附件，请等当前任务结束后发送。", Toast.LENGTH_SHORT).show()
            return false
        }
        when (mode) {
            RunningInputMode.REMIND_CURRENT -> remindCurrent(outgoingText)
            RunningInputMode.QUEUE_NEXT -> queueNext(visibleText)
            RunningInputMode.FORK -> forkAndSend(visibleText, outgoingText)
        }
        return true
    }

    fun drainNextQueuedMessage(projectId: String?, conversationId: String?) {
        if (projectId.isNullOrBlank() || conversationId.isNullOrBlank()) return
        val (project, conversation) = findConversation(projectId, conversationId) ?: return
        val queued = conversation.messages.firstOrNull {
            it.role == "user" && it.sendStatus == QUEUED_NEXT_SEND_STATUS
        } ?: return
        queued.sendStatus = null
        if (conversation === activeConversation()) {
            updateMessage(queued)
        }
        val target = SendTarget(project.id, project.title, conversation.id, conversation.title)
        val outgoing = expandOutgoing(queued.content, conversation.messages)
        startPreparedMessageAfterUserBubble(queued.content, outgoing, JsonArray(), target, emptyList())
    }

    private fun remindCurrent(text: String) {
        val task = activeConversationTask()
        if (task == null) {
            Toast.makeText(activity, "当前没有可提醒的运行中任务。", Toast.LENGTH_SHORT).show()
            return
        }
        val payload = JsonObject().apply {
            addProperty("op", "runtime_note")
            addProperty("trace_id", task.traceId)
            addProperty("project_id", task.projectId)
            addProperty("conversation_id", task.conversationId)
            addProperty("message", text)
        }.toString()
        val sent = startTaskWorkService(
            TaskWorkService.ACTION_RUNTIME_INPUT,
            payload,
            task.isDevelopment,
            task.traceId
        )
        appendMessage(
            ChatMessage(
                "ai-progress",
                if (sent) "已把提醒记录到当前任务：${summarize(text, 40)}"
                else "提醒暂时没记录成功，请稍后重试。"
            )
        )
    }

    private fun queueNext(text: String) {
        appendMessage(ChatMessage("user", text, sendStatus = QUEUED_NEXT_SEND_STATUS))
        appendMessage(ChatMessage("ai-progress", "已排到下一轮，当前任务结束后自动发送。"))
    }

    private fun forkAndSend(visibleText: String, outgoingText: String) {
        val fork = forkForRunningInput(visibleText, outgoingText)
        appendMessage(ChatMessage("user", visibleText))
        startPreparedMessageAfterUserBubble(
            visibleText,
            fork.outgoingText,
            JsonArray(),
            fork.target,
            emptyList()
        )
    }

    private fun findConversation(
        projectId: String,
        conversationId: String
    ): Pair<AppProject, AppConversation>? {
        projects().forEach { project ->
            if (project.id == projectId) {
                project.conversations.firstOrNull { it.id == conversationId }?.let {
                    return project to it
                }
            }
        }
        return null
    }
}
