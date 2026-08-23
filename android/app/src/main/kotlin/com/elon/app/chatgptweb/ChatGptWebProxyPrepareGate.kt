package com.elon.app.chatgptweb

internal class ChatGptWebProxyPrepareGate(
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val timeoutMs: Long,
    private val fallback: () -> ChatGptWebProxyStatus,
    private val onReady: (ChatGptWebProxyStatus) -> Unit,
) {
    private var completed = false
    private val timeout = Runnable { complete(fallback()) }

    fun start() = schedule(timeout, timeoutMs)

    fun complete(status: ChatGptWebProxyStatus) {
        if (completed) return
        completed = true
        cancel(timeout)
        onReady(status)
    }
}
