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

internal object WebChatProductionHeaderActionPolicy {
    fun visible(
        provider: WebChatProviderIdentity,
        sessionState: String,
        pageKind: String,
    ): Boolean = provider.id == WebChatProviderId.CHATGPT_WEB &&
        provider.supports(WebChatProviderCapability.PAGE_ACTIONS) &&
        sessionState == "ready" &&
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

    private val CHAT_PAGE_KINDS = setOf("home", "conversation")
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

    fun render(button: ImageButton, provider: WebChatProviderIdentity, sessionState: String) {
        val port = consumerPort()
        val state = port?.let { cachedState(provider.id, it.state()) }
        val visible = state != null && WebChatProductionHeaderActionPolicy.visible(
            provider,
            sessionState,
            state.pageKind,
        )
        button.visibility = if (visible) View.VISIBLE else View.GONE
        val selected = visible && WebChatProductionHeaderActionPolicy
            .resolve(state!!, currentConversationPath())
            .temporaryChatSelected
        button.isSelected = selected
        button.imageTintList = ColorStateList.valueOf(activity.getColor(
            if (selected) R.color.elon_accent_primary else R.color.elon_text_primary,
        ))
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            button.stateDescription = if (selected) "临时聊天已开启" else "临时聊天未开启"
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
            resolved.temporaryChat?.let { control ->
                add(WebChatActionSheetItem(
                    id = TEMPORARY_ITEM_ID,
                    title = if (control.selected) "关闭临时聊天" else "临时聊天",
                    subtitle = if (control.selected) {
                        "已开启，本次对话不会出现在历史记录中"
                    } else {
                        "开启后，本次对话不会出现在历史记录中"
                    },
                    selected = control.selected,
                    contentDescription = ChatGptNativeNavigationSelector.TEMPORARY_CHAT,
                ))
            } ?: add(WebChatProductionInteractionPlaceholder.item(
                provider.id,
                surface = "temporary-chat",
                title = "正在同步临时聊天状态",
                state = observation,
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
            onCancelled = { requestEpoch += 1 },
            onDismissed = { activeSheet = null },
        ) { item ->
            when (item.id) {
                TEMPORARY_ITEM_ID -> toggleTemporaryChat(provider, port, resolved)
                CONVERSATION_SETTINGS_ITEM_ID -> openConversationSettings()
            }
        }
    }

    private fun toggleTemporaryChat(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        state: WebChatProductionHeaderActionState,
    ) {
        val control = state.temporaryChat ?: return
        val desiredSelected = !control.selected
        val result = port.updateControl(
            control.id,
            WebChatConsumerControlMutation.Selected(desiredSelected),
        )
        if (!result.accepted) {
            Toast.makeText(activity, errorMessage(result.error), Toast.LENGTH_SHORT).show()
            return
        }
        val epoch = ++requestEpoch
        port.requestControls()
        pollSelectedState(provider, port, desiredSelected, epoch, attempt = 0)
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

    private fun pollSelectedState(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        desiredSelected: Boolean,
        epoch: Int,
        attempt: Int,
    ) {
        if (!isCurrent(provider.id, epoch)) return
        val state = cachedState(provider.id, port.state())
        val observed = WebChatProductionHeaderActionPolicy.resolve(
            state,
            currentConversationPath(),
        ).temporaryChat
        if (observed?.selected == desiredSelected) {
            onStateChanged()
            Toast.makeText(
                activity,
                if (desiredSelected) "临时聊天已开启" else "临时聊天已关闭",
                Toast.LENGTH_SHORT,
            ).show()
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) {
            onStateChanged()
            Toast.makeText(activity, "临时聊天状态尚未确认，请稍后查看", Toast.LENGTH_SHORT).show()
            return
        }
        host.postDelayed(
            { pollSelectedState(provider, port, desiredSelected, epoch, attempt + 1) },
            POLL_INTERVAL_MS,
        )
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
        const val TEMPORARY_ITEM_ID = "header:temporary-chat"
        const val CONVERSATION_SETTINGS_ITEM_ID = "header:conversation-settings"
        const val MAX_CONTROL_COUNT = 80
        const val MAX_POLL_ATTEMPTS = 8
        const val POLL_INTERVAL_MS = 250L
    }
}
