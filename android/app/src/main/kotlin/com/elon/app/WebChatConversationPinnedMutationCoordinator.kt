package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal enum class WebChatConversationPinnedMutationProgress {
    WAITING,
    SUCCEEDED,
    NEEDS_OFFICIAL_CONFIRMATION,
}

internal object WebChatConversationPinnedMutationPolicy {
    fun desiredPinned(conversation: ChatGptWebConversation): Boolean = conversation.pinned != true

    fun actionTitle(conversation: ChatGptWebConversation): String =
        if (desiredPinned(conversation)) "置顶" else "取消置顶"

    fun progressTitle(pinned: Boolean): String = if (pinned) "正在置顶" else "正在取消置顶"

    fun completedMessage(pinned: Boolean): String = if (pinned) "已置顶" else "已取消置顶"

    fun progress(status: WebChatConsumerCommandStatus?): WebChatConversationPinnedMutationProgress =
        when (status) {
            WebChatConsumerCommandStatus.SUCCEEDED ->
                WebChatConversationPinnedMutationProgress.SUCCEEDED
            WebChatConsumerCommandStatus.FAILED,
            WebChatConsumerCommandStatus.TIMED_OUT,
            -> WebChatConversationPinnedMutationProgress.NEEDS_OFFICIAL_CONFIRMATION
            WebChatConsumerCommandStatus.PENDING,
            WebChatConsumerCommandStatus.UNKNOWN,
            null,
            -> WebChatConversationPinnedMutationProgress.WAITING
        }

    fun failureMessage(detail: String?): String = when {
        detail == "mutation_auth_unavailable" -> "网页身份正在恢复，官网尚未确认这次操作。"
        detail == "mutation_busy" -> "另一项会话操作仍在进行，官网尚未确认这次操作。"
        detail == "mutation_circuit_open" -> "直接通道正在短暂恢复，官网尚未确认这次操作。"
        detail == "mutation_timeout" -> "网络响应超时，官网尚未确认这次操作。"
        detail == "mutation_network_failure" -> "网络暂时不可用，官网尚未确认这次操作。"
        detail?.startsWith("mutation_http_") == true -> "官网没有接受这次操作，本地状态未改变。"
        else -> "官网尚未确认这次操作，本地状态未改变。"
    }
}

internal class WebChatConversationPinnedMutationCoordinator(
    private val activity: AppCompatActivity,
    private val host: android.view.View,
    private val activeProvider: () -> WebChatProviderId?,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val refreshConversationIndex: (String?) -> Boolean,
    private val openOfficialFallback: (ChatGptWebConversation) -> Unit,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null

    fun start(conversation: ChatGptWebConversation) {
        cancelPending()
        val path = ChatGptWebConversationPath.normalize(conversation.path)
            ?: return showFailure(conversation, null)
        val port = consumerPort()
            ?.takeIf { activeProvider() == WebChatProviderId.CHATGPT_WEB }
            ?: return showFailure(conversation, "mutation_unavailable")
        val pinned = WebChatConversationPinnedMutationPolicy.desiredPinned(conversation)
        val epoch = requestEpoch
        showProgress(conversation, pinned)
        val command = port.setConversationPinned(path, pinned, userConfirmed = true)
        val requestId = command.requestId
        if (!command.accepted || requestId.isNullOrBlank()) {
            refreshConversationIndex(null)
            showFailure(conversation, command.error)
            return
        }
        poll(conversation, pinned, port, requestId, epoch, attempt = 0)
    }

    fun cancelPending() {
        requestEpoch += 1
        val sheet = activeSheet
        activeSheet = null
        sheet?.dismiss()
    }

    private fun poll(
        conversation: ChatGptWebConversation,
        pinned: Boolean,
        port: WebChatConsumerPort,
        requestId: String,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch) return
        val request = port.state().commandRequests.lastOrNull { it.id == requestId }
        when (WebChatConversationPinnedMutationPolicy.progress(request?.status)) {
            WebChatConversationPinnedMutationProgress.SUCCEEDED -> {
                dismissProgress()
                Toast.makeText(
                    activity,
                    WebChatConversationPinnedMutationPolicy.completedMessage(pinned),
                    Toast.LENGTH_SHORT,
                ).show()
            }
            WebChatConversationPinnedMutationProgress.NEEDS_OFFICIAL_CONFIRMATION -> {
                refreshConversationIndex(null)
                showFailure(conversation, request?.detail)
            }
            WebChatConversationPinnedMutationProgress.WAITING -> {
                if (attempt >= MAX_POLL_ATTEMPTS) {
                    refreshConversationIndex(null)
                    showFailure(conversation, "mutation_timeout")
                    return
                }
                host.postDelayed(
                    { poll(conversation, pinned, port, requestId, epoch, attempt + 1) },
                    POLL_INTERVAL_MS,
                )
            }
        }
    }

    private fun showProgress(conversation: ChatGptWebConversation, pinned: Boolean) {
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = WebChatConversationPinnedMutationPolicy.progressTitle(pinned),
            items = listOf(WebChatActionSheetItem(
                id = "conversation-pinned-progress",
                title = conversation.title,
                subtitle = "正在等待官网确认",
                enabled = false,
                contentDescription = "web-chat-conversation-pinned-progress",
            )),
            footerActions = listOf(WebChatActionSheetFooterAction(
                label = "官网确认",
                contentDescription = "web-chat-conversation-pinned-official",
                action = {
                    requestEpoch += 1
                    openOfficialFallback(conversation)
                },
            )),
            onCancelled = { requestEpoch += 1 },
            onDismissed = { activeSheet = null },
        ) {}
    }

    private fun dismissProgress() {
        val sheet = activeSheet
        activeSheet = null
        sheet?.dismiss()
    }

    private fun showFailure(conversation: ChatGptWebConversation, detail: String?) {
        dismissProgress()
        if (activity.isFinishing || activity.isDestroyed) return
        AlertDialog.Builder(activity)
            .setTitle("会话操作未确认")
            .setMessage(WebChatConversationPinnedMutationPolicy.failureMessage(detail))
            .setNeutralButton("重试") { _, _ -> start(conversation) }
            .setPositiveButton("官网确认") { _, _ -> openOfficialFallback(conversation) }
            .setNegativeButton("取消", null)
            .show()
    }

    private companion object {
        const val POLL_INTERVAL_MS = 250L
        const val MAX_POLL_ATTEMPTS = 140
    }
}
