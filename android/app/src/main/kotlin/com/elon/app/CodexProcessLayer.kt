package com.elon.app

/**
 * Keeps Codex-style public process events attached to the final answer instead
 * of leaving many transient "working" bubbles in the chat history.
 */
internal fun mergeProcessLayerIntoTerminal(
    messages: MutableList<ChatMessage>,
    terminalMessage: ChatMessage
): Boolean {
    if (terminalMessage.role !in MainWorkflowRoles.terminal) return false
    val latestUserIndex = messages.indexOfLast { it.role == "user" }
    if (latestUserIndex < 0) return false

    val merged = mutableListOf<EvidenceEntry>()
    fun addEntry(entry: EvidenceEntry) {
        val clean = sanitizeEvidenceDetail(entry.text)
        if (clean.isBlank()) return
        val normalized = EvidenceEntry(entry.kind, summarize(clean, 96))
        if (merged.any { it.kind == normalized.kind && it.text == normalized.text }) return
        merged.add(normalized)
    }

    for (index in latestUserIndex + 1..messages.lastIndex) {
        val message = messages[index]
        evidenceEntriesFromDetails(message.evidenceDetails).forEach(::addEntry)
        processLayerContentEvidence(message)?.let(::addEntry)
    }
    evidenceEntriesFromDetails(terminalMessage.evidenceDetails).forEach(::addEntry)

    if (merged.isNotEmpty()) {
        applyEvidenceEntriesToMessage(terminalMessage, merged, working = false)
    }

    var removed = false
    for (index in messages.lastIndex downTo latestUserIndex + 1) {
        if (isTransientCodexProcessMessage(messages[index])) {
            messages.removeAt(index)
            removed = true
        }
    }
    return removed || merged.isNotEmpty()
}

internal fun isTransientCodexProcessMessage(message: ChatMessage): Boolean {
    return message.processLayer ||
        message.role == "ai-intent" ||
        message.role in MainWorkflowRoles.staleWorkflow
}

private fun processLayerContentEvidence(message: ChatMessage): EvidenceEntry? {
    if (!isTransientCodexProcessMessage(message)) return null
    val clean = sanitizeEvidenceDetail(message.content)
    if (clean.isBlank()) return null
    if (isRoutineWorkflowMessage(clean)) return null
    if (clean == "我正在处理这次请求，过程会折叠在这里。") return null
    return EvidenceEntry("progress", clean)
}
