package com.elon.app

import android.view.View
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptMessageClipboard
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation

internal object WebChatProductionMessageActionBinder {
    fun bind(
        itemView: View,
        message: ChatMessage,
        onAction: ((ChatMessage, WebChatMessageAction) -> Unit)?,
    ) {
        val row = itemView.findViewById<LinearLayout>(R.id.webChatMessageActionBar) ?: return
        val metadata = message.webChatMessage
        val actions = WebChatProductionMessageActionPolicy.resolve(message)
        val visible = metadata != null && actions.isNotEmpty() && onAction != null
        row.visibility = if (visible) View.VISIBLE else View.GONE
        if (!visible || metadata == null) {
            row.contentDescription = null
            return
        }
        val actionHandler = onAction ?: return

        row.contentDescription = "web-chat-message-actions:${selectorId(metadata)}"
        bindButton(
            itemView,
            R.id.webChatMessageCopy,
            message,
            metadata,
            WebChatMessageAction.COPY,
            actions,
            actionHandler,
        )
        bindButton(
            itemView,
            R.id.webChatMessageRegenerate,
            message,
            metadata,
            WebChatMessageAction.REGENERATE,
            actions,
            actionHandler,
        )
        bindButton(
            itemView,
            R.id.webChatMessageMore,
            message,
            metadata,
            WebChatMessageAction.MORE,
            actions,
            actionHandler,
        )
    }

    private fun bindButton(
        itemView: View,
        id: Int,
        message: ChatMessage,
        metadata: WebChatProductionMessage,
        action: WebChatMessageAction,
        actions: Set<WebChatMessageAction>,
        onAction: ((ChatMessage, WebChatMessageAction) -> Unit),
    ) {
        val button = itemView.findViewById<ImageButton>(id) ?: return
        val available = action in actions
        button.visibility = if (available) View.VISIBLE else View.GONE
        button.contentDescription = "web-chat-message-action:${selectorId(metadata)}:${action.wireValue}"
        button.setOnClickListener(if (available) View.OnClickListener { onAction(message, action) } else null)
    }

    private fun selectorId(message: WebChatProductionMessage): String =
        "${message.providerWireValue}:${ChatGptNativeControlPresentation.stableContextId(message.sourceMessageId)}"
}

internal data class WebChatContextAction(
    val controlId: String,
    val semantic: String,
    val label: String,
    val requiresUserConfirmation: Boolean,
    val nativeSelector: String,
    val subtitle: String? = null,
    val enabled: Boolean = true,
)

internal object WebChatProductionMessageActionFeedback {
    fun copyAccepted(): String = "已复制消息"

    fun regenerateAccepted(): String = "正在重新生成回答…"

    fun contextActionAccepted(label: String): String = "已执行：$label"
}

internal object WebChatProductionMessageActionControls {
    fun messageContextIds(controls: List<WebChatConsumerControlDescriptor>): Set<String> =
        controls.asSequence()
            .map(WebChatConsumerControlDescriptor::control)
            .filter { it.region == "message" && it.enabled && !isPrimaryCopy(it) }
            .mapNotNull(WebChatConsumerControl::contextId)
            .map(ChatGptNativeControlPresentation::stableContextId)
            .toSet()

    fun contextActions(
        controls: List<WebChatConsumerControlDescriptor>,
        contextId: String,
    ): List<WebChatContextAction> = controls.asSequence()
        .filter { descriptor ->
            val control = descriptor.control
            control.region in CONTEXT_REGIONS &&
                control.enabled &&
                control.contextId?.let(ChatGptNativeControlPresentation::stableContextId) == contextId &&
                !isPrimaryCopy(control) &&
                control.semantic != "more" &&
                descriptor.presentation != WebChatConsumerControlPresentation.OFFICIAL_FALLBACK
        }
        .map { descriptor ->
            val control = descriptor.control
            val officialReadAloud = control.semantic ==
                WebChatProductionReadAloudActionPolicy.OFFICIAL_SEMANTIC
            WebChatContextAction(
                controlId = control.id,
                semantic = control.semantic,
                label = if (officialReadAloud) {
                    WebChatProductionReadAloudActionPolicy.officialLabel(control.label)
                } else {
                    control.label.trim()
                },
                requiresUserConfirmation = descriptor.requiresUserConfirmation,
                nativeSelector = if (officialReadAloud) {
                    WebChatProductionReadAloudActionPolicy.officialSelector(contextId)
                } else {
                    descriptor.nativeSelector
                        ?.trim()
                        ?.takeIf(String::isNotBlank)
                        ?: "web-chat-message-context-action:" +
                        ChatGptNativeControlPresentation.stableContextId(control.id)
                },
            )
        }
        .filter { it.controlId.isNotBlank() && it.label.isNotBlank() }
        .distinctBy(WebChatContextAction::controlId)
        .take(MAX_CONTEXT_ACTIONS)
        .toList()

    fun messageOverflowControl(
        controls: List<WebChatConsumerControlDescriptor>,
        contextId: String,
    ): WebChatConsumerControlDescriptor? = controls.firstOrNull { descriptor ->
        val control = descriptor.control
        control.region == "message" &&
            control.semantic == "more" &&
            control.enabled &&
            control.contextId?.let(ChatGptNativeControlPresentation::stableContextId) == contextId
    }

    private fun isPrimaryCopy(control: WebChatConsumerControl): Boolean =
        control.semantic == "copy" || control.label.trim() in setOf("复制", "Copy")

    private const val MAX_CONTEXT_ACTIONS = 50
    private val CONTEXT_REGIONS = setOf("message", "overlay")
}

internal class WebChatProductionMessageActionCoordinator(
    private val activity: AppCompatActivity,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val openOfficialFallback: () -> Unit,
) {
    private val clipboard = ChatGptMessageClipboard(activity)
    private val nativeReadAloud = WebChatNativeReadAloudController(
        context = activity,
        onFailure = { showFeedback("朗读暂时不可用，请检查系统语音设置后重试") },
    )
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null
    private var actionById = emptyMap<String, WebChatContextAction>()

    fun handle(message: ChatMessage, action: WebChatMessageAction) {
        val metadata = message.webChatMessage ?: return
        if (action !in WebChatProductionMessageActionPolicy.resolve(message)) return
        when (action) {
            WebChatMessageAction.COPY -> {
                clipboard.copy(message.content)
                showFeedback(WebChatProductionMessageActionFeedback.copyAccepted())
            }
            WebChatMessageAction.REGENERATE -> {
                val accepted = dispatch(
                    failureMessage = "当前回答暂时不能重新生成",
                ) { port -> port.executeSessionCommand("chatgpt_regenerate_response") }
                if (accepted) showFeedback(WebChatProductionMessageActionFeedback.regenerateAccepted())
            }
            WebChatMessageAction.MORE -> showMore(message, metadata)
        }
    }

    fun release() {
        cancelPending()
        nativeReadAloud.release()
    }

    private fun showMore(chatMessage: ChatMessage, message: WebChatProductionMessage) {
        cancelPending()
        val port = consumerPort()
        val contextId = ChatGptNativeControlPresentation.stableContextId(message.sourceMessageId)
        val state = port?.state()
        val observed = WebChatProductionMessageActionControls.contextActions(
            state?.controls.orEmpty(),
            contextId,
        )
        val overflow = state?.let {
            WebChatProductionMessageActionControls.messageOverflowControl(it.controls, contextId)
        }
        val needsOfficialPreparation =
            WebChatProductionReadAloudActionPolicy.needsOfficialPreparation(
                actions = observed,
                portAvailable = port != null,
            )
        val epoch = requestEpoch
        presentMoreSheet(chatMessage, contextId, observed, needsOfficialPreparation)
        if (port == null || observed.any(WebChatProductionReadAloudActionPolicy::isOfficial)) return

        val request = overflow?.let { port.invokeControl(it.control.id, userConfirmed = false) }
            ?: port.requestControls()
        if (request.accepted) {
            pollOfficialActions(
                message = chatMessage,
                contextId = contextId,
                port = port,
                epoch = epoch,
                overlayRequested = overflow != null,
                attempt = 0,
            )
        } else {
            presentMoreSheet(chatMessage, contextId, observed, officialPending = false)
        }
    }

    private fun presentMoreSheet(
        message: ChatMessage,
        contextId: String,
        observed: List<WebChatContextAction>,
        officialPending: Boolean,
    ) {
        val official = observed.filter(WebChatProductionReadAloudActionPolicy::isOfficial)
        val actions = official +
            observed.filterNot(WebChatProductionReadAloudActionPolicy::isOfficial) +
            systemReadAloudAction(message, contextId)
        actionById = actions.associateBy(WebChatContextAction::controlId)
        val presented = buildList {
            if (officialPending && official.isEmpty()) {
                add(WebChatProductionReadAloudActionPolicy.pendingOfficialAction(contextId))
            }
            addAll(actions)
        }
        val items = presented.map { action ->
            WebChatActionSheetItem(
                id = action.controlId,
                title = action.label,
                subtitle = action.subtitle ?: if (action.requiresUserConfirmation) "执行前需要确认" else null,
                enabled = action.enabled,
                contentDescription = action.nativeSelector,
            )
        }
        activeSheet?.let { sheet ->
            sheet.updateItems(items)
            return
        }
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "消息操作",
            items = items,
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "官网功能",
                    contentDescription = "web-chat-message-actions-official",
                    action = openOfficialFallback,
                ),
            ),
            onDismissed = {
                requestEpoch += 1
                activeSheet = null
                actionById = emptyMap()
            },
        ) { item -> actionById[item.id]?.let { confirmAndInvoke(message, it) } }
    }

    private fun pollOfficialActions(
        message: ChatMessage,
        contextId: String,
        port: WebChatConsumerPort,
        epoch: Int,
        overlayRequested: Boolean,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeSheet == null) return
        val state = port.state()
        val observed = WebChatProductionMessageActionControls.contextActions(
            state.controls,
            contextId,
        )
        if (observed.any(WebChatProductionReadAloudActionPolicy::isOfficial)) {
            presentMoreSheet(message, contextId, observed, officialPending = false)
            return
        }
        var requested = overlayRequested
        if (!requested) {
            val overflow = WebChatProductionMessageActionControls.messageOverflowControl(
                state.controls,
                contextId,
            )
            if (overflow != null) {
                requested = port.invokeControl(overflow.control.id, userConfirmed = false).accepted
            }
        }
        if (attempt >= MAX_OFFICIAL_ACTION_POLL_ATTEMPTS) {
            presentMoreSheet(message, contextId, observed, officialPending = false)
            return
        }
        activity.window.decorView.postDelayed(
            {
                pollOfficialActions(
                    message,
                    contextId,
                    port,
                    epoch,
                    requested,
                    attempt + 1,
                )
            },
            OFFICIAL_ACTION_POLL_INTERVAL_MS,
        )
    }

    private fun cancelPending() {
        requestEpoch += 1
        activeSheet?.dismiss()
        activeSheet = null
        actionById = emptyMap()
    }

    private fun confirmAndInvoke(message: ChatMessage, action: WebChatContextAction) {
        if (action.semantic == WebChatProductionReadAloudActionPolicy.SYSTEM_SEMANTIC) {
            toggleSystemReadAloud(message)
            return
        }
        if (WebChatProductionReadAloudActionPolicy.isOfficial(action)) nativeReadAloud.stop()
        if (!action.requiresUserConfirmation) {
            invoke(action, userConfirmed = false)
            return
        }
        AlertDialog.Builder(activity)
            .setTitle(action.label)
            .setMessage("确认执行这个网页操作？")
            .setPositiveButton(android.R.string.ok) { _, _ -> invoke(action, userConfirmed = true) }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun invoke(action: WebChatContextAction, userConfirmed: Boolean) {
        val accepted = dispatch(
            failureMessage = "网页操作执行失败",
        ) { port -> port.invokeControl(action.controlId, userConfirmed) }
        if (accepted) {
            showFeedback(WebChatProductionMessageActionFeedback.contextActionAccepted(action.label))
        }
    }

    private fun dispatch(
        failureMessage: String,
        action: (WebChatConsumerPort) -> WebChatConsumerCommandResult,
    ): Boolean {
        val result = consumerPort()?.let(action)
        if (result?.accepted == true) return true
        showOfficialFallback(
            title = "消息操作",
            message = "$failureMessage。可以在官方页面继续。",
        )
        return false
    }

    private fun showOfficialFallback(title: String, message: String) {
        if (activity.isFinishing || activity.isDestroyed) return
        AlertDialog.Builder(activity)
            .setTitle(title)
            .setMessage(message)
            .setPositiveButton("打开官方页") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun showFeedback(message: String) =
        Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()

    private fun systemReadAloudAction(
        message: ChatMessage,
        contextId: String,
    ): List<WebChatContextAction> {
        val sourceId = message.webChatMessage?.sourceMessageId.orEmpty()
        if (message.role == "user" || message.content.isBlank() || sourceId.isBlank()) return emptyList()
        val active = nativeReadAloud.isActive(sourceId)
        return listOf(
            WebChatContextAction(
                controlId = "$SYSTEM_READ_ALOUD_CONTROL:$contextId",
                semantic = WebChatProductionReadAloudActionPolicy.SYSTEM_SEMANTIC,
                label = WebChatProductionReadAloudActionPolicy.systemLabel(active),
                requiresUserConfirmation = false,
                nativeSelector = WebChatProductionReadAloudActionPolicy.systemSelector(contextId),
            ),
        )
    }

    private fun toggleSystemReadAloud(message: ChatMessage) {
        val sourceId = message.webChatMessage?.sourceMessageId ?: return
        if (!nativeReadAloud.isActive(sourceId)) stopOfficialReadAloudIfActive()
        val feedback = when (nativeReadAloud.toggle(sourceId, message.content)) {
            WebChatNativeReadAloudResult.STARTED -> "开始系统朗读"
            WebChatNativeReadAloudResult.STOPPED -> "已停止系统朗读"
            WebChatNativeReadAloudResult.EMPTY -> "当前回答没有可朗读文字"
        }
        showFeedback(feedback)
    }

    private fun stopOfficialReadAloudIfActive() {
        val stopAction = actionById.values.firstOrNull { action ->
            WebChatProductionReadAloudActionPolicy.isOfficial(action) &&
                WebChatProductionReadAloudActionPolicy.isStopLabel(action.label)
        } ?: return
        consumerPort()?.invokeControl(stopAction.controlId, userConfirmed = false)
    }

    private companion object {
        const val SYSTEM_READ_ALOUD_CONTROL = "system_read_aloud"
        const val MAX_OFFICIAL_ACTION_POLL_ATTEMPTS = 12
        const val OFFICIAL_ACTION_POLL_INTERVAL_MS = 200L
    }
}
