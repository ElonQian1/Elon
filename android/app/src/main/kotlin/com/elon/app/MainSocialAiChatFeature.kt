package com.elon.app

import android.view.View
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainSocialAiChatFeature(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    findSocialAiFriend: () -> AppFriend?,
    closeGroupChat: () -> Unit,
    closeProjectChat: () -> Unit,
    openFriend: (AppFriend) -> Unit,
    onFriendOpened: () -> Unit,
    private val rebindWorkFriend: () -> Unit,
    private val suspendWorkFriend: () -> Unit,
    setChatAdapter: (ChatAdapter) -> Unit,
    showMessageActions: (View, ChatMessage) -> Unit,
    clearPendingSendState: () -> Unit,
    collapseInputComposer: () -> Unit,
    private val inputComposerViews: () -> MainInputComposerViews?,
    private val showWorkModelSelector: () -> Unit,
    private val updateWorkModel: () -> Unit,
) {
    private val webChatController: ChatGptSocialChatController by lazy {
        ChatGptSocialChatController(
            activity = activity,
            binding = binding,
            setChatAdapter = setChatAdapter,
            showMessageActions = showMessageActions,
            clearPendingSendState = clearPendingSendState,
            collapseInputComposer = collapseInputComposer,
            openOfficialFallback = { modeController.openOfficialFallback() },
        )
    }
    private val modeController: SocialAiChatModeController by lazy {
        SocialAiChatModeController(
            activity = activity,
            binding = binding,
            findSocialAiFriend = findSocialAiFriend,
            closeGroupChat = closeGroupChat,
            closeProjectChat = closeProjectChat,
            openFriend = openFriend,
            onFriendOpened = onFriendOpened,
            activateWorkMode = ::activateWorkMode,
            activateChatProvider = ::activateChatProvider,
            deactivateChatProvider = ::deactivateChatProvider,
        )
    }

    fun onFriendChanged(friend: AppFriend?) = modeController.onFriendChanged(friend)

    fun trySendMessage(text: String, attachments: List<PendingAttachment>): Boolean =
        webChatController.trySendMessage(text, attachments)

    fun currentMessages(): List<ChatMessage> = webChatController.currentMessages()

    fun openSocialAiChat(): Boolean = modeController.openSocialAiChat()

    fun openChatGptWeb() = modeController.openChatGptWeb()

    fun openOfficialFallback() = modeController.openOfficialFallback()

    fun interactionMode(): SocialAiInteractionMode = modeController.interactionMode()

    fun providerId(): WebChatProviderId = modeController.providerId()

    fun providerName(): String = WebChatProviderRegistry.get(providerId()).displayName

    fun isChatModeActive(): Boolean = modeController.isChatModeActive()

    fun webChatState(): String = webChatController.stateWireValue()

    fun webChatModel(): String = webChatController.currentModel()

    fun webChatAttachmentPhase(): String = webChatController.attachmentSendPhase()

    fun webChatPendingAttachmentCount(): Int = webChatController.pendingAttachmentCount()

    fun selectInteractionMode(value: String): Boolean =
        modeController.selectInteractionMode(SocialAiInteractionMode.fromWireValue(value))

    fun selectProvider(value: String): Boolean {
        val id = WebChatProviderId.fromWireValue(value)
        return value == id.wireValue && modeController.selectChatProvider(id)
    }

    fun onHostResumed(resumeWorkChat: () -> Unit) {
        if (isChatModeActive()) webChatController.onHostResumed() else resumeWorkChat()
    }

    fun onHostPaused() = webChatController.onHostPaused()

    fun destroy() = webChatController.destroy()

    private fun activateWorkMode() {
        deactivateChatProvider()
        rebindWorkFriend()
    }

    private fun deactivateChatProvider() {
        webChatController.deactivate()
        inputComposerViews()?.let { views ->
            views.planModeButton.visibility = View.VISIBLE
            views.modelButtonShell.setOnClickListener { showWorkModelSelector() }
            binding.modelButton.setOnClickListener { showWorkModelSelector() }
        }
        updateWorkModel()
    }

    private fun activateChatProvider(provider: WebChatProviderIdentity) {
        suspendWorkFriend()
        webChatController.activate(provider)
        inputComposerViews()?.let { views ->
            views.planModeButton.visibility = View.GONE
            views.modelButtonShell.setOnClickListener { webChatController.requestModelOptions() }
            binding.modelButton.setOnClickListener { webChatController.requestModelOptions() }
        }
    }
}
