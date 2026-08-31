package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
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
    private val openConversation: (String) -> Boolean,
    private val conversationIndex: () -> ChatGptWebConversationIndexState,
    private val refreshConversationIndex: (String?) -> Boolean,
    private val openOfficialFallback: () -> Unit,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null
    private var activeDialog: AlertDialog? = null
    private var destinationsById = emptyMap<String, ChatGptWebProject>()
    private var writeAttempted = false

    fun show(conversation: ChatGptWebConversation) {
        cancelPending()
        val destinations = WebChatConversationProjectMovePolicy.destinations(
            conversationIndex(),
            conversation,
        )
        if (destinations.isEmpty()) {
            showNoDestinations()
            return
        }
        destinationsById = destinations.associateBy(ChatGptWebProject::id)
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "移动到项目",
            items = destinations.map { project ->
                WebChatActionSheetItem(
                    id = project.id,
                    title = project.title,
                    subtitle = if (project.active) "当前打开" else null,
                    contentDescription = "web-chat-conversation-project-destination:${project.id}",
                )
            },
            footerActions = listOf(officialFooter()),
            onCancelled = { requestEpoch += 1 },
            onDismissed = {
                activeSheet = null
                destinationsById = emptyMap()
            },
        ) { item ->
            val destination = destinationsById[item.id] ?: return@showUpdatable
            host.post { beginMove(conversation, destination) }
        }
    }

    fun cancelPending() {
        requestEpoch += 1
        activeSheet?.dismiss()
        activeSheet = null
        activeDialog?.dismiss()
        activeDialog = null
        destinationsById = emptyMap()
        writeAttempted = false
    }

    private fun beginMove(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
    ) {
        val epoch = requestEpoch
        writeAttempted = false
        showProgress(destination, "正在准备当前会话")
        val targetPath = ChatGptWebConversationPath.normalize(conversation.path)
            ?: return fail(conversation, destination, "会话地址已经变化", epoch)
        when (readiness(targetPath)) {
            WebChatConversationActionReadiness.SHOW -> openConversationOptions(
                conversation,
                destination,
                epoch,
            )
            WebChatConversationActionReadiness.CANCEL -> Unit
            WebChatConversationActionReadiness.WAIT -> {
                if (!openConversation(targetPath)) {
                    fail(conversation, destination, "暂时无法打开该会话", epoch)
                    return
                }
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
            WebChatConversationActionReadiness.CANCEL -> Unit
            WebChatConversationActionReadiness.WAIT -> {
                if (attempt >= MAX_NAVIGATION_POLLS) {
                    fail(conversation, destination, "切换会话超时", epoch)
                    return
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
        val port = consumerPort()
            ?: return fail(conversation, destination, "网页会话正在恢复", epoch)
        updateProgress(destination, "正在打开会话设置")
        val initial = WebChatConversationProjectMovePolicy.conversationOptions(
            port.state(),
            conversation,
        )
        if (initial != null) {
            invokeAndWait(
                port = port,
                control = initial,
                userConfirmed = false,
                epoch = epoch,
                onSucceeded = {
                    waitForMoveTrigger(conversation, destination, port, epoch, attempt = 0)
                },
                onFailed = { fail(conversation, destination, "无法打开会话设置", epoch) },
            )
            return
        }
        val request = port.requestControls()
        if (!request.accepted) {
            fail(conversation, destination, "会话设置尚未就绪", epoch)
            return
        }
        pollForConversationOptions(conversation, destination, port, epoch, attempt = 0)
    }

    private fun pollForConversationOptions(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(epoch)) return
        val control = WebChatConversationProjectMovePolicy.conversationOptions(
            port.state(),
            conversation,
        )
        if (control != null) {
            invokeAndWait(
                port = port,
                control = control,
                userConfirmed = false,
                epoch = epoch,
                onSucceeded = {
                    waitForMoveTrigger(conversation, destination, port, epoch, attempt = 0)
                },
                onFailed = { fail(conversation, destination, "无法打开会话设置", epoch) },
            )
            return
        }
        if (attempt >= MAX_CONTROL_POLLS) {
            fail(conversation, destination, "未找到当前会话设置", epoch)
            return
        }
        host.postDelayed({
            pollForConversationOptions(conversation, destination, port, epoch, attempt + 1)
        }, POLL_INTERVAL_MS)
    }

    private fun waitForMoveTrigger(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(epoch)) return
        val trigger = WebChatConversationProjectMovePolicy.moveTrigger(
            port.state(),
            conversation,
        )
        if (trigger != null) {
            updateProgress(destination, "正在打开项目列表")
            invokeAndWait(
                port = port,
                control = trigger,
                userConfirmed = true,
                epoch = epoch,
                onSucceeded = {
                    waitForProjectChoice(conversation, destination, port, epoch, attempt = 0)
                },
                onFailed = { fail(conversation, destination, "无法打开项目列表", epoch) },
            )
            return
        }
        if (attempt == CONTROL_REFRESH_POLL) port.requestControls()
        if (attempt >= MAX_CONTROL_POLLS) {
            fail(conversation, destination, "官网项目入口暂不可用", epoch)
            return
        }
        host.postDelayed({
            waitForMoveTrigger(conversation, destination, port, epoch, attempt + 1)
        }, POLL_INTERVAL_MS)
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
            val result = port.invokeControl(choice.control.id, userConfirmed = true)
            writeAttempted = WebChatConversationProjectMovePolicy.writeMayHaveBeenSubmitted(result)
            if (!result.accepted || result.requestId.isNullOrBlank()) {
                fail(conversation, destination, "移动操作未提交", epoch)
                return
            }
            waitForCommand(
                port = port,
                requestId = result.requestId,
                epoch = epoch,
                attempt = 0,
                onSucceeded = {
                    updateProgress(destination, "正在同步会话目录")
                    refreshConversationIndex(destination.id)
                    pollReconciliation(conversation, destination, epoch, attempt = 0)
                },
                onFailed = { fail(conversation, destination, "官网未确认移动结果", epoch) },
            )
            return
        }
        if (attempt == CONTROL_REFRESH_POLL) port.requestControls()
        if (attempt >= MAX_CONTROL_POLLS) {
            fail(conversation, destination, "未找到所选项目", epoch)
            return
        }
        host.postDelayed({
            waitForProjectChoice(conversation, destination, port, epoch, attempt + 1)
        }, POLL_INTERVAL_MS)
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

    private fun pollReconciliation(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(epoch)) return
        if (WebChatConversationProjectMovePolicy.reconciled(
                conversationIndex(),
                conversation,
                destination,
            )
        ) {
            complete(destination)
            return
        }
        if (attempt >= MAX_RECONCILIATION_POLLS) {
            fail(conversation, destination, "目录尚未确认移动结果", epoch)
            return
        }
        host.postDelayed({
            pollReconciliation(conversation, destination, epoch, attempt + 1)
        }, POLL_INTERVAL_MS)
    }

    private fun readiness(targetPath: String): WebChatConversationActionReadiness =
        WebChatProductionConversationActionPolicy.evaluate(
            providerId = activeProvider(),
            targetPath = targetPath,
            currentPath = currentConversationPath(),
            state = currentState(),
        )

    private fun showProgress(destination: ChatGptWebProject, subtitle: String) {
        activeSheet?.dismiss()
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "移动到项目",
            items = listOf(progressItem(destination, subtitle)),
            footerActions = listOf(officialFooter()),
            onCancelled = { requestEpoch += 1 },
            onDismissed = { activeSheet = null },
        ) {}
    }

    private fun updateProgress(destination: ChatGptWebProject, subtitle: String) {
        activeSheet?.updateItems(listOf(progressItem(destination, subtitle)))
    }

    private fun progressItem(
        destination: ChatGptWebProject,
        subtitle: String,
    ) = WebChatActionSheetItem(
        id = "project-move-progress",
        title = destination.title,
        subtitle = subtitle,
        enabled = false,
        contentDescription = "web-chat-conversation-project-move-progress",
    )

    private fun complete(destination: ChatGptWebProject) {
        activeSheet?.dismiss()
        activeSheet = null
        writeAttempted = false
        Toast.makeText(activity, "已移动到“${destination.title}”", Toast.LENGTH_SHORT).show()
    }

    private fun fail(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
        detail: String,
        epoch: Int,
    ) {
        if (!isCurrent(epoch) || activity.isFinishing || activity.isDestroyed) return
        activeSheet?.dismiss()
        activeSheet = null
        val attempted = writeAttempted
        val message = if (attempted) {
            "$detail。已经提交过一次操作，为避免重复移动，应用不会自动重试。可以刷新目录或在官网确认。"
        } else {
            "$detail。尚未提交移动操作，可以重试或在官网完成。"
        }
        val builder = AlertDialog.Builder(activity)
            .setTitle(if (attempted) "移动结果待确认" else "暂时无法移动")
            .setMessage(message)
            .setPositiveButton("官网确认") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
        if (attempted) {
            builder.setNeutralButton("刷新目录") { _, _ ->
                refreshConversationIndex(destination.id)
            }
        } else {
            builder.setNeutralButton("重试") { _, _ -> show(conversation) }
        }
        val dialog = builder.create()
        activeDialog = dialog
        dialog.setOnDismissListener {
            if (activeDialog === dialog) activeDialog = null
        }
        dialog.show()
    }

    private fun showNoDestinations() {
        if (activity.isFinishing || activity.isDestroyed) return
        val dialog = AlertDialog.Builder(activity)
            .setTitle("没有其他项目")
            .setMessage("项目目录已经保留在本机。创建或同步其他项目后即可移动会话。")
            .setNeutralButton("刷新目录") { _, _ -> refreshConversationIndex(null) }
            .setPositiveButton("官网查看") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .create()
        activeDialog = dialog
        dialog.setOnDismissListener {
            if (activeDialog === dialog) activeDialog = null
        }
        dialog.show()
    }

    private fun officialFooter() = WebChatActionSheetFooterAction(
        label = "官网完成",
        contentDescription = "web-chat-conversation-project-move-official",
        action = openOfficialFallback,
    )

    private fun isCurrent(epoch: Int): Boolean =
        epoch == requestEpoch && activeProvider() == WebChatProviderId.CHATGPT_WEB

    private companion object {
        const val POLL_INTERVAL_MS = WebChatConversationProjectMoveTiming.POLL_INTERVAL_MS
        const val MAX_NAVIGATION_POLLS = WebChatConversationProjectMoveTiming.NAVIGATION_POLL_LIMIT
        const val MAX_CONTROL_POLLS = WebChatConversationProjectMoveTiming.CONTROL_POLL_LIMIT
        const val MAX_COMMAND_POLLS = WebChatConversationProjectMoveTiming.COMMAND_POLL_LIMIT
        const val MAX_RECONCILIATION_POLLS =
            WebChatConversationProjectMoveTiming.RECONCILIATION_POLL_LIMIT
        const val CONTROL_REFRESH_POLL = WebChatConversationProjectMoveTiming.CONTROL_REFRESH_POLL
    }
}
