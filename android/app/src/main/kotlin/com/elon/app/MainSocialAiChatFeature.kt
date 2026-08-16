package com.elon.app

import android.view.View
import android.view.ViewGroup
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
    private val refreshInputComposerVisual: () -> Unit,
    private val chatGptWebLifecycle: MainChatGptWebLifecycle,
) {
    private var onWebChatNavigationChanged: () -> Unit = {}
    private val providerDraftStore = WebChatProviderDraftStore(activity)
    private val providerDrafts = providerDraftStore.restore()
    private val persistProviderDrafts = Runnable { providerDraftStore.save(providerDrafts) }
    private val composerDrafts = SocialAiComposerDraftCoordinator(
        providerDrafts = providerDrafts,
        readText = { binding.inputEdit.text },
        writeText = { draft ->
            binding.inputEdit.setText(draft)
            binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
        },
        onProviderDraftChanged = ::scheduleProviderDraftSave,
    )
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
            onComposerStateChanged = ::refreshConsumerComposerUi,
            audioPermissionController = chatGptWebLifecycle.audioPermissionController,
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
            onComposerStateChanged = ::refreshConsumerComposerUi,
        )
    }
    private val googleController by googleControllerDelegate
    private val providerPicker by lazy {
        WebChatProviderPicker(
            activity = activity,
            currentProvider = ::providerId,
            currentModel = ::webChatModel,
            currentState = ::webChatState,
            authenticated = ::webChatAuthenticated,
            composerReady = ::webChatComposerReady,
            selectProvider = ::selectChatProvider,
            requestModelOptions = { activeController().requestModelOptions() },
            openOfficialFallback = ::openOfficialFallback,
        )
    }
    private val consumerStatusBannerDelegate = lazy {
        WebChatConsumerStatusBanner(
            activity = activity,
            onRetry = {
                if (isChatModeActive()) {
                    activeController().onHostResumed()
                    refreshConsumerComposerUi()
                }
            },
            onOfficialPage = ::openOfficialFallback,
        )
    }
    private val consumerStatusBanner by consumerStatusBannerDelegate
    private val productionComposerToolsDelegate = lazy {
        WebChatProductionComposerToolsCoordinator(
            activity = activity,
            host = binding.root,
            mcpPort = {
                if (isChatModeActive()) activeController().mcpPort() else null
            },
            activeProvider = {
                if (isChatModeActive()) providerId() else null
            },
            openOfficialFallback = ::openOfficialFallback,
        )
    }
    private val productionComposerTools by productionComposerToolsDelegate
    private val productionFeatureNavigationDelegate = lazy {
        WebChatProductionFeatureNavigationCoordinator(
            activity = activity,
            host = binding.root,
            mcpPort = ::chatGptMcpPort,
            activeProvider = {
                if (isChatModeActive()) providerId() else null
            },
            openOfficialFallback = ::openOfficialFallback,
        )
    }
    private val productionFeatureNavigation by productionFeatureNavigationDelegate
    private val productionPageActionsDelegate = lazy {
        WebChatProductionPageActionsCoordinator(
            activity = activity,
            host = binding.root,
            mcpPort = ::chatGptMcpPort,
            activeProvider = {
                if (isChatModeActive()) providerId() else null
            },
            openOfficialFallback = ::openOfficialFallback,
        )
    }
    private val productionPageActions by productionPageActionsDelegate
    private val productionConversationActions by lazy {
        WebChatProductionConversationActionsCoordinator(
            activity, binding.root,
            activeProvider = { providerId().takeIf { isChatModeActive() } },
            currentConversationPath = { activeController().currentConversationPath() },
            currentState = { activeController().stateWireValue() },
            openConversation = ::openWebChatConversation,
            showPageActions = { productionPageActions.show(WebChatProviderRegistry.get(providerId())) },
            openOfficialFallback = ::openOfficialFallback,
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
            deactivateChatProvider = { deactivateChatProvider() },
            officialFallbackUrl = { activeController().officialFallbackUrl() },
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

    fun startWebChatRealtimeVoice(): Boolean =
        productionComposerTools.startRealtimeVoice(WebChatProviderRegistry.get(providerId()))

    fun interactionMode(): SocialAiInteractionMode = modeController.interactionMode()

    fun providerId(): WebChatProviderId = modeController.providerId()

    fun providerName(): String = WebChatProviderRegistry.get(providerId()).displayName

    fun webChatProductionCapabilities(): org.json.JSONObject =
        WebChatProductionCapabilityContract.describe(WebChatProviderRegistry.get(providerId()))

    fun isChatModeActive(): Boolean = modeController.isChatModeActive()

    fun webChatState(): String = activeController().stateWireValue()

    fun webChatModel(): String = activeController().currentModel()

    fun webChatAdapterVersion(): Int = activeController().adapterVersion()

    fun webChatAuthenticated(): Boolean = activeController().authenticated()

    fun webChatComposerReady(): Boolean = activeController().composerReady()

    fun webChatComposerCanSubmit(): Boolean {
        if (!isChatModeActive()) return true
        val controller = activeController()
        return WebChatConsumerComposerStateResolver.resolve(
            provider = WebChatProviderRegistry.get(providerId()),
            state = controller.stateWireValue(),
            composerReady = controller.composerReady(),
            attachmentSupported = controller.attachmentSupported(),
        ).submissionEnabled
    }

    fun onComposerTextChanged(value: CharSequence?) = composerDrafts.onTextChanged(value)

    fun webChatStreaming(): Boolean = isChatModeActive() && activeController().streaming()

    fun webChatLastCommandStatus(): WebChatCommandStatus? =
        if (isChatModeActive()) activeController().lastCommandStatus() else null

    fun stopWebChatGeneration(): Boolean {
        if (!webChatStreaming()) return false
        activeController().stopGeneration()
        return true
    }

    fun webChatAttachmentSupported(): Boolean = activeController().attachmentSupported()

    fun webChatAttachmentPhase(): String = activeController().attachmentSendPhase()

    fun webChatPendingAttachmentCount(): Int = activeController().pendingAttachmentCount()

    fun webChatConversationPath(): String? = activeController().currentConversationPath()

    fun webChatConversationIndex(): ChatGptWebConversationIndexState =
        webChatNavigationSession()?.index() ?: ChatGptWebConversationIndexState()

    fun chatGptMcpPort(): WebChatSocialMcpPort? {
        if (!isChatModeActive()) return null
        return if (providerId() == WebChatProviderId.CHATGPT_WEB) chatGptController.mcpPort() else null
    }

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
            openFeatureNavigation = ::openProductionFeatureNavigation,
            providerId = { providerId().wireValue },
            providerName = ::providerName,
            localProjectActions = {
                activeController().takeIf(WebChatSocialController::supportsLocalProjects)?.let { controller ->
                    WebChatLocalProjectActions(
                        createProject = controller::createLocalProject,
                        assignConversation = controller::assignConversationToLocalProject,
                    )
                }
            },
            remoteConversationActionsAvailable = { providerId() == WebChatProviderId.CHATGPT_WEB },
            openRemoteConversationActions = productionConversationActions::show,
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

    private fun openProductionFeatureNavigation() {
        productionFeatureNavigation.show(WebChatProviderRegistry.get(providerId()))
    }

    fun selectInteractionMode(value: String): Boolean {
        val mode = SocialAiInteractionMode.parse(value) ?: return false
        return modeController.selectInteractionMode(mode)
    }

    fun selectProvider(value: String): Boolean {
        val id = WebChatProviderId.fromWireValue(value)
        return value == id.wireValue && selectChatProvider(id)
    }

    fun onHostResumed(resumeWorkChat: () -> Unit) {
        if (isChatModeActive()) activeController().onHostResumed() else resumeWorkChat()
    }

    fun onHostPaused() {
        composerDrafts.rememberCurrent()
        flushProviderDrafts()
        if (chatGptControllerDelegate.isInitialized()) chatGptController.onHostPaused()
        if (googleControllerDelegate.isInitialized()) googleController.onHostPaused()
    }

    fun destroy() {
        composerDrafts.rememberCurrent()
        flushProviderDrafts()
        if (chatGptControllerDelegate.isInitialized()) chatGptController.destroy()
        if (googleControllerDelegate.isInitialized()) googleController.destroy()
        chatGptWebLifecycle.dispose()
    }

    private fun activateWorkMode() {
        composerDrafts.activateWorkMode()
        deactivateChatProvider(releaseComposerDraft = false)
        rebindWorkFriend()
    }

    private fun deactivateChatProvider(releaseComposerDraft: Boolean = true) {
        if (releaseComposerDraft) composerDrafts.release()
        if (productionComposerToolsDelegate.isInitialized()) productionComposerTools.cancelPending()
        if (productionFeatureNavigationDelegate.isInitialized()) {
            productionFeatureNavigation.cancelPending()
        }
        if (productionPageActionsDelegate.isInitialized()) productionPageActions.cancelPending()
        if (chatGptControllerDelegate.isInitialized()) chatGptController.deactivate()
        if (googleControllerDelegate.isInitialized()) googleController.deactivate()
        if (consumerStatusBannerDelegate.isInitialized()) consumerStatusBanner.hide()
        binding.modelButton.tag = null
        WebChatComposerProviderPresentation.clear(binding.modelButton)
        binding.inputEdit.contentDescription = null
        binding.inputEdit.hint = "输入内容"
        inputComposerViews()?.let { views ->
            views.modelButtonShell.tag = null
            views.modelButtonShell.layoutParams = views.modelButtonShell.layoutParams.apply {
                width = dp(MODEL_BUTTON_WORK_WIDTH_DP)
            }
            views.planModeButton.visibility = View.VISIBLE
            views.webToolsButton.visibility = View.GONE
            views.webToolsButton.setOnClickListener(null)
            views.attachmentButton.visibility = View.VISIBLE
            views.attachmentButton.contentDescription = WebChatProductionSelectors.WORK_ATTACHMENT
            views.modelButtonShell.setOnClickListener { showWorkModelSelector() }
            binding.modelButton.setOnClickListener { showWorkModelSelector() }
        }
        updateWorkModel()
        refreshInputComposerVisual()
    }

    private fun activateChatProvider(provider: WebChatProviderIdentity) {
        suspendWorkFriend()
        composerDrafts.activateProvider(provider.id)
        if (productionComposerToolsDelegate.isInitialized()) productionComposerTools.cancelPending()
        if (productionFeatureNavigationDelegate.isInitialized()) {
            productionFeatureNavigation.cancelPending()
        }
        if (productionPageActionsDelegate.isInitialized()) productionPageActions.cancelPending()
        binding.modelButton.tag = WEB_CHAT_MODEL_BUTTON_OWNER
        if (chatGptControllerDelegate.isInitialized()) chatGptController.deactivate()
        if (googleControllerDelegate.isInitialized()) googleController.deactivate()
        val controller = controllerFor(provider.id)
        controller.activate(provider)
        ensureConsumerStatusBannerAttached()
        binding.inputEdit.contentDescription = WebChatProductionSelectors.composerInput(provider.id)
        binding.moreButton.apply {
            visibility = View.VISIBLE
            setImageResource(R.drawable.ic_more_horizontal)
            contentDescription = WebChatProductionSelectors.pageActions(provider.id)
            setOnClickListener { productionPageActions.show(provider) }
        }
        inputComposerViews()?.let { views ->
            views.modelButtonShell.tag = WEB_CHAT_MODEL_BUTTON_OWNER
            views.modelButtonShell.layoutParams = views.modelButtonShell.layoutParams.apply {
                width = dp(MODEL_BUTTON_CHAT_WIDTH_DP)
            }
            views.planModeButton.visibility = View.GONE
            views.webToolsButton.contentDescription = WebChatProductionSelectors.composerTools(provider.id)
            views.webToolsButton.setOnClickListener {
                productionComposerTools.show(provider)
            }
            views.attachmentButton.contentDescription = WebChatProductionSelectors.attachment(provider.id)
            views.modelButtonShell.setOnClickListener { providerPicker.show() }
            binding.modelButton.setOnClickListener { providerPicker.show() }
        }
        binding.root.post { controller.refreshComposerModel() }
        refreshConsumerComposerUi()
    }

    private fun refreshConsumerComposerUi() {
        if (isChatModeActive()) {
            val provider = WebChatProviderRegistry.get(providerId())
            val controller = activeController()
            val state = WebChatConsumerComposerStateResolver.resolve(
                provider = provider,
                state = controller.stateWireValue(),
                composerReady = controller.composerReady(),
                attachmentSupported = controller.attachmentSupported(),
            )
            binding.inputEdit.hint = state.inputHint
            consumerStatusBanner.render(
                WebChatConsumerRecoveryPolicy.resolve(provider, controller.stateWireValue()),
            )
            inputComposerViews()?.let { views ->
                views.attachmentButton.visibility = if (state.attachmentVisible) View.VISIBLE else View.GONE
                views.webToolsButton.visibility = if (state.toolsVisible) View.VISIBLE else View.GONE
            }
        } else if (consumerStatusBannerDelegate.isInitialized()) consumerStatusBanner.hide()
        refreshInputComposerVisual()
    }

    private fun ensureConsumerStatusBannerAttached() {
        val banner = consumerStatusBanner
        if (banner.parent === binding.inputLayout) return
        (banner.parent as? ViewGroup)?.removeView(banner)
        binding.inputLayout.addView(banner, 0)
    }

    private fun activeController(): WebChatSocialController = controllerFor(providerId())

    private fun selectChatProvider(id: WebChatProviderId): Boolean {
        if (providerId() == id && isChatModeActive()) return true
        return modeController.selectChatProvider(id)
    }

    private fun scheduleProviderDraftSave() {
        binding.root.removeCallbacks(persistProviderDrafts)
        binding.root.postDelayed(persistProviderDrafts, DRAFT_SAVE_DELAY_MS)
    }

    private fun flushProviderDrafts() {
        binding.root.removeCallbacks(persistProviderDrafts)
        providerDraftStore.save(providerDrafts)
    }

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
        const val DRAFT_SAVE_DELAY_MS = 500L
    }

}

internal const val WEB_CHAT_MODEL_BUTTON_OWNER = "web_chat_model_button"
