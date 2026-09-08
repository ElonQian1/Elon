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
                when {
                    receipt?.status == WebChatConsumerCommandStatus.SUCCEEDED && receipt.detail == "library_attachment_associated" -> {
                        stopWatching()
                        onAttached()
                    }
                    receipt?.status in setOf(WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT) ||
                        SystemClock.elapsedRealtime() - started >= 16_000 -> {
                        stopWatching()
                        status("附件未能确认加入；请保留当前草稿，检查附件后重试")
                    }
                    else -> host.postDelayed(this, 300)
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
