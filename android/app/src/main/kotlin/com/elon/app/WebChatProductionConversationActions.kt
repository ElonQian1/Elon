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
        sameConversation(targetPath, currentPath) && state == "ready" ->
            WebChatConversationActionReadiness.SHOW
        else -> WebChatConversationActionReadiness.WAIT
    }

    private fun sameConversation(targetPath: String, currentPath: String?): Boolean {
        val targetIdentity = ChatGptWebConversationPath.identity(targetPath) ?: return false
        return targetIdentity == ChatGptWebConversationPath.identity(currentPath)
    }
}

internal class WebChatProductionConversationActionsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val activeProvider: () -> WebChatProviderId?,
    private val currentConversationPath: () -> String?,
    private val currentState: () -> String,
    private val openConversationTracked: (String) -> WebChatConsumerCommandResult,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val conversationIndex: () -> com.elon.app.chatgptweb.ChatGptWebConversationIndexState,
    private val refreshConversationIndex: (String?) -> Boolean,
    private val probeConversationProject: (String, String) -> Boolean,
    private val suspendConversationRefresh: () -> Unit,
    private val resumeConversationRefresh: () -> Unit,
    private val showPageActions: () -> Unit,
    private val openOfficialFallback: () -> Unit,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null
    private val pinnedMutation = WebChatConversationPinnedMutationCoordinator(
        activity = activity,
        host = host,
        activeProvider = activeProvider,
        consumerPort = consumerPort,
        refreshConversationIndex = refreshConversationIndex,
        openOfficialFallback = ::showPageActionsFor,
    )
    private val projectMove = WebChatProductionConversationProjectMoveCoordinator(
        activity = activity,
        host = host,
        activeProvider = activeProvider,
        consumerPort = consumerPort,
        currentConversationPath = currentConversationPath,
        currentState = currentState,
        openConversation = openConversationTracked,
        conversationIndex = conversationIndex,
        refreshConversationIndex = refreshConversationIndex,
        probeConversationProject = probeConversationProject,
        suspendConversationRefresh = suspendConversationRefresh,
        resumeConversationRefresh = resumeConversationRefresh,
        openOfficialFallback = openOfficialFallback,
    )

    fun show(conversation: ChatGptWebConversation) {
        cancelPending()
        if (projectMove.recoverPending(interactive = true)) return
        val epoch = requestEpoch
        var selectedActionId: String? = null
        val canMove = WebChatConversationProjectMovePolicy.destinations(
            conversationIndex(),
            conversation,
        ).isNotEmpty()
        val items = buildList {
            add(WebChatActionSheetItem(
                id = ACTION_SET_PINNED,
                title = WebChatConversationPinnedMutationPolicy.actionTitle(conversation),
                subtitle = "后台确认官网结果，不切换会话",
                contentDescription = "web-chat-conversation-action-set-pinned",
            ))
            if (canMove) add(WebChatActionSheetItem(
                id = ACTION_MOVE_TO_PROJECT,
                title = "移动到项目",
                subtitle = "从已缓存的项目中选择",
                contentDescription = "web-chat-conversation-action-move-to-project",
            ))
            add(WebChatActionSheetItem(
                id = ACTION_MORE_SETTINGS,
                title = "更多会话设置",
                subtitle = "重命名、归档、分享或删除",
                contentDescription = "web-chat-conversation-action-more-settings",
            ))
        }
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "会话操作",
            items = items,
            footerActions = listOf(WebChatActionSheetFooterAction(
                label = "官网完成",
                contentDescription = "web-chat-conversation-actions-official",
                action = openOfficialFallback,
            )),
            onCancelled = { requestEpoch += 1 },
            onDismissed = {
                activeSheet = null
                val actionId = selectedActionId
                selectedActionId = null
                if (actionId != null) {
                    host.postDelayed({
                        if (epoch == requestEpoch) dispatchSelectedAction(actionId, conversation)
                    }, ACTION_SHEET_HANDOFF_SETTLE_MS)
                }
            },
        ) { item ->
            selectedActionId = item.id
        }
    }

    private fun dispatchSelectedAction(
        actionId: String,
        conversation: ChatGptWebConversation,
    ) {
        when (actionId) {
            ACTION_SET_PINNED -> pinnedMutation.start(conversation)
            ACTION_MOVE_TO_PROJECT -> projectMove.show(conversation)
            ACTION_MORE_SETTINGS -> showPageActionsFor(conversation)
        }
    }

    private fun showPageActionsFor(conversation: ChatGptWebConversation) {
        val targetPath = ChatGptWebConversationPath.normalize(conversation.path)
            ?: return showRecovery(conversation)
        if (WebChatConversationDraftNavigation.blocks(
                targetPath = targetPath,
                currentPath = currentConversationPath(),
                draftPresent = consumerPort()?.state()?.draftPresent == true,
            )
        ) {
            showDraftBlocked()
            return
        }
        val epoch = requestEpoch
        when (readiness(targetPath)) {
            WebChatConversationActionReadiness.SHOW -> showPageActions()
            WebChatConversationActionReadiness.CANCEL -> Unit
            WebChatConversationActionReadiness.WAIT -> {
                if (!openConversationTracked(targetPath).accepted) return showRecovery(conversation)
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
        pinnedMutation.cancelPending()
        projectMove.cancelPending()
    }

    fun recoverPending(): Boolean = projectMove.recoverPending()

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

    private fun showDraftBlocked() {
        if (activity.isFinishing || activity.isDestroyed) return
        dismissTransition()
        WebChatConversationDraftNavigation.dialog(activity).show()
    }

    private companion object {
        const val ACTION_SET_PINNED = "set-pinned"
        const val ACTION_MOVE_TO_PROJECT = "move-to-project"
        const val ACTION_MORE_SETTINGS = "more-settings"
        const val ACTION_SHEET_HANDOFF_SETTLE_MS = 48L
        const val POLL_INTERVAL_MS = 250L
        const val MAX_POLL_ATTEMPTS = 24
    }
}
