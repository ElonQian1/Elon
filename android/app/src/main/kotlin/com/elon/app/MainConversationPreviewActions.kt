package com.elon.app

import android.view.View
import com.elon.app.databinding.ActivityMainBinding

internal class MainConversationPreviewActions(
    private val binding: ActivityMainBinding,
    private val projects: () -> List<AppProject>,
    private val conversations: () -> MutableList<AppConversation>,
    private val activeProject: () -> AppProject,
    private val activeConversation: () -> AppConversation,
    private val activeProjectIndex: () -> Int,
    private val activeConversationIndex: () -> Int,
    private val chatAdapter: () -> ChatAdapter,
    private val conversationTaskKey: (String, String) -> String,
    private val workflowTerminalRoles: Set<String>,
    private val closeStaleWorkflowMessages: (MutableList<ChatMessage>) -> Unit,
    private val hasRunningTasks: () -> Boolean,
    private val saveConversations: () -> Unit,
    private val saveProjects: () -> Unit,
    private val reloadProjects: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val renderProjectList: () -> Unit
) {
    fun findConversationLocationByKey(key: String): Pair<Int, Int>? {
        projects().forEachIndexed { projectIndex, project ->
            project.conversations.forEachIndexed { conversationIndex, _ ->
                if (conversationTaskKey(project.id, project.conversations[conversationIndex].id) == key) {
                    return projectIndex to conversationIndex
                }
            }
        }
        return null
    }

    fun ensureConversationLocationByKey(key: String): Pair<Int, Int>? {
        findConversationLocationByKey(key)?.let { return it }
        reloadProjects()
        renderConversationList()
        renderProjectList()
        return findConversationLocationByKey(key)
    }

    fun appendMessageToConversation(projectIndex: Int, conversationIndex: Int, message: ChatMessage) {
        val project = projects().getOrNull(projectIndex) ?: return
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        if (message.role in workflowTerminalRoles) {
            mergeProcessLayerIntoTerminal(conversation.messages, message)
            closeStaleWorkflowMessages(conversation.messages)
        }
        conversation.messages.add(message)
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        if (!conversation.ended) {
            conversation.subtitle = summarize(message.content, 30)
            project.subtitle = summarize(message.content, 34)
        }
        saveProjects()
        renderConversationList()
        if (isAdapterShowing(projectIndex, conversationIndex)) {
            chatAdapter().notifyDataSetChanged()
            binding.chatList.scrollToPosition(chatAdapter().itemCount - 1)
        }
    }

    fun appendEvidenceToConversation(
        projectIndex: Int,
        conversationIndex: Int,
        entry: EvidenceEntry,
        working: Boolean
    ) {
        val project = projects().getOrNull(projectIndex) ?: return
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        val clean = sanitizeEvidenceDetail(entry.text)
        if (clean.isBlank()) return
        val latestUserIndex = conversation.messages.indexOfLast { it.role == "user" }
        val latestAssistantIndex = conversation.messages.indices.lastOrNull { index ->
            index > latestUserIndex &&
                conversation.messages[index].role == "ai" &&
                !conversation.messages[index].processLayer
        } ?: -1
        val evidenceFloor = maxOf(latestUserIndex, latestAssistantIndex)
        val evidenceIndex = conversation.messages.indices.lastOrNull { index ->
            index > evidenceFloor &&
                conversation.messages[index].role in MainWorkflowRoles.assistantEvidence &&
                conversation.messages[index].processLayer
        } ?: run {
            conversation.messages.add(
                ChatMessage(
                    role = "ai-intent",
                    content = "我正在处理这次请求，过程会折叠在这里。",
                    evidenceWorking = working,
                    processLayer = true
                )
            )
            conversation.messages.lastIndex
        }
        val target = conversation.messages[evidenceIndex]
        val entries = evidenceEntriesFromDetails(target.evidenceDetails).toMutableList()
        entries.add(EvidenceEntry(entry.kind, summarize(clean, 96)))
        while (entries.size > 40) entries.removeAt(0)
        applyEvidenceEntriesToMessage(target, entries, working)
        markConversationUpdated(project, conversation, target.content)
        saveProjects()
        notifyConversationChanged(projectIndex, conversationIndex, evidenceIndex)
    }

    fun stopEvidenceForConversation(projectIndex: Int, conversationIndex: Int) {
        val project = projects().getOrNull(projectIndex) ?: return
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        var changed = false
        conversation.messages.forEach { message ->
            if (message.evidenceWorking) {
                message.evidenceWorking = false
                changed = true
            }
        }
        if (!changed) return
        saveProjects()
        notifyConversationChanged(projectIndex, conversationIndex, null)
    }

    fun appendStreamChunkToConversation(
        projectIndex: Int,
        conversationIndex: Int,
        streamId: String,
        chunk: String
    ) {
        val project = projects().getOrNull(projectIndex) ?: return
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        val index = conversation.messages.indexOfLast { it.streamId == streamId }
        if (index < 0) return
        conversation.messages[index].content += chunk
        markConversationUpdated(project, conversation, conversation.messages[index].content)
        saveProjects()
        notifyConversationChanged(projectIndex, conversationIndex, index)
    }

    fun updateFirstConversationStatus(text: String) {
        val conversations = conversations()
        if (conversations.isEmpty()) conversations.add(defaultAppConversation())
        if (conversations[0].ended) return
        conversations[0].subtitle = text
        saveConversations()
        renderConversationList()
    }

    fun updateIdleReadyStatus() {
        if (!hasRunningTasks()) {
            updateFirstConversationStatus("已就绪 · 点击进入开发会话")
        }
    }

    fun prepareActiveConversationTitle(messageText: String) {
        if (refreshConversationTitleFromUserMessage(activeConversation(), messageText)) {
            saveConversations()
            renderConversationList()
        }
    }

    fun updateActiveConversationPreview(message: ChatMessage) {
        val conversation = activeConversation()
        val project = activeProject()
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        when (message.role) {
            "user" -> updateUserPreview(conversation, project, message)
            "ai", "ai-intent", "ai-working", "ai-progress", "ai-tool", "ai-complete", "ai-stopped", "error" -> {
                if (!conversation.ended) {
                    conversation.subtitle = summarize(message.content, 30)
                    project.subtitle = summarize(message.content, 34)
                }
            }
        }
        saveConversations()
        renderConversationList()
        if (binding.projectPage.visibility == View.VISIBLE) renderProjectList()
    }

    private fun updateUserPreview(
        conversation: AppConversation,
        project: AppProject,
        message: ChatMessage
    ) {
        val previewText = previewTextForChatContent(message.content, message.attachments)
        conversation.subtitle = summarize(previewText, 30)
        project.subtitle = summarize(previewText, 34)
        refreshConversationTitleFromUserMessage(conversation, previewText)
    }

    private fun refreshConversationTitleFromUserMessage(
        conversation: AppConversation,
        messageText: String
    ): Boolean {
        if (!shouldAutoGenerateConversationTitle(conversation)) return false
        val changed = updateConversationTitleFromUserMessage(conversation, messageText)
        if (changed) binding.topTitleText.text = conversation.title
        return changed
    }

    private fun markConversationUpdated(
        project: AppProject,
        conversation: AppConversation,
        preview: String
    ) {
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        if (!conversation.ended) {
            conversation.subtitle = summarize(preview, 30)
            project.subtitle = summarize(preview, 34)
        }
    }

    private fun notifyConversationChanged(projectIndex: Int, conversationIndex: Int, messageIndex: Int?) {
        renderConversationList()
        if (binding.projectPage.visibility == View.VISIBLE) renderProjectList()
        if (isAdapterShowing(projectIndex, conversationIndex)) {
            if (messageIndex != null) {
                chatAdapter().notifyMessageUpdated(messageIndex)
            } else {
                chatAdapter().notifyDataSetChanged()
            }
            binding.chatList.scrollToPosition(chatAdapter().itemCount - 1)
        }
    }

    private fun isAdapterShowing(projectIndex: Int, conversationIndex: Int): Boolean {
        val conversation = projects()
            .getOrNull(projectIndex)
            ?.conversations
            ?.getOrNull(conversationIndex)
            ?: return false
        return projectIndex == activeProjectIndex() &&
            conversationIndex == activeConversationIndex() &&
            chatAdapter().ownsMessages(conversation.messages)
    }

}
