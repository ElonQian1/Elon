package com.elon.app

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal enum class WebChatProviderSwitchDecision {
    ALREADY_ACTIVE,
    SWITCH_NOW,
    CONFIRM_ATTACHMENT_DISCARD,
}

internal object WebChatProviderSwitchPolicy {
    fun resolve(
        currentProvider: WebChatProviderId,
        targetProvider: WebChatProviderId,
        chatModeActive: Boolean,
        pendingAttachmentCount: Int,
    ): WebChatProviderSwitchDecision = when {
        chatModeActive && currentProvider == targetProvider ->
            WebChatProviderSwitchDecision.ALREADY_ACTIVE
        chatModeActive && pendingAttachmentCount > 0 ->
            WebChatProviderSwitchDecision.CONFIRM_ATTACHMENT_DISCARD
        else -> WebChatProviderSwitchDecision.SWITCH_NOW
    }
}

internal class WebChatProviderSwitchCoordinator(
    private val activity: AppCompatActivity,
    private val currentProvider: () -> WebChatProviderId,
    private val chatModeActive: () -> Boolean,
    private val pendingAttachmentCount: () -> Int,
    private val prepareInputHandoff: (discardPendingAttachments: Boolean) -> Unit,
    private val commitProvider: (WebChatProviderId) -> Boolean,
) {
    fun requestFromConsumer(targetProvider: WebChatProviderId): Boolean {
        val pendingCount = pendingAttachmentCount()
        return when (WebChatProviderSwitchPolicy.resolve(
            currentProvider = currentProvider(),
            targetProvider = targetProvider,
            chatModeActive = chatModeActive(),
            pendingAttachmentCount = pendingCount,
        )) {
            WebChatProviderSwitchDecision.ALREADY_ACTIVE -> true
            WebChatProviderSwitchDecision.SWITCH_NOW -> commit(targetProvider, discardAttachments = false)
            WebChatProviderSwitchDecision.CONFIRM_ATTACHMENT_DISCARD -> {
                showAttachmentDiscardConfirmation(targetProvider, pendingCount)
                false
            }
        }
    }

    fun selectWithoutPrompt(targetProvider: WebChatProviderId): Boolean {
        return when (WebChatProviderSwitchPolicy.resolve(
            currentProvider = currentProvider(),
            targetProvider = targetProvider,
            chatModeActive = chatModeActive(),
            pendingAttachmentCount = pendingAttachmentCount(),
        )) {
            WebChatProviderSwitchDecision.ALREADY_ACTIVE -> true
            WebChatProviderSwitchDecision.SWITCH_NOW -> commit(targetProvider, discardAttachments = false)
            WebChatProviderSwitchDecision.CONFIRM_ATTACHMENT_DISCARD -> false
        }
    }

    private fun showAttachmentDiscardConfirmation(targetProvider: WebChatProviderId, pendingCount: Int) {
        if (activity.isFinishing || activity.isDestroyed) return
        val providerName = WebChatProviderRegistry.get(targetProvider).displayName
        AlertDialog.Builder(activity)
            .setTitle(R.string.web_chat_provider_switch_title)
            .setMessage(activity.getString(
                R.string.web_chat_provider_switch_attachment_message,
                pendingCount,
                providerName,
            ))
            .setNegativeButton(android.R.string.cancel, null)
            .setPositiveButton(R.string.web_chat_provider_switch_discard) { _, _ ->
                commit(targetProvider, discardAttachments = true)
            }
            .show()
    }

    private fun commit(targetProvider: WebChatProviderId, discardAttachments: Boolean): Boolean {
        prepareInputHandoff(discardAttachments)
        return commitProvider(targetProvider)
    }
}
