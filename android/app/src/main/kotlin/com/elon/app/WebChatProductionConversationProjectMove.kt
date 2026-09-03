package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndex
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebProject

internal class WebChatProductionConversationProjectMoveCoordinator(
    private val activity: AppCompatActivity,
    private val host: android.view.View,
    private val activeProvider: () -> WebChatProviderId?,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val currentConversationPath: () -> String?,
    private val currentState: () -> String,
    private val openConversation: (String) -> WebChatConsumerCommandResult,
    private val conversationIndex: () -> ChatGptWebConversationIndexState,
    private val refreshConversationIndex: (String?) -> Boolean,
    private val probeConversationProject: (String, String) -> Boolean,
    private val suspendConversationRefresh: () -> Unit,
    private val resumeConversationRefresh: () -> Unit,
    private val openOfficialFallback: () -> Unit,
) {
    private val recoveryStore = WebChatConversationProjectMoveRecoveryStore(activity)
    private val ui = WebChatConversationProjectMoveUi(activity, host)
    private val readTransition = WebChatConversationProjectMoveReadTransition(host, ::isCurrent)
    private val destinationFallback = WebChatConversationProjectMoveDestinationFallback(
        ui,
        recoveryStore,
        openOfficialFallback,
        ::isCurrent,
        ::cancelPending,
        onPrepared = { conversation, selected, port, epoch ->
            projectChoiceRevealRequested = false
            projectChoiceRevealRequestId = null
            showProgress(selected, "正在提交一次移动操作")
            waitForProjectChoice(conversation, selected, port, epoch, attempt = 0)
        },
        onPrepareFailed = { conversation, selected, epoch ->
            fail(conversation, selected, "无法更新安全恢复记录", epoch)
        },
    )
    private val reconciler = WebChatConversationProjectMoveReconciler(
        host = host,
        conversationIndex = conversationIndex,
        probeConversationProject = probeConversationProject,
        refreshConversationIndex = refreshConversationIndex,
        suspendConversationRefresh = suspendConversationRefresh,
        resumeConversationRefresh = resumeConversationRefresh,
        readTransition = readTransition,
        isCurrent = ::isCurrent,
        requestConfirmation = { port, control, epoch, onSucceeded, onFailed ->
            invokeAndWait(port, control, true, epoch, onSucceeded, onFailed)
        },
        updateProgress = ::updateProgress,
        onCompleted = ::complete,
        onNotApplied = ::settleNotApplied,
        onFailed = ::fail,
    )
    private var requestEpoch = 0
    private var writeAttempted = false
    private var conversationRefreshHeld = false
    private var recoveryActive = false
    private var recoveryDirectoryRequested = false
    private var lastRecoveryAttemptKey: String? = null
    private var projectChoiceRevealRequested = false
    private var projectChoiceRevealRequestId: String? = null
    private var navigationRequestId: String? = null

    fun show(conversation: ChatGptWebConversation) {
        cancelPending()
        if (recoverPending(interactive = true)) return
        val destinations = WebChatConversationProjectMovePolicy.destinations(
            conversationIndex(),
            conversation,
        )
        if (destinations.isEmpty()) {
            showNoDestinations()
            return
        }
        val epoch = requestEpoch
        ui.showDestinationPicker(
            destinations = destinations,
            onCancelled = { requestEpoch += 1 },
            onSelected = { destination ->
                if (isCurrent(epoch)) beginMove(conversation, destination)
            },
            openOfficialFallback = openOfficialFallback,
        )
    }

    fun recoverPending(interactive: Boolean = false): Boolean {
        val record = recoveryStore.restore() ?: return false
        if (record.stage == WebChatConversationProjectMoveStage.PREPARED) {
            recoveryStore.clear()
            return false
        }
        if (recoveryActive) return true
        val attemptKey = "${record.updatedAtMs}:${record.destinationProjectId}"
        if (lastRecoveryAttemptKey == attemptKey) {
            if (interactive) showPendingRecovery()
            return true
        }
        if (
            activeProvider() != WebChatProviderId.CHATGPT_WEB ||
            currentState() != "ready"
        ) return true
        val index = conversationIndex()
        val destination = index.projects.singleOrNull { it.id == record.destinationProjectId }
        if (destination == null) {
            if (!recoveryDirectoryRequested) {
                recoveryDirectoryRequested = true
                refreshConversationIndex(null)
            }
            if (interactive) showPendingRecovery()
            return true
        }
        val identity = ChatGptWebConversationPath.identity(record.conversationPath)
            ?: run {
                recoveryStore.clear()
                return false
            }
        val conversation = index.conversations.firstOrNull {
            ChatGptWebConversationIndex.identityOf(it) == identity
        } ?: ChatGptWebConversation(
            id = identity,
            title = "当前会话",
            path = record.conversationPath,
            active = false,
            projectId = record.sourceProjectId,
        )
        val port = consumerPort() ?: return true
        requestEpoch += 1
        val epoch = requestEpoch
        lastRecoveryAttemptKey = attemptKey
        recoveryActive = true
        recoveryDirectoryRequested = false
        writeAttempted = true
        holdConversationRefresh()
        showProgress(destination, "正在恢复移动结果")
        beginReadOnlyReconciliation(
            conversation,
            destination,
            port,
            epoch,
            sourceProjectId = record.sourceProjectId,
            allowConfirmation = false,
        )
        return true
    }

    fun cancelPending() {
        requestEpoch += 1
        ui.dismissAll()
        writeAttempted = false
        recoveryActive = false
        projectChoiceRevealRequested = false
        projectChoiceRevealRequestId = null
        navigationRequestId = null
        destinationFallback.reset()
        clearPreparedRecovery()
        releaseConversationRefresh()
    }

    private fun beginMove(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
    ) {
        val epoch = requestEpoch
        writeAttempted = false
        recoveryActive = false
        lastRecoveryAttemptKey = null
        recoveryDirectoryRequested = false
        projectChoiceRevealRequested = false
        projectChoiceRevealRequestId = null
        navigationRequestId = null
        destinationFallback.reset()
        val targetPath = ChatGptWebConversationPath.normalize(conversation.path)
            ?: return fail(conversation, destination, "会话地址已经变化", epoch)
        val initialReadiness = readiness(targetPath)
        if (initialReadiness == WebChatConversationActionReadiness.WAIT && blocksForDraft(targetPath)) {
            blockForDraft(epoch)
            return
        }
        holdConversationRefresh()
        showProgress(destination, "正在准备当前会话")
        if (recoveryStore.prepare(conversation, destination) == null) {
            fail(conversation, destination, "无法建立安全恢复记录", epoch)
            return
        }
        when (initialReadiness) {
            WebChatConversationActionReadiness.SHOW -> openConversationOptions(
                conversation,
                destination,
                epoch,
            )
            WebChatConversationActionReadiness.CANCEL -> {
                recoveryStore.clear()
                releaseConversationRefresh()
            }
            WebChatConversationActionReadiness.WAIT -> {
                val navigation = openConversation(targetPath)
                if (!navigation.accepted) {
                    fail(conversation, destination, "暂时无法打开该会话", epoch)
                    return
                }
                navigationRequestId = navigation.requestId
                updateProgress(destination, "正在切换到该会话")
                pollUntilReady(conversation, destination, targetPath, epoch, attempt = 0)
            }
        }
    }

    private fun pollUntilReady(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        targetPath: String,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(epoch)) return
        when (readiness(targetPath)) {
            WebChatConversationActionReadiness.SHOW -> openConversationOptions(
                conversation,
                destination,
                epoch,
            )
            WebChatConversationActionReadiness.CANCEL -> {
                recoveryStore.clear()
                releaseConversationRefresh()
            }
            WebChatConversationActionReadiness.WAIT -> {
                if (blocksForDraft(targetPath)) {
                    blockForDraft(epoch)
                    return
                }
                val navigationStatus = consumerPort()?.state()?.let { state ->
                    WebChatConversationProjectMovePolicy.commandStatus(state, navigationRequestId)
                } ?: WebChatConsumerCommandStatus.UNKNOWN
                if (navigationStatus == WebChatConsumerCommandStatus.FAILED) {
                    fail(conversation, destination, "官网拒绝切换会话", epoch)
                    return
                }
                if (navigationStatus == WebChatConsumerCommandStatus.TIMED_OUT) {
                    fail(conversation, destination, "官网未确认切换会话", epoch)
                    return
                }
                if (attempt >= MAX_NAVIGATION_POLLS) {
                    fail(conversation, destination, "切换会话超时", epoch)
                    return
                }
                if (
                    navigationRequestId == null &&
                    WebChatConversationProjectMoveTiming.shouldRetryNavigation(attempt)
                ) {
                    val navigation = openConversation(targetPath)
                    if (!navigation.accepted) {
                        fail(conversation, destination, "暂时无法打开该会话", epoch)
                        return
                    }
                    navigationRequestId = navigation.requestId
                }
                host.postDelayed({
                    pollUntilReady(conversation, destination, targetPath, epoch, attempt + 1)
                }, POLL_INTERVAL_MS)
            }
        }
    }

    private fun openConversationOptions(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        epoch: Int,
    ) {
        if (!isCurrent(epoch)) return
        navigationRequestId = null
        val port = consumerPort()
            ?: return fail(conversation, destination, "网页会话正在恢复", epoch)
        updateProgress(destination, "正在打开会话设置")
        val initial = WebChatConversationProjectMovePolicy.conversationOptions(
            port.state(),
            conversation,
        )
        if (initial != null) {
            readTransition.invoke(
                port = port,
                control = initial,
                userConfirmed = false,
                epoch = epoch,
                onAccepted = {
                    waitForMoveTrigger(conversation, destination, port, epoch, attempt = 0)
                },
                onRejected = {
                    retryConversationOptionsAfterReadRejection(
                        conversation,
                        destination,
                        port,
                        epoch,
                        pollAttempt = 0,
                        invokeRetry = 0,
                    )
                },
            )
            return
        }
        pollForConversationOptions(
            conversation,
            destination,
            port,
            epoch,
            attempt = 0,
            invokeRetry = 0,
        )
    }

    private fun pollForConversationOptions(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
        invokeRetry: Int,
    ) {
        if (!isCurrent(epoch)) return
        val control = WebChatConversationProjectMovePolicy.conversationOptions(
            port.state(),
            conversation,
        )
        if (control != null) {
            readTransition.invoke(
                port = port,
                control = control,
                userConfirmed = false,
                epoch = epoch,
                onAccepted = {
                    waitForMoveTrigger(conversation, destination, port, epoch, attempt = 0)
                },
                onRejected = {
                    retryConversationOptionsAfterReadRejection(
                        conversation,
                        destination,
                        port,
                        epoch,
                        pollAttempt = attempt,
                        invokeRetry = invokeRetry,
                    )
                },
            )
            return
        }
        if (WebChatConversationProjectMoveTiming.shouldRefreshControls(attempt)) {
            readTransition.refreshControls(port, epoch) {
                pollForConversationOptions(
                    conversation,
                    destination,
                    port,
                    epoch,
                    attempt + 1,
                    invokeRetry,
                )
            }
            return
        }
        if (attempt >= MAX_CONTROL_POLLS) {
            fail(conversation, destination, "未找到当前会话设置", epoch)
            return
        }
        host.postDelayed({
            pollForConversationOptions(
                conversation,
                destination,
                port,
                epoch,
                attempt + 1,
                invokeRetry,
            )
        }, POLL_INTERVAL_MS)
    }

    private fun retryConversationOptionsAfterReadRejection(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        pollAttempt: Int,
        invokeRetry: Int,
    ) {
        if (!isCurrent(epoch)) return
        if (invokeRetry >= MAX_READ_CONTROL_INVOKE_RETRIES) {
            fail(conversation, destination, "无法打开会话设置", epoch)
            return
        }
        readTransition.refreshControls(port, epoch) {
            pollForConversationOptions(
                conversation,
                destination,
                port,
                epoch,
                attempt = pollAttempt + 1,
                invokeRetry = invokeRetry + 1,
            )
        }
    }

    private fun waitForMoveTrigger(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        readTransition.waitForMoveTrigger(
            conversation = conversation,
            port = port,
            epoch = epoch,
            attempt = attempt,
            onProgress = { updateProgress(destination, it) },
            onReady = {
                waitForProjectChoice(conversation, destination, port, epoch, attempt = 0)
            },
            onFailure = { fail(conversation, destination, it, epoch) },
        )
    }

    private fun waitForProjectChoice(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(epoch)) return
        val choice = WebChatConversationProjectMovePolicy.projectChoice(port.state(), destination)
        if (choice != null) {
            updateProgress(destination, "正在提交一次移动操作")
            if (recoveryStore.armWrite() == null) {
                fail(conversation, destination, "无法保存移动恢复状态", epoch)
                return
            }
            writeAttempted = true
            val result = port.invokeControl(choice.control.id, userConfirmed = true)
            if (!result.accepted) {
                writeAttempted = false
                recoveryStore.clear()
                fail(conversation, destination, "移动操作未提交", epoch)
                return
            }
            if (result.requestId.isNullOrBlank()) {
                beginReadOnlyReconciliation(
                    conversation,
                    destination,
                    port,
                    epoch,
                    sourceProjectId(conversation),
                )
                return
            }
            waitForCommand(
                port = port,
                requestId = result.requestId,
                epoch = epoch,
                attempt = 0,
                onSucceeded = {
                    beginReadOnlyReconciliation(
                        conversation,
                        destination,
                        port,
                        epoch,
                        sourceProjectId(conversation),
                    )
                },
                onFailed = {
                    // The official page may navigate while handling the project choice and lose
                    // its command receipt. Never replay the write; reconcile the observed owner.
                    beginReadOnlyReconciliation(
                        conversation,
                        destination,
                        port,
                        epoch,
                        sourceProjectId(conversation),
                    )
                },
            )
            return
        }
        if (!projectChoiceRevealRequested) {
            projectChoiceRevealRequested = true
            updateProgress(destination, "正在查找目标项目")
            val result = port.revealProjectChoice(destination.title)
            projectChoiceRevealRequestId = result.requestId
            if (!result.accepted) {
                if (destinationFallback.show(
                        conversationIndex(), conversation, port, epoch,
                    )) return
                fail(conversation, destination, "所选项目当前不可用", epoch)
                return
            }
            host.postDelayed({
                waitForProjectChoice(conversation, destination, port, epoch, attempt + 1)
            }, POLL_INTERVAL_MS)
            return
        }
        val revealStatus = WebChatConversationProjectMovePolicy.commandStatus(
            port.state(),
            projectChoiceRevealRequestId,
        )
        if (
            revealStatus == WebChatConsumerCommandStatus.FAILED ||
            revealStatus == WebChatConsumerCommandStatus.TIMED_OUT
        ) {
            if (destinationFallback.show(
                    conversationIndex(), conversation, port, epoch,
                )) return
            fail(conversation, destination, "所选项目当前不可用", epoch)
            return
        }
        if (WebChatConversationProjectMoveTiming.shouldRefreshControls(attempt)) {
            readTransition.refreshControls(port, epoch) {
                waitForProjectChoice(
                    conversation,
                    destination,
                    port,
                    epoch,
                    attempt + 1,
                )
            }
            return
        }
        if (attempt >= MAX_CONTROL_POLLS) {
            if (destinationFallback.show(
                    conversationIndex(), conversation, port, epoch,
                )) return
            fail(conversation, destination, "所选项目当前不可用", epoch)
            return
        }
        host.postDelayed({
            waitForProjectChoice(conversation, destination, port, epoch, attempt + 1)
        }, POLL_INTERVAL_MS)
    }

    private fun beginReadOnlyReconciliation(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        sourceProjectId: String?,
        allowConfirmation: Boolean = true,
    ) {
        if (!isCurrent(epoch)) return
        releaseConversationRefresh()
        reconciler.begin(
            conversation,
            destination,
            port,
            epoch,
            sourceProjectId = sourceProjectId,
            allowConfirmation = allowConfirmation,
        )
    }

    private fun invokeAndWait(
        port: WebChatConsumerPort,
        control: WebChatConsumerControlDescriptor,
        userConfirmed: Boolean,
        epoch: Int,
        onSucceeded: () -> Unit,
        onFailed: () -> Unit,
    ) {
        val result = port.invokeControl(control.control.id, userConfirmed)
        if (!result.accepted || result.requestId.isNullOrBlank()) {
            onFailed()
            return
        }
        waitForCommand(
            port,
            result.requestId,
            epoch,
            attempt = 0,
            onSucceeded,
            onFailed,
        )
    }

    private fun waitForCommand(
        port: WebChatConsumerPort,
        requestId: String,
        epoch: Int,
        attempt: Int,
        onSucceeded: () -> Unit,
        onFailed: () -> Unit,
    ) {
        if (!isCurrent(epoch)) return
        when (WebChatConversationProjectMovePolicy.commandStatus(port.state(), requestId)) {
            WebChatConsumerCommandStatus.SUCCEEDED -> onSucceeded()
            WebChatConsumerCommandStatus.FAILED,
            WebChatConsumerCommandStatus.TIMED_OUT -> onFailed()
            WebChatConsumerCommandStatus.PENDING,
            WebChatConsumerCommandStatus.UNKNOWN -> {
                if (attempt >= MAX_COMMAND_POLLS) {
                    onFailed()
                    return
                }
                host.postDelayed({
                    waitForCommand(
                        port,
                        requestId,
                        epoch,
                        attempt + 1,
                        onSucceeded,
                        onFailed,
                    )
                }, POLL_INTERVAL_MS)
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

    private fun showProgress(destination: ChatGptWebProject, subtitle: String) {
        ui.showProgress(destination, subtitle)
    }

    private fun updateProgress(destination: ChatGptWebProject, subtitle: String) {
        ui.updateProgress(destination, subtitle)
    }

    private fun complete(destination: ChatGptWebProject) {
        ui.complete(destination)
        writeAttempted = false
        recoveryActive = false
        lastRecoveryAttemptKey = null
        navigationRequestId = null
        recoveryStore.clear()
        releaseConversationRefresh()
    }

    private fun settleNotApplied(conversation: ChatGptWebConversation, epoch: Int) {
        if (!isCurrent(epoch)) return
        writeAttempted = false
        recoveryActive = false
        lastRecoveryAttemptKey = null
        navigationRequestId = null
        recoveryStore.clear()
        releaseConversationRefresh()
        if (activity.isFinishing || activity.isDestroyed) return
        ui.showNotApplied(
            onOfficialFallback = openOfficialFallback,
            onRetry = { show(conversation) },
        )
    }

    private fun fail(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        detail: String,
        epoch: Int,
    ) {
        if (!isCurrent(epoch)) return
        releaseConversationRefresh()
        if (activity.isFinishing || activity.isDestroyed) return
        val attempted = writeAttempted
        navigationRequestId = null
        recoveryActive = false
        lastRecoveryAttemptKey = null
        if (!attempted) recoveryStore.clear()
        ui.showFailure(
            attempted = attempted,
            detail = detail,
            onOfficialFallback = openOfficialFallback,
            onRefresh = {
                lastRecoveryAttemptKey = null
                refreshConversationIndex(destination.id)
                host.postDelayed({ recoverPending(interactive = true) }, POLL_INTERVAL_MS)
            },
            onRetry = { show(conversation) },
        )
    }

    private fun blocksForDraft(targetPath: String) = WebChatConversationDraftNavigation.blocks(
        targetPath, currentConversationPath(), consumerPort()?.state()?.draftPresent == true,
    )

    private fun blockForDraft(epoch: Int) {
        if (!isCurrent(epoch)) return
        cancelPending()
        if (!activity.isFinishing && !activity.isDestroyed) ui.showDraftBlocked()
    }

    private fun sourceProjectId(conversation: ChatGptWebConversation): String? =
        ChatGptWebConversationPath.canonicalProjectId(conversation.projectId)
            ?: ChatGptWebConversationPath.projectId(conversation.path)

    private fun holdConversationRefresh() {
        if (conversationRefreshHeld) return
        conversationRefreshHeld = true
        suspendConversationRefresh()
    }

    private fun releaseConversationRefresh() {
        if (!conversationRefreshHeld) return
        conversationRefreshHeld = false
        resumeConversationRefresh()
    }

    private fun clearPreparedRecovery() {
        if (recoveryStore.restore()?.stage == WebChatConversationProjectMoveStage.PREPARED) {
            recoveryStore.clear()
        }
    }

    private fun showPendingRecovery() {
        ui.showPendingRecovery(
            onRefresh = {
                lastRecoveryAttemptKey = null
                refreshConversationIndex(null)
                host.postDelayed({ recoverPending(interactive = true) }, POLL_INTERVAL_MS)
            },
            onOfficialFallback = openOfficialFallback,
        )
    }

    private fun showNoDestinations() {
        ui.showNoDestinations(
            onRefresh = { refreshConversationIndex(null) },
            onOfficialFallback = openOfficialFallback,
        )
    }

    private fun isCurrent(epoch: Int): Boolean =
        epoch == requestEpoch && activeProvider() == WebChatProviderId.CHATGPT_WEB

    private companion object {
        const val POLL_INTERVAL_MS = WebChatConversationProjectMoveTiming.POLL_INTERVAL_MS
        const val MAX_NAVIGATION_POLLS = WebChatConversationProjectMoveTiming.NAVIGATION_POLL_LIMIT
        const val MAX_CONTROL_POLLS = WebChatConversationProjectMoveTiming.CONTROL_POLL_LIMIT
        const val MAX_COMMAND_POLLS = WebChatConversationProjectMoveTiming.COMMAND_POLL_LIMIT
        const val MAX_READ_CONTROL_INVOKE_RETRIES = 1
    }
}
