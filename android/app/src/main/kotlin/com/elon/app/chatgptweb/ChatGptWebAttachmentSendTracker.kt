package com.elon.app.chatgptweb

internal class ChatGptWebAttachmentSendTracker private constructor(
    val prompt: String,
    val localAttachmentCount: Int,
    private val baselineAttachmentIds: Set<String>,
    private val baselineUserMessageIds: Set<String>,
) {
    private var transportSequence = 0L
    private var transportCompletedCount = 0

    enum class Phase(val wireValue: String) {
        UPLOADING("uploading"),
        SENDING("sending"),
        FAILED("failed"),
    }

    sealed interface Observation {
        data object Wait : Observation
        data object SendPrompt : Observation
        data class Complete(val userMessageId: String) : Observation
        data class Failed(val detail: String) : Observation
    }

    var phase: Phase = Phase.UPLOADING
        private set

    fun observe(snapshot: ChatGptWebSnapshot): Observation {
        newUserMessage(snapshot)?.let { return Observation.Complete(it.id) }
        if (phase != Phase.UPLOADING) return Observation.Wait

        val uploaded = snapshot.attachments.filterNot { it.id in baselineAttachmentIds }
        if (uploaded.any { it.state == "error" }) {
            phase = Phase.FAILED
            return Observation.Failed("附件上传失败，请重试或打开官网功能。")
        }
        val domReady = uploaded.size >= localAttachmentCount && uploaded.none { it.state != "ready" }
        val transportReady = transportCompletedCount >= localAttachmentCount &&
            snapshot.composerReady && !snapshot.streaming
        if (!domReady && !transportReady) {
            return Observation.Wait
        }
        phase = Phase.SENDING
        return Observation.SendPrompt
    }

    fun observeTransport(evidence: ChatGptWebAttachmentTransportEvidence): Observation {
        if (phase != Phase.UPLOADING || !evidence.supported || evidence.sequence <= transportSequence) {
            return Observation.Wait
        }
        transportSequence = evidence.sequence
        if (evidence.state == ChatGptWebAttachmentTransportState.COMPLETED) {
            transportCompletedCount = maxOf(transportCompletedCount, evidence.completedCount)
        }
        return Observation.Wait
    }

    fun markSendFailed() {
        phase = Phase.FAILED
    }

    fun uploadedAttachmentIds(snapshot: ChatGptWebSnapshot): List<String> = snapshot.attachments
        .asSequence()
        .filterNot { it.id in baselineAttachmentIds }
        .filter(ChatGptWebAttachment::removable)
        .map(ChatGptWebAttachment::id)
        .toList()

    private fun newUserMessage(snapshot: ChatGptWebSnapshot): ChatGptWebMessage? {
        val newUserMessages = snapshot.messages.filter { it.role == "user" && it.id !in baselineUserMessageIds }
        if (newUserMessages.isEmpty()) return null
        val cleanPrompt = prompt.trim()
        return if (cleanPrompt.isEmpty()) {
            newUserMessages.lastOrNull { message ->
                message.parts.any { part -> part.type == "file" || part.type == "image" }
            }
        } else {
            newUserMessages.lastOrNull { it.content.trim() == cleanPrompt }
        }
    }

    companion object {
        fun begin(
            prompt: String,
            localAttachmentCount: Int,
            snapshot: ChatGptWebSnapshot,
        ): ChatGptWebAttachmentSendTracker {
            require(localAttachmentCount > 0)
            return ChatGptWebAttachmentSendTracker(
                prompt = prompt.trim(),
                localAttachmentCount = localAttachmentCount,
                baselineAttachmentIds = snapshot.attachments.mapTo(linkedSetOf(), ChatGptWebAttachment::id),
                baselineUserMessageIds = snapshot.messages
                    .asSequence()
                    .filter { it.role == "user" }
                    .mapTo(linkedSetOf(), ChatGptWebMessage::id),
            )
        }
    }
}

internal data class ChatGptWebAttachmentSendUpdate(
    val phase: String,
    val attachmentCount: Int,
    val detail: String? = null,
    val userMessageId: String? = null,
)
