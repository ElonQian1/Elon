package com.elon.app

import android.content.res.ColorStateList
import android.os.Build
import android.view.View
import android.widget.ImageButton
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptNativeNavigationSelector
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal data class WebChatProductionHeaderActionState(
    val temporaryChat: WebChatConsumerControl? = null,
    val conversationSettingsAvailable: Boolean = false,
) {
    val temporaryChatSelected: Boolean get() = temporaryChat?.selected == true
}

internal data class WebChatProductionHeaderButtonPresentation(
    val iconRes: Int,
    val selected: Boolean,
    val statusLabel: String,
)

internal object WebChatProductionHeaderActionPolicy {
    fun visible(
        provider: WebChatProviderIdentity,
        sessionState: String,
        pageKind: String,
    ): Boolean = provider.id == WebChatProviderId.CHATGPT_WEB &&
        provider.supports(WebChatProviderCapability.PAGE_ACTIONS) &&
        sessionState in USABLE_SESSION_STATES &&
        pageKind.trim().lowercase() in CHAT_PAGE_KINDS

    fun resolve(
        state: WebChatConsumerState,
        currentConversationPath: String?,
    ): WebChatProductionHeaderActionState = WebChatProductionHeaderActionState(
        temporaryChat = state.controls.asSequence()
            .map(WebChatConsumerControlDescriptor::control)
            .firstOrNull { control ->
                control.semantic == TEMPORARY_CHAT &&
                    control.enabled &&
                    control.supportsSelectedState
            },
        conversationSettingsAvailable = state.pageKind.equals("conversation", ignoreCase = true) &&
            ChatGptWebConversationPath.normalize(currentConversationPath) != null,
    )

    fun buttonPresentation(selected: Boolean?) = WebChatProductionHeaderButtonPresentation(
        iconRes = R.drawable.ic_temporary_chat,
        selected = selected == true,
        statusLabel = when (selected) {
            true -> "临时聊天已开启"
            false -> "临时聊天未开启"
            null -> "临时聊天状态同步中"
        },
    )

    fun temporaryChatItem(
        control: WebChatConsumerControl?,
        observation: WebChatProductionObservationState,
    ) = WebChatActionSheetItem(
        id = TEMPORARY_ITEM_ID,
        title = if (control?.selected == true) "关闭临时聊天" else "临时聊天",
        subtitle = when {
            control?.selected == true -> "已开启，本次对话不会出现在历史记录中"
            control != null -> "开启后，本次对话不会出现在历史记录中"
            observation == WebChatProductionObservationState.TEMPORARILY_UNOBSERVED ->
                "点按后重新同步官网状态并开启"
            else -> "点按后连接官网并开启，状态将在后台确认"
        },
        selected = control?.selected == true,
        contentDescription = ChatGptNativeNavigationSelector.TEMPORARY_CHAT,
    )

    private val CHAT_PAGE_KINDS = setOf("home", "conversation")
    private val USABLE_SESSION_STATES = setOf("idle", "loading", "ready")
    const val TEMPORARY_ITEM_ID = "header:temporary-chat"
    const val TEMPORARY_CHAT = "temporary_chat"
}

/**
 * Owns the unique native header entry for temporary chat and current-conversation settings.
 * Attachment, tool, model, project, and feature navigation remain on their dedicated surfaces.
 */
internal class WebChatProductionHeaderActionsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val currentSessionState: () -> String,
    private val currentConversationPath: () -> String?,
    private val openConversationSettings: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val interactionCache: WebChatProductionInteractionCache,
    private val onStateChanged: () -> Unit,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null
    private val temporaryChatIntent = WebChatTemporaryChatIntentQueue()

    fun render(button: ImageButton, provider: WebChatProviderIdentity, sessionState: String) {
        val port = consumerPort()
        val state = port?.let { cachedState(provider.id, it.state()) }
        val visible = state != null && WebChatProductionHeaderActionPolicy.visible(
            provider,
            sessionState,
            state.pageKind,
        )
        button.visibility = if (visible) View.VISIBLE else View.GONE
        val selected = if (visible) {
            WebChatProductionHeaderActionPolicy.resolve(
                state!!,
                currentConversationPath(),
            ).temporaryChat?.selected
        } else {
            null
        }
        val presentation = WebChatProductionHeaderActionPolicy.buttonPresentation(selected)
        button.setImageResource(presentation.iconRes)
        button.isSelected = presentation.selected
        button.imageTintList = ColorStateList.valueOf(activity.getColor(
            if (presentation.selected) R.color.elon_accent_primary else R.color.elon_text_primary,
        ))
        button.tooltipText = "聊天设置 · ${presentation.statusLabel}"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            button.stateDescription = presentation.statusLabel
        }
    }

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        val port = consumerPort() ?: return
        val state = cachedState(provider.id, port.state())
        if (!WebChatProductionHeaderActionPolicy.visible(
                provider,
                sessionState = currentSessionState(),
                pageKind = state.pageKind,
            )
        ) return

        val epoch = requestEpoch
        present(provider, port, state)
        if (WebChatProductionHeaderActionPolicy.resolve(
                state,
                currentConversationPath(),
            ).temporaryChat == null
        ) {
            val requested = port.requestControls()
            if (requested.accepted) pollControls(provider, port, epoch, attempt = 0)
        }
    }

    fun cancelPending() {
        requestEpoch += 1
        temporaryChatIntent.clear()
        activeSheet?.dismiss()
        activeSheet = null
    }

    private fun present(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        state: WebChatConsumerState,
        observation: WebChatProductionObservationState = WebChatProductionObservationState.SYNCING,
    ) {
        val resolved = WebChatProductionHeaderActionPolicy.resolve(
            state,
            currentConversationPath(),
        )
        val items = buildList {
            add(WebChatProductionHeaderActionPolicy.temporaryChatItem(
                resolved.temporaryChat,
                observation,
            ))
            if (resolved.conversationSettingsAvailable) {
                add(WebChatActionSheetItem(
                    id = CONVERSATION_SETTINGS_ITEM_ID,
                    title = "会话设置",
                    subtitle = "分享、添加到项目、重命名、置顶、归档或删除",
                    contentDescription = WebChatProductionSelectors.pageActions(provider.id),
                ))
            }
        }
        activeSheet?.let { sheet ->
            sheet.updateItems(items)
            return
        }
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "聊天设置",
            items = items,
            footerActions = listOf(WebChatActionSheetFooterAction(
                label = "打开官网",
                contentDescription = "web-chat-header-actions-official:${provider.id.wireValue}",
                action = openOfficialFallback,
            )),
            onCancelled = {
                requestEpoch += 1
                temporaryChatIntent.clear()
            },
            onDismissed = { activeSheet = null },
        ) { item ->
            when (item.id) {
                WebChatProductionHeaderActionPolicy.TEMPORARY_ITEM_ID ->
                    toggleTemporaryChat(provider, port)
                CONVERSATION_SETTINGS_ITEM_ID -> openConversationSettings()
            }
        }
    }

    private fun toggleTemporaryChat(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
    ) {
        val control = WebChatProductionHeaderActionPolicy.resolve(
            cachedState(provider.id, port.state()),
            currentConversationPath(),
        ).temporaryChat
        val desiredSelected = control?.selected?.not() ?: true
        if (!temporaryChatIntent.begin(desiredSelected)) {
            Toast.makeText(activity, "临时聊天正在切换", Toast.LENGTH_SHORT).show()
            return
        }
        val epoch = ++requestEpoch
        port.requestControls()
        pollTemporaryChatIntent(provider, port, epoch, attempt = 0)
    }

    private fun pollControls(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(provider.id, epoch)) return
        val state = cachedState(provider.id, port.state())
        if (WebChatProductionHeaderActionPolicy.resolve(
                state,
                currentConversationPath(),
            ).temporaryChat != null
        ) {
            present(provider, port, state, WebChatProductionObservationState.AVAILABLE)
            onStateChanged()
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) {
            present(provider, port, state, WebChatProductionObservationState.TEMPORARILY_UNOBSERVED)
            return
        }
        host.postDelayed({ pollControls(provider, port, epoch, attempt + 1) }, POLL_INTERVAL_MS)
    }

    private fun pollTemporaryChatIntent(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(provider.id, epoch)) return
        val state = cachedState(provider.id, port.state())
        val control = WebChatProductionHeaderActionPolicy.resolve(
            state,
            currentConversationPath(),
        ).temporaryChat
        when (val decision = temporaryChatIntent.evaluate(control)) {
            WebChatTemporaryChatIntentDecision.Idle -> return
            WebChatTemporaryChatIntentDecision.AwaitingControl,
            WebChatTemporaryChatIntentDecision.AwaitingConfirmation -> {
                if (attempt >= MAX_INTENT_POLL_ATTEMPTS) {
                    temporaryChatIntent.clear()
                    onStateChanged()
                    Toast.makeText(
                        activity,
                        "临时聊天状态尚未确认，请稍后重试",
                        Toast.LENGTH_SHORT,
                    ).show()
                    return
                }
                if (attempt % CONTROL_REFRESH_INTERVAL == 0) port.requestControls()
                host.postDelayed(
                    { pollTemporaryChatIntent(provider, port, epoch, attempt + 1) },
                    POLL_INTERVAL_MS,
                )
            }
            is WebChatTemporaryChatIntentDecision.Apply -> {
                val result = port.updateControl(
                    decision.controlId,
                    WebChatConsumerControlMutation.Selected(decision.selected),
                )
                if (!result.accepted) {
                    temporaryChatIntent.mutationRejected(decision.controlId)
                    if (result.error !in RECOVERABLE_CONTROL_ERRORS) {
                        temporaryChatIntent.clear()
                        Toast.makeText(activity, errorMessage(result.error), Toast.LENGTH_SHORT).show()
                        return
                    }
                }
                port.requestControls()
                host.postDelayed(
                    { pollTemporaryChatIntent(provider, port, epoch, attempt + 1) },
                    POLL_INTERVAL_MS,
                )
            }
            is WebChatTemporaryChatIntentDecision.Confirmed -> {
                temporaryChatIntent.clear()
                present(provider, port, state, WebChatProductionObservationState.AVAILABLE)
                onStateChanged()
                Toast.makeText(
                    activity,
                    if (decision.selected) "临时聊天已开启" else "临时聊天已关闭",
                    Toast.LENGTH_SHORT,
                ).show()
            }
        }
    }

    private fun cachedState(
        providerId: WebChatProviderId,
        state: WebChatConsumerState,
    ): WebChatConsumerState = state.copy(
        controls = interactionCache.controls(
            providerId,
            state.copy(controls = state.controls.take(MAX_CONTROL_COUNT)),
        ),
    )

    private fun isCurrent(providerId: WebChatProviderId, epoch: Int): Boolean =
        requestEpoch == epoch && activeProvider() == providerId

    private fun errorMessage(error: String?): String = when (error) {
        "stale_control_id" -> "官网控件已变化，请重试"
        "control_state_not_settable" -> "临时聊天状态暂时不能修改"
        "bridge_not_ready", "adapter_not_current" -> "网页正在恢复，请稍后重试"
        else -> "临时聊天切换失败，请重试"
    }

    private companion object {
        const val CONVERSATION_SETTINGS_ITEM_ID = "header:conversation-settings"
        const val MAX_CONTROL_COUNT = 80
        const val MAX_POLL_ATTEMPTS = 8
        const val MAX_INTENT_POLL_ATTEMPTS = 16
        const val CONTROL_REFRESH_INTERVAL = 4
        const val POLL_INTERVAL_MS = 250L
        val RECOVERABLE_CONTROL_ERRORS = setOf(
            "stale_control_id",
            "bridge_not_ready",
            "adapter_not_current",
        )
    }
}
