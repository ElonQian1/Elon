package com.elon.app

internal class MainProgressNarrativeActions(
    private val isDevelopmentRequest: () -> Boolean,
    private val recordEvidence: (String, String) -> Unit,
    private val attachEvidenceToLatestAi: () -> Unit
) {
    private val emittedProgressSignals = linkedSetOf<String>()

    fun clear() {
        emittedProgressSignals.clear()
    }

    fun maybeAppendVisibleCliSignal(category: String, line: String): Boolean {
        if (!isDevelopmentRequest()) return false
        val narrative = CodexProgressNarrative.fromCliOutput(category, line) ?: return false
        return appendProgressNarrative(narrative)
    }

    fun maybeAppendWorkflowProgressNarrative(content: String): Boolean {
        if (!isDevelopmentRequest()) return false
        val narrative = CodexProgressNarrative.fromWorkflowProgress(content) ?: return false
        return appendProgressNarrative(narrative)
    }

    fun maybeAppendTaskEventNarrative(event: String, content: String): Boolean {
        if (!isDevelopmentRequest()) return false
        val narrative = CodexProgressNarrative.fromTaskEvent(event, content) ?: return false
        return appendProgressNarrative(narrative)
    }

    fun maybeAppendToolCallNarrative(tool: String): Boolean {
        if (!isDevelopmentRequest()) return false
        val narrative = CodexProgressNarrative.fromToolCall(tool) ?: return false
        return appendProgressNarrative(narrative)
    }

    private fun appendProgressNarrative(narrative: CodexProgressNarrative.Narrative): Boolean {
        if (!emittedProgressSignals.add(narrative.key)) return false
        recordEvidence("progress", narrative.message.content)
        attachEvidenceToLatestAi()
        return true
    }
}
