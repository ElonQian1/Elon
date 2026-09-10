package com.elon.app

internal object WebChatLibraryAttachmentReceiptPolicy {
    // Scope read (10s) and mounted preparation (12s) share the page operation budget.
    const val OPERATION_TIMEOUT_MS = 24_000L
    const val COMMAND_TIMEOUT_MS = 30_000L
    const val OBSERVATION_TIMEOUT_MS = 31_000L

    enum class Outcome { WAIT, ATTACHED, UNCONFIRMED }

    fun outcome(succeeded: Boolean, terminal: Boolean, detail: String?, elapsedMs: Long): Outcome = when {
        succeeded && detail == "library_attachment_associated" -> Outcome.ATTACHED
        succeeded || terminal || elapsedMs >= OBSERVATION_TIMEOUT_MS -> Outcome.UNCONFIRMED
        else -> Outcome.WAIT
    }
}
