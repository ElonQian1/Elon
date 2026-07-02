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
        val evidenceIndex = conversation.messages.indices.lastOrNull { index ->
            index > latestUserIndex && conversation.messages[index].role in MainWorkflowRoles.assistantEvidence
        } ?: run {
            conversation.messages.add(
                ChatMessage(
                    role = "ai-intent",
                    content = "我正在处理这次请求，过程会折叠在这里。",
                    evidenceWorking = working
                )
            )
            conversation.messages.lastIndex
        }
        val target = conversation.messages[evidenceIndex]
        val entries = evidenceEntriesFrom(target).toMutableList()
        entries.add(EvidenceEntry(entry.kind, summarize(clean, 96)))
        while (entries.size > 40) entries.removeAt(0)
        target.evidenceTitle = evidenceTitle(entries)
        target.evidenceDetails = evidenceDetails(entries)
        target.evidenceWorking = working
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
        if (updateConversationTitleFromUserMessage(activeConversation(), messageText)) {
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
        conversation.subtitle = summarize(message.content, 30)
        project.subtitle = summarize(message.content, 34)
        updateConversationTitleFromUserMessage(conversation, message.content)
    }

    private fun updateConversationTitleFromUserMessage(
        conversation: AppConversation,
        messageText: String
    ): Boolean {
        if (!shouldAutoGenerateConversationTitle(conversation)) return false
        val title = autoConversationTitleFromMessage(messageText)
        if (title.isBlank()) return false
        conversation.title = title
        binding.topTitleText.text = title
        return true
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

    private fun evidenceEntriesFrom(message: ChatMessage): List<EvidenceEntry> {
        return message.evidenceDetails
            ?.lineSequence()
            ?.mapNotNull(::evidenceEntryFromLine)
            ?.toList()
            .orEmpty()
    }

    private fun evidenceEntryFromLine(line: String): EvidenceEntry? {
        val cleaned = line.trim().removePrefix("·").trim()
        if (cleaned.isBlank()) return null
        val label = cleaned.substringBefore("：", "").trim()
        val text = cleaned.substringAfter("：", cleaned).trim()
        val kind = when (label) {
            "命令" -> "command"
            "文件" -> "file"
            "编辑" -> "edit"
            "构建" -> "build"
            "CLI" -> "cli"
            "环境" -> "env"
            "连接" -> "connection"
            "结果" -> "result"
            else -> "progress"
        }
        return EvidenceEntry(kind, text)
    }
}
