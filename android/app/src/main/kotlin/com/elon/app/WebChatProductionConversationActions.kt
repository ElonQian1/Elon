package com.elon.app

import android.view.View
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal enum class WebChatConversationActionReadiness {
    SHOW,
    WAIT,
    CANCEL,
}

internal object WebChatProductionConversationActionPolicy {
    fun evaluate(
        providerId: WebChatProviderId?,
        targetPath: String,
        currentPath: String?,
        state: String,
    ): WebChatConversationActionReadiness = when {
        providerId != WebChatProviderId.CHATGPT_WEB -> WebChatConversationActionReadiness.CANCEL
        currentPath == targetPath && state == "ready" -> WebChatConversationActionReadiness.SHOW
        else -> WebChatConversationActionReadiness.WAIT
    }
}

internal class WebChatProductionConversationActionsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val activeProvider: () -> WebChatProviderId?,
    private val currentConversationPath: () -> String?,
    private val currentState: () -> String,
    private val openConversation: (String) -> Boolean,
    private val showPageActions: () -> Unit,
    private val openOfficialFallback: () -> Unit,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null

    fun show(conversation: ChatGptWebConversation) {
        cancelPending()
        val targetPath = ChatGptWebConversationPath.normalize(conversation.path)
            ?: return showRecovery(conversation)
        val epoch = requestEpoch
        when (readiness(targetPath)) {
            WebChatConversationActionReadiness.SHOW -> showPageActions()
            WebChatConversationActionReadiness.CANCEL -> Unit
            WebChatConversationActionReadiness.WAIT -> {
                if (!openConversation(targetPath)) return showRecovery(conversation)
                showTransition(conversation)
                poll(conversation, targetPath, epoch, attempt = 0)
            }
        }
    }

    fun cancelPending() {
        requestEpoch += 1
        val sheet = activeSheet
        activeSheet = null
        sheet?.dismiss()
    }

    private fun poll(
        conversation: ChatGptWebConversation,
        targetPath: String,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch) return
        when (readiness(targetPath)) {
            WebChatConversationActionReadiness.SHOW -> {
                dismissTransition()
                showPageActions()
            }
            WebChatConversationActionReadiness.CANCEL -> Unit
            WebChatConversationActionReadiness.WAIT -> {
                if (attempt >= MAX_POLL_ATTEMPTS) return showRecovery(conversation)
                host.postDelayed(
                    { poll(conversation, targetPath, epoch, attempt + 1) },
                    POLL_INTERVAL_MS,
                )
            }
        }
    }

    private fun readiness(targetPath: String): WebChatConversationActionReadiness =
        WebChatProductionConversationActionPolicy.evaluate(
            providerId = activeProvider(),
            targetPath = targetPath,
            currentPath = currentConversationPath(),
            state = currentState(),
        )

    private fun showTransition(conversation: ChatGptWebConversation) {
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "会话操作",
            items = listOf(WebChatActionSheetItem(
                id = "conversation-transition",
                title = conversation.title,
                subtitle = "正在切换到该会话",
                enabled = false,
                contentDescription = "web-chat-conversation-actions-transition",
            )),
            footerActions = listOf(WebChatActionSheetFooterAction(
                label = "官网完成",
                contentDescription = "web-chat-conversation-actions-official",
                action = openOfficialFallback,
            )),
            onCancelled = {
                if (activeSheet != null) requestEpoch += 1
            },
            onDismissed = { activeSheet = null },
        ) {}
    }

    private fun dismissTransition() {
        val sheet = activeSheet
        activeSheet = null
        sheet?.dismiss()
    }

    private fun showRecovery(conversation: ChatGptWebConversation) {
        if (activity.isFinishing || activity.isDestroyed) return
        dismissTransition()
        AlertDialog.Builder(activity)
            .setTitle("会话操作暂不可用")
            .setMessage("“${conversation.title}”已经保留在会话列表中。可以重试，或在官网中完成管理操作。")
            .setNeutralButton("重试") { _, _ -> show(conversation) }
            .setPositiveButton("官网完成") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .show()
    }

    private companion object {
        const val POLL_INTERVAL_MS = 250L
        const val MAX_POLL_ATTEMPTS = 24
    }
}
