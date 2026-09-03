package com.elon.app

import android.view.View
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject

internal class WebChatConversationProjectMoveReconciler(
    private val host: View,
    private val conversationIndex: () -> ChatGptWebConversationIndexState,
    private val probeConversationProject: (String, String) -> Boolean,
    private val refreshConversationIndex: (String?) -> Boolean,
    private val suspendConversationRefresh: () -> Unit,
    private val resumeConversationRefresh: () -> Unit,
    private val readTransition: WebChatConversationProjectMoveReadTransition,
    private val isCurrent: (Int) -> Boolean,
    private val requestConfirmation: (
        WebChatConsumerPort,
        WebChatConsumerControlDescriptor,
        Int,
        () -> Unit,
        () -> Unit,
    ) -> Unit,
    private val updateProgress: (ChatGptWebProject, String) -> Unit,
    private val onCompleted: (ChatGptWebProject) -> Unit,
    private val onNotApplied: (ChatGptWebConversation, Int) -> Unit,
    private val onFailed: (ChatGptWebConversation, ChatGptWebProject, String, Int) -> Unit,
) {
    fun begin(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        sourceProjectId: String?,
        allowConfirmation: Boolean,
    ) {
        if (!isCurrent(epoch)) return
        updateProgress(destination, "正在同步会话目录")
        requestMembershipReconciliation(conversation, destination)
        poll(
            conversation,
            destination,
            port,
            epoch,
            attempt = 0,
            sourceProjectId = sourceProjectId,
            confirmationAttempted = false,
            confirmationAllowed = allowConfirmation,
        )
    }

    private fun poll(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
        sourceProjectId: String?,
        confirmationAttempted: Boolean,
        confirmationAllowed: Boolean,
    ) {
        if (!isCurrent(epoch)) return
        val index = conversationIndex()
        val state = port.state()
        if (
            attempt == 0 ||
            WebChatConversationProjectMoveTiming.shouldRefreshDirectory(attempt) ||
            attempt >= WebChatConversationProjectMoveTiming.RECONCILIATION_POLL_LIMIT
        ) {
            WebChatConversationProjectMoveDiagnostics.recordReconciliation(
                attempt,
                index,
                state,
                conversation,
                destination,
            )
        }
        val outcome = WebChatConversationProjectMoveRecoveryPolicy.resolve(
            index,
            conversation,
            sourceProjectId,
            destination,
        )
        if (outcome == WebChatConversationProjectMoveRecoveryOutcome.MOVED_TO_DESTINATION) {
            onCompleted(destination)
            return
        }
        if (confirmationAllowed && !confirmationAttempted) {
            val confirmation = WebChatConversationProjectMovePolicy.confirmation(state, conversation)
            if (confirmation != null) {
                updateProgress(destination, "正在确认移动")
                requestConfirmation(
                    port,
                    confirmation,
                    epoch,
                    {
                        updateProgress(destination, "正在同步会话目录")
                        requestMembershipReconciliation(conversation, destination)
                        poll(
                            conversation,
                            destination,
                            port,
                            epoch,
                            attempt = 0,
                            sourceProjectId = sourceProjectId,
                            confirmationAttempted = true,
                            confirmationAllowed = true,
                        )
                    },
                    { onFailed(conversation, destination, "官网未确认移动结果", epoch) },
                )
                return
            }
            if (WebChatConversationProjectMoveTiming.shouldRefreshControls(attempt)) {
                readTransition.refreshControls(port, epoch) {
                    poll(
                        conversation,
                        destination,
                        port,
                        epoch,
                        attempt + 1,
                        sourceProjectId,
                        confirmationAttempted,
                        confirmationAllowed,
                    )
                }
                return
            }
        }
        if (attempt >= WebChatConversationProjectMoveTiming.RECONCILIATION_POLL_LIMIT) {
            if (outcome == WebChatConversationProjectMoveRecoveryOutcome.REMAINS_AT_SOURCE) {
                onNotApplied(conversation, epoch)
            } else {
                onFailed(conversation, destination, "目录尚未确认移动结果", epoch)
            }
            return
        }
        if (WebChatConversationProjectMoveTiming.shouldRefreshDirectory(attempt)) {
            requestMembershipReconciliation(
                conversation,
                destination,
                fullDirectory = WebChatConversationProjectMoveTiming.shouldRefreshFullDirectory(
                    attempt,
                ),
            )
        }
        host.postDelayed({
            poll(
                conversation,
                destination,
                port,
                epoch,
                attempt + 1,
                sourceProjectId,
                confirmationAttempted,
                confirmationAllowed,
            )
        }, WebChatConversationProjectMoveTiming.POLL_INTERVAL_MS)
    }

    private fun requestMembershipReconciliation(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        fullDirectory: Boolean = false,
    ) {
        probeConversationProject(conversation.path, destination.id)
        if (fullDirectory) restartConversationRefreshGlobally()
        else refreshConversationIndex(destination.id)
    }

    private fun restartConversationRefreshGlobally() {
        suspendConversationRefresh()
        resumeConversationRefresh()
        refreshConversationIndex(null)
    }
}
