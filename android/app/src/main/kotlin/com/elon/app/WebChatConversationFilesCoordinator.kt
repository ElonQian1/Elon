package com.elon.app

import android.view.View
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation

internal object WebChatConversationFilesPresentation {
    fun rows(index: WebChatConversationFileIndex?, loading: Boolean, failed: Boolean): List<WebChatActionSheetItem> = buildList {
        val status = when {
            failed -> "读取失败，可重试"
            loading -> "正在更新"
            index == null -> "尚未读取"
            index.truncated -> "部分附件"
            index.files.isEmpty() -> "此会话暂无附件"
            else -> "${index.files.size} 个附件"
        }
        add(WebChatActionSheetItem("status", status, enabled = false,
            contentDescription = "web-chat-conversation-files-status"))
        index?.files?.forEachIndexed { position, file ->
            add(WebChatActionSheetItem("file-$position", file.name,
                subtitle = listOf(if (file.role == "user") "我" else "AI",
                    if (file.kind == "image") "图片" else "文件", file.mediaType)
                    .filter(String::isNotEmpty).joinToString(" · "),
                contentDescription = "web-chat-conversation-file-$position"))
        }
    }
}

internal class WebChatConversationFilesCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val openConversation: (ChatGptWebConversation) -> Unit,
    private val openOfficial: (ChatGptWebConversation) -> Unit,
) {
    private var epoch = 0
    private var sheet: WebChatActionSheetHandle? = null
    private var detail: AlertDialog? = null
    private var pollTask: Runnable? = null

    fun show(conversation: ChatGptWebConversation, force: Boolean = false) {
        cancel()
        val owner = consumerPort() ?: return
        val currentEpoch = epoch
        var index = owner.conversationFiles(conversation.path)
        val needsRefresh = force || index?.isFresh(System.currentTimeMillis()) != true
        var selected: WebChatConversationFile? = null
        sheet = WebChatActionSheet.showUpdatable(activity, "会话附件",
            WebChatConversationFilesPresentation.rows(index, needsRefresh, false),
            footerActions = listOf(
                WebChatActionSheetFooterAction("刷新", "web-chat-conversation-files-refresh") {
                    host.post { if (currentEpoch == epoch && consumerPort() === owner) show(conversation, force = true) }
                },
                WebChatActionSheetFooterAction("官网选项", "web-chat-conversation-files-official") {
                    host.post { if (currentEpoch == epoch && consumerPort() === owner) openOfficial(conversation) }
                },
            ),
            onDismissed = {
                if (currentEpoch == epoch) {
                    sheet = null
                    stopPolling()
                    val value = selected
                    if (value != null) host.post {
                        if (currentEpoch == epoch && consumerPort() === owner) showFile(value, conversation)
                    }
                }
            },
        ) { item ->
            val position = item.id.removePrefix("file-").toIntOrNull()
            selected = position?.let { index?.files?.getOrNull(it) }
        }
        if (sheet == null || !needsRefresh) return
        val request = owner.requestConversationFiles(conversation.path)
        if (!request.accepted || request.requestId == null) {
            sheet?.updateItems(WebChatConversationFilesPresentation.rows(index, false, true))
            return
        }
        val startedAt = android.os.SystemClock.elapsedRealtime()
        val task = object : Runnable {
            override fun run() {
                if (currentEpoch != epoch || sheet == null || consumerPort() !== owner) return
                val result = owner.conversationFiles(conversation.path)
                val status = owner.state().commandRequests.firstOrNull { it.id == request.requestId }?.status
                if (result?.requestId == request.requestId) {
                    index = result
                    sheet?.updateItems(WebChatConversationFilesPresentation.rows(index, false, false))
                    pollTask = null
                    return
                }
                if (status in setOf(WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT,
                        WebChatConsumerCommandStatus.SUCCEEDED) || android.os.SystemClock.elapsedRealtime() - startedAt >= 15_000) {
                    sheet?.updateItems(WebChatConversationFilesPresentation.rows(index, false, true))
                    pollTask = null
                    return
                }
                host.postDelayed(this, 250)
            }
        }
        pollTask = task
        host.post(task)
    }

    private fun showFile(file: WebChatConversationFile, conversation: ChatGptWebConversation) {
        if (activity.isFinishing || activity.isDestroyed) return
        detail = AlertDialog.Builder(activity).setTitle(file.name)
            .setMessage(listOf(if (file.role == "user") "来源：我" else "来源：AI",
                file.mediaType).filter(String::isNotEmpty).joinToString("\n"))
            .setPositiveButton("打开所在会话") { _, _ -> openConversation(conversation) }
            .setNegativeButton("关闭", null).show()
    }

    private fun stopPolling() {
        pollTask?.let(host::removeCallbacks)
        pollTask = null
    }

    fun cancel() {
        epoch += 1
        stopPolling()
        val current = sheet
        sheet = null
        current?.dismiss()
        detail?.dismiss()
        detail = null
    }
}
