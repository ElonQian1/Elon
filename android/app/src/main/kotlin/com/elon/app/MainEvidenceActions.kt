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
}
