package com.elon.app

import android.view.View
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState

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
    private var onWebChatNavigationChanged: () -> Unit = {}
    private val chatGptControllerDelegate = lazy {
        ChatGptSocialChatController(
            activity = activity,
            binding = binding,
            setChatAdapter = setChatAdapter,
            showMessageActions = showMessageActions,
            clearPendingSendState = clearPendingSendState,
            collapseInputComposer = collapseInputComposer,
            openOfficialFallback = { modeController.openOfficialFallback() },
            onConversationIndexChanged = { onWebChatNavigationChanged() },
        )
    }
    private val chatGptController by chatGptControllerDelegate
    private val googleControllerDelegate = lazy {
        GoogleWebSocialChatController(
            activity = activity,
            binding = binding,
            setChatAdapter = setChatAdapter,
            showMessageActions = showMessageActions,
            clearPendingSendState = clearPendingSendState,
            collapseInputComposer = collapseInputComposer,
            openOfficialFallback = { modeController.openOfficialFallback() },
            onConversationIndexChanged = { onWebChatNavigationChanged() },
        )
    }
    private val googleController by googleControllerDelegate
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
    private val webChatNavigationSessions: WebChatNavigationSessionRegistry by lazy {
        WebChatNavigationSessionRegistry(
            listOf(
                WebChatNavigationSession(
                    providerId = WebChatProviderId.CHATGPT_WEB,
                    capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION,
                    indexSource = chatGptController::conversationIndex,
                    refreshSource = chatGptController::requestConversationIndex,
                    newConversationSource = {
                        if (webChatState() != "ready") {
                            false
                        } else {
                            chatGptController.startNewConversation()
                            true
                        }
                    },
                    openConversationSource = chatGptController::openConversation,
                    openProjectSource = chatGptController::openProject,
                ),
                WebChatNavigationSession(
                    providerId = WebChatProviderId.GOOGLE_WEB,
                    capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION,
                    indexSource = googleController::conversationIndex,
                    refreshSource = googleController::requestConversationIndex,
                    newConversationSource = {
                        if (webChatState() != "ready") {
                            false
                        } else {
                            googleController.startNewConversation()
                            true
                        }
                    },
                    openConversationSource = googleController::openConversation,
                    openProjectSource = googleController::openProject,
                ),
            ),
        )
    }

    fun onFriendChanged(friend: AppFriend?) = modeController.onFriendChanged(friend)

    fun trySendMessage(text: String, attachments: List<PendingAttachment>): Boolean =
        activeController().trySendMessage(text, attachments)

    fun currentMessages(): List<ChatMessage> = activeController().currentMessages()

    fun openSocialAiChat(): Boolean = modeController.openSocialAiChat()

    fun openChatGptWeb() = modeController.openChatGptWeb()

    fun openOfficialFallback() = modeController.openOfficialFallback()

    fun interactionMode(): SocialAiInteractionMode = modeController.interactionMode()

    fun providerId(): WebChatProviderId = modeController.providerId()

    fun providerName(): String = WebChatProviderRegistry.get(providerId()).displayName

    fun isChatModeActive(): Boolean = modeController.isChatModeActive()

    fun webChatState(): String = activeController().stateWireValue()

    fun webChatModel(): String = activeController().currentModel()

    fun webChatAdapterVersion(): Int = activeController().adapterVersion()

    fun webChatAuthenticated(): Boolean = activeController().authenticated()

    fun webChatComposerReady(): Boolean = activeController().composerReady()

    fun webChatAttachmentSupported(): Boolean = activeController().attachmentSupported()

    fun webChatAttachmentPhase(): String = activeController().attachmentSendPhase()

    fun webChatPendingAttachmentCount(): Int = activeController().pendingAttachmentCount()

    fun webChatConversationPath(): String? = activeController().currentConversationPath()

    fun webChatConversationIndex(): ChatGptWebConversationIndexState =
        webChatNavigationSession()?.index() ?: ChatGptWebConversationIndexState()

    fun webChatNavigationAvailable(): Boolean = webChatNavigationSession() != null

    fun createWebChatSideMenuCoordinator(): com.elon.app.chatgptweb.ChatGptWebSideMenuCoordinator {
        lateinit var coordinator: com.elon.app.chatgptweb.ChatGptWebSideMenuCoordinator
        coordinator = com.elon.app.chatgptweb.ChatGptWebSideMenuCoordinator(
            activity = activity,
            index = ::webChatConversationIndex,
            refreshIndex = ::refreshWebChatConversationIndex,
            newConversation = { startNewWebChatConversation() },
            openConversation = ::openWebChatConversation,
            openProject = ::openWebChatProject,
            openOfficialFallback = ::openOfficialFallback,
            providerName = ::providerName,
            active = { isChatModeActive() && webChatNavigationAvailable() },
        )
        onWebChatNavigationChanged = coordinator::onIndexChanged
        return coordinator
    }

    fun refreshWebChatConversationIndex(): Boolean =
        isChatModeActive() && webChatNavigationSession()?.refresh() == true

    fun startNewWebChatConversation(): Boolean {
        if (!isChatModeActive()) return false
        return webChatNavigationSession()?.newConversation() == true
    }

    fun openWebChatConversation(path: String): Boolean =
        isChatModeActive() &&
            webChatNavigationSession()?.openConversation(path) == true

    fun openWebChatProject(path: String): Boolean =
        isChatModeActive() &&
            webChatNavigationSession()?.openProject(path) == true

    fun discardWebChatAcceptanceAttachmentSend(): Boolean =
        activeController().discardAcceptanceAttachmentSend()

    fun selectInteractionMode(value: String): Boolean {
        val mode = SocialAiInteractionMode.parse(value) ?: return false
        return modeController.selectInteractionMode(mode)
    }

    fun selectProvider(value: String): Boolean {
        val id = WebChatProviderId.fromWireValue(value)
        return value == id.wireValue && modeController.selectChatProvider(id)
    }

    fun onHostResumed(resumeWorkChat: () -> Unit) {
        if (isChatModeActive()) activeController().onHostResumed() else resumeWorkChat()
    }

    fun onHostPaused() {
        if (chatGptControllerDelegate.isInitialized()) chatGptController.onHostPaused()
        if (googleControllerDelegate.isInitialized()) googleController.onHostPaused()
    }

    fun destroy() {
        if (chatGptControllerDelegate.isInitialized()) chatGptController.destroy()
        if (googleControllerDelegate.isInitialized()) googleController.destroy()
    }

    private fun activateWorkMode() {
        deactivateChatProvider()
        rebindWorkFriend()
    }

    private fun deactivateChatProvider() {
        if (chatGptControllerDelegate.isInitialized()) chatGptController.deactivate()
        if (googleControllerDelegate.isInitialized()) googleController.deactivate()
        binding.modelButton.tag = null
        inputComposerViews()?.let { views ->
            views.modelButtonShell.tag = null
            views.modelButtonShell.layoutParams = views.modelButtonShell.layoutParams.apply {
                width = dp(MODEL_BUTTON_WORK_WIDTH_DP)
            }
            views.planModeButton.visibility = View.VISIBLE
            views.modelButtonShell.setOnClickListener { showWorkModelSelector() }
            binding.modelButton.setOnClickListener { showWorkModelSelector() }
        }
        updateWorkModel()
    }

    private fun activateChatProvider(provider: WebChatProviderIdentity) {
        suspendWorkFriend()
        binding.modelButton.tag = WEB_CHAT_MODEL_BUTTON_OWNER
        if (chatGptControllerDelegate.isInitialized()) chatGptController.deactivate()
        if (googleControllerDelegate.isInitialized()) googleController.deactivate()
        val controller = controllerFor(provider.id)
        controller.activate(provider)
        inputComposerViews()?.let { views ->
            views.modelButtonShell.tag = WEB_CHAT_MODEL_BUTTON_OWNER
            views.modelButtonShell.layoutParams = views.modelButtonShell.layoutParams.apply {
                width = dp(MODEL_BUTTON_CHAT_WIDTH_DP)
            }
            views.planModeButton.visibility = View.GONE
            views.modelButtonShell.setOnClickListener { activeController().requestModelOptions() }
            binding.modelButton.setOnClickListener { activeController().requestModelOptions() }
        }
        binding.root.post { controller.refreshComposerModel() }
    }

    private fun activeController(): WebChatSocialController = controllerFor(providerId())

    private fun controllerFor(id: WebChatProviderId): WebChatSocialController = when (id) {
        WebChatProviderId.CHATGPT_WEB -> chatGptController
        WebChatProviderId.GOOGLE_WEB -> googleController
    }

    private fun webChatNavigationSession(): WebChatNavigationSession? =
        webChatNavigationSessions.session(providerId())

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val MODEL_BUTTON_WORK_WIDTH_DP = 76
        const val MODEL_BUTTON_CHAT_WIDTH_DP = 144
    }
}

internal const val WEB_CHAT_MODEL_BUTTON_OWNER = "web_chat_model_button"
