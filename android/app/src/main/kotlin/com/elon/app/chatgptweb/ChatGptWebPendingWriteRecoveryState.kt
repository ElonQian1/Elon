package com.elon.app.chatgptweb

internal object ChatGptWebPendingWriteRecoveryState {
    private val states = setOf(
        "disabled", "idle", "checking", "clear", "recovered", "unconfirmed", "unavailable",
    )

    fun parse(value: String): String = value.takeIf { it in states } ?: "disabled"
}
