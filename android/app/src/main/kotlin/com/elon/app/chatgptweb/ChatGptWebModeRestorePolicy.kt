package com.elon.app.chatgptweb

internal object ChatGptWebModeRestorePolicy {
    data class Decision(
        val target: ChatGptWebModeController.Mode?,
        val consumePending: Boolean,
    )

    fun decide(
        pending: ChatGptWebModeController.Mode?,
        current: ChatGptWebModeController.Mode,
        nativeModeEnabled: Boolean,
    ): Decision = when {
        pending == null -> Decision(target = null, consumePending = false)
        current != ChatGptWebModeController.Mode.QUICK ->
            Decision(target = current, consumePending = true)
        pending == ChatGptWebModeController.Mode.NATIVE && !nativeModeEnabled ->
            Decision(target = null, consumePending = false)
        else -> Decision(target = pending, consumePending = true)
    }
}
