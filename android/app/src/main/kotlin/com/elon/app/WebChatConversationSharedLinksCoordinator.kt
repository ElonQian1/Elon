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
    private val conversationTitle: (String) -> String? = { null },
) {
    private var epoch = 0
    private var dialog: AlertDialog? = null
    private var accountWide = false
    private var canvasShares = false

    fun show(conversation: ChatGptWebConversation, allConversations: Boolean = false, canvas: Boolean = false) {
        cancel()
        accountWide = allConversations || canvas
        canvasShares = canvas
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

    private fun load(conversation: ChatGptWebConversation, offset: Int = 0, selection: String? = null) {
        if (!active()) return
        val path = ChatGptWebSharedLinks.path(conversation.path) ?: return failure(conversation, "share_invalid_selection")
        val port = consumerPort() ?: return failure(conversation, "share_list_unavailable")
        val result = if (canvasShares) port.manageCanvasShares(offset, selection)
            else if (accountWide) port.manageAccountShares(offset, selection) else port.manageConversationShares(path)
        track(AlertDialog.Builder(activity).setTitle(listTitle()).setMessage("正在读取")
            .setNegativeButton("关闭", null).create())
        await(conversation, result, epoch, 0) { detail ->
            if (accountWide) {
                val page = ChatGptWebSharedLinks.parseAccount(detail)?.takeIf {
                    it.offset == offset && (selection == null || it.ticket == selection) &&
                        (it.resource == ChatGptWebSharedLinks.Resource.CANVAS) == canvasShares
                } ?: return@await failure(conversation, "share_list_unconfirmed")
                showLinks(conversation, ChatGptWebSharedLinks.Index(path, page.ticket, page.complete, page.items), page)
                return@await
            }
            val index = ChatGptWebSharedLinks.parse(detail)?.takeIf { it.path == path }
                ?: return@await failure(conversation, "share_list_unconfirmed")
            showLinks(conversation, index)
        }
    }

    private fun showLinks(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index,
        page: ChatGptWebSharedLinks.AccountIndex? = null,
    ) {
        if (!active()) return
        val title = listTitle()
        val builder = AlertDialog.Builder(activity)
            .setTitle(title + if (index.complete) "" else "（部分）")
            .setNeutralButton("官网查看") { _, _ -> openOfficial(conversation) }
            .setNegativeButton("关闭", null)
        if (index.items.isEmpty()) {
            builder.setMessage(if (!index.complete) "官网返回的列表不完整，尚不能确认所有分享链接。"
                else if (canvasShares) "这个账号没有画布公开分享链接。"
                else if (page == null) "这个会话没有公开分享链接。" else "这个账号没有会话公开分享链接。")
        } else {
            val labels = index.items.mapIndexed { position, link ->
                val date = link.createdAt?.let { runCatching {
                    DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date.from(Instant.parse(it)))
                }.getOrNull() }
                val name = if (page == null) null else link.path?.let(conversationTitle)?.takeIf(String::isNotBlank)
                (name ?: "${if (canvasShares) "画布分享" else "分享链接"} ${(page?.offset ?: 0) + position + 1}") + (date?.let { " · $it" } ?: "")
            }.toTypedArray()
            builder.setItems(labels) { _, position -> showLink(conversation, index, index.items[position], page) }
        }
        page?.nextOffset?.let { next -> builder.setPositiveButton("下一页") { _, _ -> load(conversation, next, page.ticket) } }
        if (page != null && page.offset > 0) {
            builder.setNeutralButton("上一页") { _, _ -> load(conversation, page.offset - 100, page.ticket) }
        }
        track(builder.create())
        dialog?.listView?.contentDescription = if (canvasShares) "web-chat-canvas-share-links-list"
            else if (page == null) "web-chat-share-links-list" else "web-chat-account-share-links-list"
        if (page?.nextOffset != null) dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-account-shares-next"
        if (page != null && page.offset > 0) dialog?.getButton(AlertDialog.BUTTON_NEUTRAL)?.contentDescription = "web-chat-account-shares-previous"
    }

    private fun showLink(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
        page: ChatGptWebSharedLinks.AccountIndex? = null,
    ) {
        val builder = AlertDialog.Builder(activity).setTitle(if (canvasShares) "画布公开分享链接" else "公开分享链接").setMessage(link.url)
            .setPositiveButton("复制链接") { _, _ ->
                val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                clipboard?.setPrimaryClip(ClipData.newPlainText(if (canvasShares) "画布分享链接" else "会话分享链接", link.url))
                if (clipboard != null) Toast.makeText(activity, "链接已复制", Toast.LENGTH_SHORT).show()
            }
            .setNeutralButton("取消分享") { _, _ -> confirmRevoke(conversation, index, link, page) }
            .setNegativeButton("返回列表") { _, _ -> showLinks(conversation, index, page) }
        if (canvasShares) builder.setView(WebChatCanvasContentView.entry(activity,
            open = { readCanvas(conversation, index, link, page) },
            update = { confirmCanvasUpdate(conversation, index, link, page) }))
        track(builder.create())
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-existing-share-copy"
        dialog?.getButton(AlertDialog.BUTTON_NEUTRAL)?.contentDescription = "web-chat-share-revoke"
    }

    private fun readCanvas(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
        page: ChatGptWebSharedLinks.AccountIndex?,
    ) {
        if (!active() || link.resource != ChatGptWebSharedLinks.Resource.CANVAS) return
        val result = consumerPort()?.readCanvasShare(link.id, index.ticket)
            ?: return failure(conversation, "share_canvas_unavailable")
        track(AlertDialog.Builder(activity).setTitle("画布原文").setMessage("正在读取")
            .setNegativeButton("返回链接") { _, _ -> epoch += 1; showLink(conversation, index, link, page) }.create())
        await(conversation, result, epoch, 0) { detail ->
            if (detail != "share_canvas_ready") return@await failure(conversation, "share_canvas_unconfirmed")
            showCanvasContent(conversation, result, link) { showLink(conversation, index, link, page) }
        }
    }

    private fun showCanvasContent(
        conversation: ChatGptWebConversation, result: WebChatConsumerCommandResult,
        link: ChatGptWebSharedLinks.Link, backToList: Boolean = false, back: () -> Unit,
    ) {
        val content = consumerPort()?.canvasContent()?.takeIf { it.requestId == result.requestId && it.id == link.id }
            ?: return failure(conversation, "share_canvas_unconfirmed")
        track(WebChatCanvasContentView.dialog(activity, content, back))
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.apply {
            contentDescription = "web-chat-canvas-content-copy"
            setCompoundDrawablesWithIntrinsicBounds(R.drawable.ic_msg_copy, 0, 0, 0)
        }
        dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.contentDescription = "web-chat-canvas-content-back"
        if (backToList) dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.text = "返回列表"
    }

    private fun confirmCanvasUpdate(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index,
        link: ChatGptWebSharedLinks.Link, page: ChatGptWebSharedLinks.AccountIndex?,
    ) {
        if (!active() || link.resource != ChatGptWebSharedLinks.Resource.CANVAS) return
        track(AlertDialog.Builder(activity).setTitle("更新这条公开画布？")
            .setMessage("这条链接将展示原画布的最新内容，任何持有链接的人都能查看。原画布不会被修改。\n\n${link.url}")
            .setPositiveButton("更新公开内容") { _, _ -> updateCanvas(conversation, index, link) }
            .setNegativeButton("暂不更新") { _, _ -> showLink(conversation, index, link, page) }.create())
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-share-update-confirm"
        dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.contentDescription = "web-chat-canvas-share-update-cancel"
    }

    private fun updateCanvas(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
    ) {
        if (!active()) return
        val result = consumerPort()?.updateCanvasShare(link.id, index.ticket, userConfirmed = true)
            ?: return failure(conversation, "share_canvas_unavailable")
        track(AlertDialog.Builder(activity).setTitle("更新公开画布").setMessage("正在确认结果")
            .setNegativeButton("关闭", null).create())
        await(conversation, result, epoch, 0, timeoutCode = "share_canvas_update_unconfirmed") { detail ->
            if (detail != "share_canvas_updated") return@await failure(conversation, "share_canvas_update_unconfirmed")
            // Publishing consumed the selection ticket; returning reloads the list before another write.
            showCanvasContent(conversation, result, link, backToList = true) { load(conversation) }
        }
    }

    private fun confirmRevoke(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
        page: ChatGptWebSharedLinks.AccountIndex? = null,
    ) {
        track(AlertDialog.Builder(activity).setTitle("取消这条公开分享？")
            .setMessage("取消后，这条链接将无法查看。${if (canvasShares) "原画布" else "原会话"}不会删除，其他人已保存的副本不受影响。\n\n${link.url}")
            .setPositiveButton("取消分享") { _, _ -> revoke(conversation, index, link) }
            .setNegativeButton("保留链接") { _, _ -> showLink(conversation, index, link, page) }.create())
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-share-revoke-confirm"
    }

    private fun revoke(
        conversation: ChatGptWebConversation, index: ChatGptWebSharedLinks.Index, link: ChatGptWebSharedLinks.Link,
    ) {
        if (!active()) return
        val port = consumerPort()
        val result = (if (canvasShares) port?.manageCanvasShares(selectionTicket = index.ticket, shareId = link.id, userConfirmed = true)
            else port?.manageConversationShares(link.path ?: index.path, link.id, index.ticket, userConfirmed = true))
            ?: return failure(conversation, "share_list_unavailable")
        track(AlertDialog.Builder(activity).setTitle("取消分享").setMessage("正在确认结果")
            .setNegativeButton("关闭", null).create())
        await(conversation, result, epoch, 0, timeoutCode = "share_revoke_unconfirmed") { detail ->
            if (detail != "share_link_revoked") return@await failure(conversation, "share_revoke_unconfirmed")
            Toast.makeText(activity, "已取消这条分享", Toast.LENGTH_SHORT).show()
            load(conversation)
        }
    }

    private fun listTitle() = if (canvasShares) "画布公开分享链接" else if (accountWide) "全部公开分享链接" else "公开分享链接"

    private fun await(
        conversation: ChatGptWebConversation, result: WebChatConsumerCommandResult,
        token: Int, attempt: Int, timeoutCode: String = "share_list_unconfirmed", completed: (String?) -> Unit,
    ) {
        if (token != epoch || !active()) return
        if (!result.accepted || result.requestId.isNullOrBlank()) return failure(conversation, result.error)
        val state = consumerPort()?.state()
        if (state?.adapterCurrent != true) return failure(conversation, "share_context_changed")
        val receipt = state.commandRequests.firstOrNull { it.id == result.requestId }
        when (receipt?.status) {
            WebChatConsumerCommandStatus.SUCCEEDED -> { completed(receipt.detail); return }
            WebChatConsumerCommandStatus.FAILED -> return failure(conversation, receipt.detail)
            WebChatConsumerCommandStatus.TIMED_OUT -> return failure(conversation, timeoutCode)
            else -> Unit
        }
        if (attempt >= 80) return failure(conversation, timeoutCode)
        host.postDelayed({ await(conversation, result, token, attempt + 1, timeoutCode, completed) }, 250L)
    }

    private fun failure(conversation: ChatGptWebConversation, code: String?) {
        if (!active()) return
        val message = when (code) {
            "share_canvas_restricted" -> "这个画布当前受到访问限制，无法展示正文。"
            "share_canvas_scope_unconfirmed" -> "画布分享范围已变化，请重新读取。"
            "share_canvas_too_large" -> "画布较大，暂时无法完整显示。可以在官网查看完整内容。"
            "share_canvas_update_unconfirmed" -> "未能确认公开画布的更新结果。请重新读取画布或在官网核对，不会自动重复发布。"
            "share_canvas_version_unconfirmed" -> "尚未读到画布版本，未提交更新。请重新读取后再试。"
            "share_revoke_unconfirmed" -> "未能确认取消分享的结果。可以重新读取列表核对，不会自动重复取消。"
            "share_cooldown" -> "上一次分享操作尚未确认，请先重新读取并核对结果，稍后再操作。"
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
