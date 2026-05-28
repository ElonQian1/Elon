package com.elon.app

internal class MainEvidenceActions(
    private val activeConversation: () -> AppConversation,
    private val chatAdapter: () -> ChatAdapter,
    private val saveConversations: () -> Unit,
    private val assistantEvidenceRoles: Set<String>
) {
    private val entries = mutableListOf<EvidenceEntry>()

    fun recordEvidence(kind: String, detail: String) {
        val clean = sanitizeEvidenceDetail(detail)
        if (clean.isBlank()) return

        entries.add(EvidenceEntry(kind, summarize(clean, 96)))
        while (entries.size > 40) {
            entries.removeAt(0)
        }
        attachEvidenceToLatestAi()
    }

    fun attachEvidenceToLatestAi() {
        if (entries.isEmpty()) return
        val messages = activeConversation().messages
        val latestUserIndex = messages.indexOfLast { it.role == "user" }
        val index = messages.indices.lastOrNull { it > latestUserIndex && messages[it].role in assistantEvidenceRoles }
            ?: return

        // 清除其他消息上遗留的 evidenceWorking 标志，避免旧折叠层误触发闪烁动画
        messages.forEachIndexed { i, msg ->
            if (i != index && msg.evidenceWorking) {
                msg.evidenceWorking = false
                chatAdapter().notifyMessageUpdated(i)
            }
        }

        applyEvidenceToMessage(messages[index], entries, working = true)
        chatAdapter().notifyMessageUpdated(index)
        saveConversations()
    }

    fun finalizeEvidenceForLatestAssistant() {
        if (entries.isEmpty()) return
        val messages = activeConversation().messages
        val latestUserIndex = messages.indexOfLast { it.role == "user" }
        val index = messages.indices.lastOrNull { it > latestUserIndex && messages[it].role in assistantEvidenceRoles }
            ?: return

        applyEvidenceToMessage(messages[index], entries, working = false)
        entries.clear()
        chatAdapter().notifyMessageUpdated(index)
        saveConversations()
    }

    fun aiMessageWithCurrentEvidence(
        content: String,
        attachments: List<ChatAttachment> = emptyList()
    ): ChatMessage {
        val message = ChatMessage("ai", content, attachments.takeIf { it.isNotEmpty() })
        if (entries.isNotEmpty()) {
            clearDuplicateCurrentEvidenceFromActiveConversation(entries)
            stopWorkingEvidenceForActiveConversation()
            applyEvidenceToMessage(message, entries, working = false)
            entries.clear()
        }
        return message
    }

    fun stopWorkingEvidenceForActiveConversation() {
        var changed = false
        activeConversation().messages.forEachIndexed { index, message ->
            if (message.evidenceWorking) {
                message.evidenceWorking = false
                chatAdapter().notifyMessageUpdated(index)
                changed = true
            }
        }
        if (changed) saveConversations()
    }

    fun clearCurrentEvidence() {
        entries.clear()
    }

    private fun applyEvidenceToMessage(message: ChatMessage, entries: List<EvidenceEntry>, working: Boolean) {
        message.evidenceTitle = evidenceTitle(entries)
        message.evidenceDetails = evidenceDetails(entries)
        message.evidenceWorking = working
    }

    private fun clearDuplicateCurrentEvidenceFromActiveConversation(currentEntries: List<EvidenceEntry>) {
        val title = evidenceTitle(currentEntries)
        val details = evidenceDetails(currentEntries)
        var changed = false
        activeConversation().messages.forEachIndexed { index, message ->
            val sameCurrentEvidence = message.role in assistantEvidenceRoles &&
                message.evidenceTitle == title &&
                message.evidenceDetails == details
            if (sameCurrentEvidence) {
                message.evidenceTitle = null
                message.evidenceDetails = null
                message.evidenceExpanded = false
                message.evidenceWorking = false
                chatAdapter().notifyMessageUpdated(index)
                changed = true
            }
        }
        if (changed) saveConversations()
    }
}
