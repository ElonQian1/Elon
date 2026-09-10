package com.elon.app

import android.os.SystemClock
import android.view.View

internal class WebChatLibraryAttachmentAction(
    private val host: View,
    private val currentPort: () -> WebChatConsumerPort?,
    private val status: (String) -> Unit,
    private val onAttached: () -> Unit,
) {
    private var poll: Runnable? = null
    val busy: Boolean get() = poll != null

    fun start(owner: WebChatConsumerPort, file: WebChatLibraryEntry) {
        if (busy || currentPort() !== owner) return
        val result = owner.attachLibraryFile(file.handle)
        if (!result.accepted || result.requestId == null) {
            status("附件暂未加入，请刷新文件库后重试")
            return
        }
        status("正在加入附件")
        val started = SystemClock.elapsedRealtime()
        val page = owner.state().pageUrl
        val task = object : Runnable {
            override fun run() {
                if (poll !== this) return
                val state = owner.state()
                if (currentPort() !== owner || state.pageUrl != page || !state.adapterCurrent) {
                    stopWatching()
                    status("会话已变化，请在当前会话重新选择附件")
                    return
                }
                val receipt = state.commandRequests.firstOrNull { it.id == result.requestId }
                val outcome = WebChatLibraryAttachmentReceiptPolicy.outcome(
                    succeeded = receipt?.status == WebChatConsumerCommandStatus.SUCCEEDED,
                    terminal = receipt?.status in setOf(WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT),
                    detail = receipt?.detail,
                    elapsedMs = SystemClock.elapsedRealtime() - started,
                )
                when (outcome) {
                    WebChatLibraryAttachmentReceiptPolicy.Outcome.ATTACHED -> {
                        stopWatching()
                        onAttached()
                    }
                    WebChatLibraryAttachmentReceiptPolicy.Outcome.UNCONFIRMED -> {
                        stopWatching()
                        status("附件结果未确认；请返回会话检查附件，勿重复加入")
                    }
                    WebChatLibraryAttachmentReceiptPolicy.Outcome.WAIT -> host.postDelayed(this, 300)
                }
            }
        }
        poll = task
        host.post(task)
    }

    // Closing the browser only stops observing; it does not claim to cancel an accepted attachment.
    fun stopWatching() {
        poll?.let(host::removeCallbacks)
        poll = null
    }
}
