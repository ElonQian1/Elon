package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal class WebChatConversationShareCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val activeProvider: () -> WebChatProviderId?,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val openConversation: (String) -> WebChatConsumerCommandResult,
    private val openOfficial: (ChatGptWebConversation) -> Unit,
) {
    private var epoch = 0
    private var dialog: AlertDialog? = null

    fun show(conversation: ChatGptWebConversation) {
        cancel()
        val path = ChatGptWebConversationPath.normalize(conversation.path) ?: return
        val state = consumerPort()?.state() ?: return
        if (!active()) return
        if (!conversation.projectId.isNullOrBlank() || path.startsWith("/g/")) {
            failure(conversation, "share_project_scope_unconfirmed")
            return
        }
        val current = WebChatConversationSharePolicy.sameConversation(path, state.pageUrl)
        if (!current && (state.draftPresent || state.streaming || state.dictationActive ||
                state.dictationCaptureActive || state.dictationCapturePending)) {
            track(WebChatConversationDraftNavigation.dialog(activity))
            return
        }
        if (!current && !openConversation(path).accepted) return failure(conversation, "share_context_unavailable")
        if (current && state.adapterCurrent) confirm(conversation, path)
        else {
            progress("正在打开要分享的会话")
            awaitConversation(conversation, path, epoch, 0)
        }
    }

    fun cancel() {
        epoch += 1
        dismiss()
    }

    private fun active(): Boolean = activeProvider() == WebChatProviderId.CHATGPT_WEB &&
        !activity.isFinishing && !activity.isDestroyed

    private fun awaitConversation(conversation: ChatGptWebConversation, path: String, token: Int, attempt: Int) {
        if (token != epoch || !active()) return
        val state = consumerPort()?.state()
        if (state?.adapterCurrent == true && WebChatConversationSharePolicy.sameConversation(path, state.pageUrl)) {
            confirm(conversation, path)
            return
        }
        if (attempt >= 60) return failure(conversation, "share_context_unavailable")
        host.postDelayed({ awaitConversation(conversation, path, token, attempt + 1) }, 250L)
    }

    private fun confirm(conversation: ChatGptWebConversation, path: String) {
        if (!active()) return
        track(AlertDialog.Builder(activity)
            .setTitle("创建公开分享链接")
            .setMessage("分享“${conversation.title}”当前分支已有的对话内容。任何拿到链接的人都可查看，请确认不含不宜公开的信息。")
            .setPositiveButton("创建链接") { _, _ -> start(conversation, path) }
            .setNegativeButton("取消", null)
            .create())
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-share-confirm"
    }

    private fun start(conversation: ChatGptWebConversation, path: String) {
        if (!active()) return
        val port = consumerPort() ?: return
        if (!WebChatConversationSharePolicy.sameConversation(path, port.state().pageUrl)) {
            failure(conversation, "share_context_changed")
            return
        }
        val result = port.shareConversation(path, userConfirmed = true)
        val requestId = result.requestId
        if (!result.accepted || requestId.isNullOrBlank()) return failure(conversation, result.error)
        progress("正在创建分享链接")
        awaitResult(conversation, path, requestId, epoch, 0)
    }

    private fun awaitResult(conversation: ChatGptWebConversation, path: String, requestId: String, token: Int, attempt: Int) {
        if (token != epoch || !active()) return
        val state = consumerPort()?.state()
        if (state == null || !state.adapterCurrent || !WebChatConversationSharePolicy.sameConversation(path, state.pageUrl)) {
            return failure(conversation, "share_result_unconfirmed")
        }
        val receipt = state.commandRequests.firstOrNull { it.id == requestId }
        when (receipt?.status) {
            WebChatConsumerCommandStatus.SUCCEEDED -> {
                val url = WebChatConversationSharePolicy.resultUrl(receipt.detail)
                    ?: return failure(conversation, "share_result_unconfirmed")
                showResult(url)
                return
            }
            WebChatConsumerCommandStatus.FAILED -> return failure(conversation, receipt.detail)
            WebChatConsumerCommandStatus.TIMED_OUT -> return failure(conversation, "share_result_unconfirmed")
            else -> Unit
        }
        if (attempt >= 144) return failure(conversation, "share_result_unconfirmed")
        host.postDelayed({ awaitResult(conversation, path, requestId, token, attempt + 1) }, 250L)
    }

    private fun showResult(url: String) {
        track(AlertDialog.Builder(activity)
            .setTitle("分享链接已创建")
            .setMessage(url)
            .setPositiveButton("复制链接") { _, _ ->
                val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                clipboard?.setPrimaryClip(ClipData.newPlainText("会话分享链接", url))
                if (clipboard != null) Toast.makeText(activity, "链接已复制", Toast.LENGTH_SHORT).show()
            }
            .setNeutralButton("分享") { _, _ ->
                val intent = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, url)
                runCatching { activity.startActivity(Intent.createChooser(intent, "分享会话链接")) }
                    .onFailure { Toast.makeText(activity, "暂时无法打开分享应用", Toast.LENGTH_SHORT).show() }
            }
            .setNegativeButton("关闭", null)
            .create())
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-share-copy"
        dialog?.getButton(AlertDialog.BUTTON_NEUTRAL)?.contentDescription = "web-chat-share-distribute"
    }

    private fun failure(conversation: ChatGptWebConversation, code: String?) {
        if (!active()) return
        track(AlertDialog.Builder(activity)
            .setTitle("分享未完成")
            .setMessage(WebChatConversationSharePolicy.errorMessage(code))
            .setPositiveButton("官网查看") { _, _ -> openOfficial(conversation) }
            .setNegativeButton("关闭", null)
            .create())
    }

    private fun progress(message: String) {
        val next = AlertDialog.Builder(activity).setTitle("分享会话").setMessage(message)
            .setNegativeButton("关闭", null).create()
        next.setOnCancelListener { cancel() }
        next.setOnDismissListener { if (dialog === next) { dialog = null; epoch += 1 } }
        track(next)
    }

    private fun track(next: AlertDialog) {
        dismiss()
        dialog = next
        next.show()
    }

    private fun dismiss() {
        val previous = dialog
        dialog = null
        previous?.dismiss()
    }
}
