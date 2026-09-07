package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebSharedLinks
import java.text.DateFormat
import java.time.Instant
import java.util.Date

internal class WebChatConversationSharedLinksCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val activeProvider: () -> WebChatProviderId?,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val openOfficial: (ChatGptWebConversation) -> Unit,
) {
    private var epoch = 0
    private var dialog: AlertDialog? = null

    fun show(conversation: ChatGptWebConversation) {
        cancel()
        load(conversation)
    }

    fun cancel() {
        epoch += 1
        val previous = dialog
        dialog = null
        previous?.dismiss()
    }

    private fun active() = activeProvider() == WebChatProviderId.CHATGPT_WEB &&
        !activity.isFinishing && !activity.isDestroyed

    private fun load(conversation: ChatGptWebConversation) {
        if (!active()) return
        val path = ChatGptWebSharedLinks.path(conversation.path) ?: return failure(conversation, "share_invalid_selection")
        val result = consumerPort()?.manageConversationShares(path)
            ?: return failure(conversation, "share_list_unavailable")
        track(AlertDialog.Builder(activity).setTitle("公开分享链接").setMessage("正在读取")
            .setNegativeButton("关闭", null).create())
        await(conversation, result, epoch, 0, revoking = false) { detail ->
            val index = ChatGptWebSharedLinks.parse(detail)?.takeIf { it.path == path }
                ?: return@await failure(conversation, "share_list_unconfirmed")
            showLinks(conversation, index)
        }
    }

    private fun showLinks(conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index) {
        if (!active()) return
        val builder = AlertDialog.Builder(activity)
            .setTitle(if (index.complete) "公开分享链接" else "公开分享链接（部分）")
            .setNeutralButton("官网查看") { _, _ -> openOfficial(conversation) }
            .setNegativeButton("关闭", null)
        if (index.items.isEmpty()) {
            builder.setMessage(if (index.complete) "这个会话没有公开分享链接。"
                else "官网返回的部分列表中没有找到本会话链接，尚不能确认完整结果。")
        } else {
            val labels = index.items.mapIndexed { position, link ->
                val date = link.createdAt?.let { runCatching {
                    DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date.from(Instant.parse(it)))
                }.getOrNull() }
                "分享链接 ${position + 1}" + (date?.let { " · $it" } ?: "")
            }.toTypedArray()
            builder.setItems(labels) { _, position -> showLink(conversation, index, index.items[position]) }
        }
        track(builder.create())
        dialog?.listView?.contentDescription = "web-chat-share-links-list"
    }

    private fun showLink(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
    ) {
        track(AlertDialog.Builder(activity).setTitle("公开分享链接").setMessage(link.url)
            .setPositiveButton("复制链接") { _, _ ->
                val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                clipboard?.setPrimaryClip(ClipData.newPlainText("会话分享链接", link.url))
                if (clipboard != null) Toast.makeText(activity, "链接已复制", Toast.LENGTH_SHORT).show()
            }
            .setNeutralButton("取消分享") { _, _ -> confirmRevoke(conversation, index, link) }
            .setNegativeButton("返回列表") { _, _ -> showLinks(conversation, index) }.create())
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-existing-share-copy"
        dialog?.getButton(AlertDialog.BUTTON_NEUTRAL)?.contentDescription = "web-chat-share-revoke"
    }

    private fun confirmRevoke(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
    ) {
        track(AlertDialog.Builder(activity).setTitle("取消这条公开分享？")
            .setMessage("取消后，这条链接将无法查看。原会话不会删除，其他人已保存的副本不受影响。\n\n${link.url}")
            .setPositiveButton("取消分享") { _, _ -> revoke(conversation, index, link) }
            .setNegativeButton("保留链接") { _, _ -> showLink(conversation, index, link) }.create())
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-share-revoke-confirm"
    }

    private fun revoke(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
    ) {
        if (!active()) return
        val result = consumerPort()?.manageConversationShares(index.path, link.id, index.ticket, userConfirmed = true)
            ?: return failure(conversation, "share_list_unavailable")
        track(AlertDialog.Builder(activity).setTitle("取消分享").setMessage("正在确认结果")
            .setNegativeButton("关闭", null).create())
        await(conversation, result, epoch, 0, revoking = true) { detail ->
            if (detail != "share_link_revoked") return@await failure(conversation, "share_revoke_unconfirmed")
            Toast.makeText(activity, "已取消这条分享", Toast.LENGTH_SHORT).show()
            load(conversation)
        }
    }

    private fun await(
        conversation: ChatGptWebConversation, result: WebChatConsumerCommandResult,
        token: Int, attempt: Int, revoking: Boolean, completed: (String?) -> Unit,
    ) {
        if (token != epoch || !active()) return
        if (!result.accepted || result.requestId.isNullOrBlank()) return failure(conversation, result.error)
        val state = consumerPort()?.state()
        if (state?.adapterCurrent != true) return failure(conversation, "share_context_changed")
        val receipt = state.commandRequests.firstOrNull { it.id == result.requestId }
        when (receipt?.status) {
            WebChatConsumerCommandStatus.SUCCEEDED -> { completed(receipt.detail); return }
            WebChatConsumerCommandStatus.FAILED -> return failure(conversation, receipt.detail)
            WebChatConsumerCommandStatus.TIMED_OUT -> return failure(conversation,
                if (revoking) "share_revoke_unconfirmed" else "share_list_unconfirmed")
            else -> Unit
        }
        if (attempt >= 80) return failure(conversation,
            if (revoking) "share_revoke_unconfirmed" else "share_list_unconfirmed")
        host.postDelayed({ await(conversation, result, token, attempt + 1, revoking, completed) }, 250L)
    }

    private fun failure(conversation: ChatGptWebConversation, code: String?) {
        if (!active()) return
        val message = when (code) {
            "share_revoke_unconfirmed", "share_cooldown" -> "未能确认取消分享的结果。可以重新读取列表核对，不会自动重复取消。"
            "share_selection_expired", "share_context_changed" -> "账号或链接列表已经变化，请重新读取后再选择。"
            else -> "暂时未能读取或确认分享链接，请重试或在官网查看。"
        }
        track(AlertDialog.Builder(activity).setTitle("分享链接尚未确认").setMessage(message)
            .setPositiveButton("重新读取") { _, _ -> load(conversation) }
            .setNeutralButton("官网查看") { _, _ -> openOfficial(conversation) }
            .setNegativeButton("关闭", null).create())
    }

    private fun track(next: AlertDialog) {
        val previous = dialog
        dialog = next
        previous?.dismiss()
        next.setOnDismissListener { if (dialog === next) { dialog = null; epoch += 1 } }
        next.show()
    }
}
