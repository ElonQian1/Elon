package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal enum class WebChatConversationMutationProgress {
    WAITING,
    SUCCEEDED,
    NEEDS_OFFICIAL_CONFIRMATION,
}

internal sealed interface WebChatConversationMutationIntent {
    data class Pinned(val value: Boolean) : WebChatConversationMutationIntent
    data class Archived(val value: Boolean) : WebChatConversationMutationIntent
    data class Renamed(val title: String) : WebChatConversationMutationIntent
}

internal object WebChatConversationMutationPolicy {
    const val MAX_TITLE_LENGTH = 160

    fun desiredPinned(conversation: ChatGptWebConversation): Boolean = conversation.pinned != true

    fun pinnedActionTitle(conversation: ChatGptWebConversation): String =
        if (desiredPinned(conversation)) "置顶" else "取消置顶"

    fun normalizedTitle(value: String): String? = value
        .replace(Regex("\\s+"), " ")
        .trim()
        .takeIf { it.isNotBlank() && it.length <= MAX_TITLE_LENGTH }

    fun progress(status: WebChatConsumerCommandStatus?): WebChatConversationMutationProgress =
        when (status) {
            WebChatConsumerCommandStatus.SUCCEEDED -> WebChatConversationMutationProgress.SUCCEEDED
            WebChatConsumerCommandStatus.FAILED,
            WebChatConsumerCommandStatus.TIMED_OUT,
            -> WebChatConversationMutationProgress.NEEDS_OFFICIAL_CONFIRMATION
            WebChatConsumerCommandStatus.PENDING,
            WebChatConsumerCommandStatus.UNKNOWN,
            null,
            -> WebChatConversationMutationProgress.WAITING
        }

    fun progressTitle(intent: WebChatConversationMutationIntent): String = when (intent) {
        is WebChatConversationMutationIntent.Pinned ->
            if (intent.value) "正在置顶" else "正在取消置顶"
        is WebChatConversationMutationIntent.Archived ->
            if (intent.value) "正在归档" else "正在恢复会话"
        is WebChatConversationMutationIntent.Renamed -> "正在重命名"
    }

    fun completedMessage(intent: WebChatConversationMutationIntent): String = when (intent) {
        is WebChatConversationMutationIntent.Pinned -> if (intent.value) "已置顶" else "已取消置顶"
        is WebChatConversationMutationIntent.Archived -> if (intent.value) "已归档" else "已恢复会话"
        is WebChatConversationMutationIntent.Renamed -> "已重命名"
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

internal class WebChatConversationMutationCoordinator(
    private val activity: AppCompatActivity,
    private val host: android.view.View,
    private val activeProvider: () -> WebChatProviderId?,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val refreshConversationIndex: (String?) -> Boolean,
    private val openOfficialFallback: (ChatGptWebConversation) -> Unit,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null

    fun start(
        conversation: ChatGptWebConversation,
        intent: WebChatConversationMutationIntent,
    ) {
        cancelPending()
        val path = ChatGptWebConversationPath.normalize(conversation.path)
            ?: return showFailure(conversation, intent, null)
        val port = consumerPort()
            ?.takeIf { activeProvider() == WebChatProviderId.CHATGPT_WEB }
            ?: return showFailure(conversation, intent, "mutation_unavailable")
        val epoch = requestEpoch
        showProgress(conversation, intent)
        val command = when (intent) {
            is WebChatConversationMutationIntent.Pinned ->
                port.setConversationPinned(path, intent.value, userConfirmed = true)
            is WebChatConversationMutationIntent.Archived ->
                port.setConversationArchived(path, intent.value, userConfirmed = true)
            is WebChatConversationMutationIntent.Renamed ->
                port.renameConversation(path, intent.title, userConfirmed = true)
        }
        val requestId = command.requestId
        if (!command.accepted || requestId.isNullOrBlank()) {
            refreshConversationIndex(null)
            showFailure(conversation, intent, command.error)
            return
        }
        poll(conversation, intent, port, requestId, epoch, attempt = 0)
    }

    fun cancelPending() {
        requestEpoch += 1
        val sheet = activeSheet
        activeSheet = null
        sheet?.dismiss()
    }

    private fun poll(
        conversation: ChatGptWebConversation,
        intent: WebChatConversationMutationIntent,
        port: WebChatConsumerPort,
        requestId: String,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch) return
        val request = port.state().commandRequests.lastOrNull { it.id == requestId }
        when (WebChatConversationMutationPolicy.progress(request?.status)) {
            WebChatConversationMutationProgress.SUCCEEDED -> {
                dismissProgress()
                Toast.makeText(
                    activity,
                    WebChatConversationMutationPolicy.completedMessage(intent),
                    Toast.LENGTH_SHORT,
                ).show()
            }
            WebChatConversationMutationProgress.NEEDS_OFFICIAL_CONFIRMATION -> {
                refreshConversationIndex(null)
                showFailure(conversation, intent, request?.detail)
            }
            WebChatConversationMutationProgress.WAITING -> {
                if (attempt >= MAX_POLL_ATTEMPTS) {
                    refreshConversationIndex(null)
                    showFailure(conversation, intent, "mutation_timeout")
                    return
                }
                host.postDelayed(
                    { poll(conversation, intent, port, requestId, epoch, attempt + 1) },
                    POLL_INTERVAL_MS,
                )
            }
        }
    }

    private fun showProgress(
        conversation: ChatGptWebConversation,
        intent: WebChatConversationMutationIntent,
    ) {
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = WebChatConversationMutationPolicy.progressTitle(intent),
            items = listOf(WebChatActionSheetItem(
                id = "conversation-mutation-progress",
                title = conversation.title,
                subtitle = "正在等待官网确认",
                enabled = false,
                contentDescription = "web-chat-conversation-mutation-progress",
            )),
            footerActions = listOf(WebChatActionSheetFooterAction(
                label = "官网确认",
                contentDescription = "web-chat-conversation-mutation-official",
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

    private fun showFailure(
        conversation: ChatGptWebConversation,
        intent: WebChatConversationMutationIntent,
        detail: String?,
    ) {
        dismissProgress()
        if (activity.isFinishing || activity.isDestroyed) return
        AlertDialog.Builder(activity)
            .setTitle("会话操作未确认")
            .setMessage(WebChatConversationMutationPolicy.failureMessage(detail))
            .setNeutralButton("重试") { _, _ -> start(conversation, intent) }
            .setPositiveButton("官网确认") { _, _ -> openOfficialFallback(conversation) }
            .setNegativeButton("取消", null)
            .show()
    }

    private companion object {
        const val POLL_INTERVAL_MS = 250L
        const val MAX_POLL_ATTEMPTS = 140
    }
}
