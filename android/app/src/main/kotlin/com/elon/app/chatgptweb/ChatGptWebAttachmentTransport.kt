package com.elon.app.chatgptweb

internal enum class ChatGptWebAttachmentTransportState(val wireValue: String) {
    ARMED("armed"),
    STARTED("started"),
    COMPLETED("completed"),
    FAILED("failed");

    companion object {
        fun fromWireValue(value: String): ChatGptWebAttachmentTransportState? =
            entries.firstOrNull { it.wireValue == value }
    }
}

internal data class ChatGptWebAttachmentTransportEvidence(
    val version: Int,
    val sequence: Long,
    val state: ChatGptWebAttachmentTransportState,
    val completedCount: Int,
) {
    val supported: Boolean
        get() = version == VERSION && sequence > 0L && completedCount in 0..MAX_ATTACHMENTS

    companion object {
        const val VERSION = 1
        const val MAX_ATTACHMENTS = 10
    }
}
