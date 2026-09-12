package com.elon.app

import android.os.SystemClock
import android.view.View
import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebWritingBlock
import org.json.JSONObject

internal class WebChatTextBlockCloudSession(
    private val host: View, private val port: () -> WebChatConsumerPort?, private val block: WebChatTextBlock,
) {
    private val owner = port()
    private val path = owner?.state()?.pageUrl?.let(ChatGptWebConversationPath::fromUrl)
    private var selection: ChatGptWebWritingBlock? = null
    private var task: Runnable? = null
    private var submitted: String? = null
    private var closed = false
    var changed: () -> Unit = {}
    var busy = false; private set
    var pending = false; private set
    var ready = false; private set
    var savedContent = block.content; private set
    var savedToCloud = false; private set
    var status = "官网尚未核对"; private set

    fun prepare() {
        if (pending) { verify(); return }
        val request = JSONObject().put("operation", "prepare").put("path", path)
            .put("messageId", block.sourceMessageId).put("id", block.id).put("content", savedContent)
        execute(request, false, "正在核对官网")
    }

    fun save(content: String) {
        if (!ready || busy || pending || content.length > WebChatTextBlock.MAX_CONTENT) return
        val selected = selection ?: return
        submitted = content
        execute(JSONObject().put("operation", "save").put("path", path).put("ticket", selected.ticket)
            .put("content", content), true, "正在保存官网")
    }

    fun verify() {
        val selected = selection ?: return
        execute(JSONObject().put("operation", "verify").put("path", path).put("ticket", selected.ticket), false, "正在核对保存结果")
    }

    private fun current() = !closed && owner != null && port() === owner && owner.state().adapterCurrent &&
        ChatGptWebConversationPath.fromUrl(owner.state().pageUrl) == path

    private fun execute(request: JSONObject, confirmed: Boolean, message: String) {
        if (busy) return
        if (!current()) { if (confirmed) submitted = null; ready = false; status = "会话已变化，修改仍保留"; changed(); return }
        val action = owner!!.writingBlock(request, confirmed)
        if (!action.accepted || action.requestId == null) {
            if (confirmed) submitted = null
            ready = false; status = failure(action.error); changed(); return
        }
        busy = true
        status = message
        if (confirmed) pending = true
        changed()
        val started = SystemClock.elapsedRealtime()
        val poll = object : Runnable {
            override fun run() {
                if (closed) return
                val receipt = owner.state().commandRequests.firstOrNull { it.id == action.requestId }
                val value = owner.writingBlock()?.takeIf { it.requestId == action.requestId &&
                    it.path == path && it.id == block.id && it.messageId == block.sourceMessageId }
                if (!current()) { finish("会话已变化，保存结果待核对", false); return }
                if (receipt?.status == WebChatConsumerCommandStatus.SUCCEEDED && value != null) {
                    selection = value
                    pending = value.pending
                    if (receipt.detail == "writing_saved" && !pending) {
                        val content = submitted
                        if (content != null) { savedContent = content; savedToCloud = true; submitted = null; finish("已保存到官网", true) }
                        else finish("官网已更新，请重新打开原文", false)
                    } else if (receipt.detail == "writing_ready" && !pending) finish("官网已核对", true)
                    else finish(if (receipt.detail == "writing_saved_sync_pending") "已写入官网，页面同步待核对" else "保存结果待核对", false)
                    return
                }
                if (receipt?.status == WebChatConsumerCommandStatus.FAILED) {
                    // An event with pending=true, not the generic UI timeout, authorizes verification.
                    if (confirmed && receipt.detail?.startsWith("writing_") == true &&
                        receipt.detail !in setOf("writing_unavailable", "writing_write_unconfirmed")) pending = false
                    finish(failure(receipt.detail), false); return
                }
                if (SystemClock.elapsedRealtime() - started > 30_000 || receipt?.status == WebChatConsumerCommandStatus.TIMED_OUT) {
                    finish(if (pending) "保存结果待核对，修改仍保留" else "核对超时，仍可编辑副本", false); return
                }
                host.postDelayed(this, 200)
            }
        }
        task = poll
        host.post(poll)
    }

    private fun finish(message: String, allowed: Boolean) {
        task = null; busy = false; ready = allowed; status = message; changed()
    }

    fun close() { closed = true; task?.let(host::removeCallbacks); task = null; changed = {} }

    private fun failure(code: String?) = when (code) {
        "writing_version_conflict", "writing_web_edit_pending" -> "官网内容已变化，修改仍保留"
        "writing_selection_unavailable", "writing_scope_unconfirmed", "writing_runtime_unavailable" -> "此写作块暂未确认可写回，仍可导出副本"
        "writing_http_401", "writing_http_403", "login_required" -> "官网身份或权限需确认，修改仍保留"
        "writing_context_changed", "writing_selection_expired" -> "会话或身份已变化，修改仍保留"
        else -> "官网核对未完成，修改仍保留"
    }
}
