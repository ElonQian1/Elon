package com.elon.app

import com.elon.app.databinding.ActivityMainBinding
import com.google.gson.JsonArray

internal class MainSendMessageActions(
    private val binding: ActivityMainBinding,
    private val pendingAttachments: List<PendingAttachment>,
    private val collapseAttachmentPanel: () -> Unit,
    private val isActiveConversationWorking: () -> Boolean,
    private val runningInputMode: () -> RunningInputMode,
    private val activeProject: () -> AppProject,
    private val activeConversation: () -> AppConversation,
    private val appendMessage: (ChatMessage) -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val uploadAttachmentsThenSend: (String, String, SendTarget) -> Unit,
    private val startPreparedMessage: (String, String, JsonArray, SendTarget, List<ChatAttachment>) -> Unit,
    private val handleRunningInput:
        (RunningInputMode, String, String, Boolean) -> Boolean,
    private val trySendFriendMessage: (String, List<PendingAttachment>) -> Boolean
) {
    fun sendMessage() {
        collapseAttachmentPanel()
        val rawText = binding.inputEdit.text.toString().trim()
        if (rawText.isEmpty() && pendingAttachments.isEmpty()) return
        if (trySendFriendMessage(rawText, pendingAttachments)) return
        if (activeConversation().ended) {
            appendMessage(ChatMessage("error", "这个会话已结束，请新建会话继续。"))
            return
        }
        val text = if (pendingAttachments.isNotEmpty()) {
            visibleTextForPendingAttachments(rawText, pendingAttachments)
        } else {
            rawText
        }
        val outgoingText = expandShortDevelopmentCommand(text, activeConversation().messages)
        if (isActiveConversationWorking()) {
            val handled = handleRunningInput(
                runningInputMode(),
                text,
                outgoingText,
                pendingAttachments.isNotEmpty()
            )
            if (handled) {
                binding.inputEdit.text.clear()
                collapseInputComposer()
            }
            return
        }
        val target = currentSendTarget()
        collapseInputComposer()
        if (pendingAttachments.isNotEmpty()) {
            uploadAttachmentsThenSend(text, outgoingText, target)
            return
        }
        startPreparedMessage(text, outgoingText, JsonArray(), target, emptyList())
    }

    private fun currentSendTarget(): SendTarget {
        val project = activeProject()
        val conversation = activeConversation()
        return SendTarget(
            projectId = project.id,
            projectTitle = project.title,
            conversationId = conversation.id,
            conversationTitle = conversation.title
        )
    }
}
