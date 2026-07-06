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
        val index = latestEvidenceIndexOrCreate(messages, latestUserIndex, working = true)
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
        val index = latestEvidenceIndexOrCreate(messages, latestUserIndex, working = false)
            ?: return

        applyEvidenceToMessage(messages[index], entries, working = false)
        entries.clear()
        chatAdapter().notifyMessageUpdated(index)
        saveConversations()
    }

    fun promoteLatestAssistantReplyWithCurrentEvidence(modelUsed: String?, nodeId: String?): Boolean {
        val messages = activeConversation().messages
        val latestUserIndex = messages.indexOfLast { it.role == "user" }
        val index = messages.indices.lastOrNull {
            it > latestUserIndex &&
                messages[it].role in assistantEvidenceRoles &&
                messages[it].content.isNotBlank() &&
                !messages[it].processLayer
        } ?: return false

        val source = messages[index]
        val promoted = source.copy(
            role = "ai",
            modelUsed = modelUsed ?: source.modelUsed,
            nodeId = nodeId ?: source.nodeId,
            finalReply = true
        )
        if (entries.isNotEmpty()) {
            clearDuplicateCurrentEvidenceFromActiveConversation(entries)
            applyEvidenceToMessage(promoted, entries, working = false)
            entries.clear()
        } else {
            promoted.evidenceWorking = false
        }

        var targetIndex = index
        var removedProcessLayer = false
        for (i in messages.lastIndex downTo latestUserIndex + 1) {
            if (i != targetIndex && isTransientCodexProcessMessage(messages[i])) {
                messages.removeAt(i)
                if (i < targetIndex) targetIndex -= 1
                removedProcessLayer = true
            }
        }

        messages.forEachIndexed { i, msg ->
            if (i != targetIndex && msg.evidenceWorking) {
                msg.evidenceWorking = false
                chatAdapter().notifyMessageUpdated(i)
            }
        }
        messages[targetIndex] = promoted
        if (removedProcessLayer) {
            chatAdapter().notifyDataSetChanged()
        } else {
            chatAdapter().notifyMessageUpdated(targetIndex)
        }
        saveConversations()
        return true
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

    private fun latestEvidenceIndexOrCreate(
        messages: MutableList<ChatMessage>,
        latestUserIndex: Int,
        working: Boolean
    ): Int? {
        val latestAssistantIndex = messages.indices.lastOrNull {
            it > latestUserIndex &&
                messages[it].role == "ai" &&
                !messages[it].processLayer
        } ?: -1
        val evidenceFloor = maxOf(latestUserIndex, latestAssistantIndex)
        messages.indices.lastOrNull {
            it > evidenceFloor &&
                messages[it].role in assistantEvidenceRoles &&
                messages[it].processLayer
        }
            ?.let { return it }

        val placeholder = ChatMessage(
            role = "ai-intent",
            content = "我正在处理这次请求，过程会折叠在这里。",
            evidenceWorking = working,
            processLayer = true
        )
        val adapter = chatAdapter()
        if (adapter.ownsMessages(messages)) {
            adapter.addMessage(placeholder)
        } else {
            messages.add(placeholder)
            adapter.notifyDataSetChanged()
        }
        saveConversations()
        return messages.indexOfLast { it === placeholder }.takeIf { it >= 0 }
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
