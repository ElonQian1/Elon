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
        val actions = metadata?.actions.orEmpty()
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
    val label: String,
    val requiresUserConfirmation: Boolean,
    val nativeSelector: String,
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
            control.region == "message" &&
                control.enabled &&
                control.contextId?.let(ChatGptNativeControlPresentation::stableContextId) == contextId &&
                !isPrimaryCopy(control) &&
                descriptor.presentation != WebChatConsumerControlPresentation.OFFICIAL_FALLBACK
        }
        .map { descriptor ->
            val control = descriptor.control
            WebChatContextAction(
                controlId = control.id,
                label = control.label.trim(),
                requiresUserConfirmation = descriptor.requiresUserConfirmation,
                nativeSelector = descriptor.nativeSelector
                    ?.trim()
                    ?.takeIf(String::isNotBlank)
                    ?: "web-chat-message-context-action:" +
                    ChatGptNativeControlPresentation.stableContextId(control.id),
            )
        }
        .filter { it.controlId.isNotBlank() && it.label.isNotBlank() }
        .distinctBy(WebChatContextAction::controlId)
        .take(MAX_CONTEXT_ACTIONS)
        .toList()

    private fun isPrimaryCopy(control: WebChatConsumerControl): Boolean =
        control.semantic == "copy" || control.label.trim() in setOf("复制", "Copy")

    private const val MAX_CONTEXT_ACTIONS = 50
}

internal class WebChatProductionMessageActionCoordinator(
    private val activity: AppCompatActivity,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val openOfficialFallback: () -> Unit,
) {
    private val clipboard = ChatGptMessageClipboard(activity)

    fun handle(message: ChatMessage, action: WebChatMessageAction) {
        val metadata = message.webChatMessage ?: return
        if (action !in metadata.actions) return
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
            WebChatMessageAction.MORE -> showMore(metadata)
        }
    }

    private fun showMore(message: WebChatProductionMessage) {
        val port = consumerPort() ?: return openOfficialFallback()
        val contextId = ChatGptNativeControlPresentation.stableContextId(message.sourceMessageId)
        val actions = WebChatProductionMessageActionControls.contextActions(
            port.state().controls,
            contextId,
        )
        if (actions.isEmpty()) {
            showOfficialFallback(
                title = "消息操作",
                message = "当前消息操作已变化，可以在官方页面继续。",
            )
            return
        }
        val byId = actions.associateBy(WebChatContextAction::controlId)
        WebChatActionSheet.show(
            activity = activity,
            title = "消息操作",
            items = actions.map { action ->
                WebChatActionSheetItem(
                    id = action.controlId,
                    title = action.label,
                    subtitle = if (action.requiresUserConfirmation) "执行前需要确认" else null,
                    contentDescription = action.nativeSelector,
                )
            },
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "官网功能",
                    contentDescription = "web-chat-message-actions-official",
                    action = openOfficialFallback,
                ),
            ),
        ) { item -> byId[item.id]?.let(::confirmAndInvoke) }
    }

    private fun confirmAndInvoke(action: WebChatContextAction) {
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
}
