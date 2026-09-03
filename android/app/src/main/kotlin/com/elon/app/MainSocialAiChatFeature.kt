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
    pendingInputAttachmentCount: () -> Int,
    prepareInputForProviderSwitch: (Boolean) -> Unit, private val nativeDictation: WebChatNativeDictationPort,
    private val showWorkModelSelector: () -> Unit,
    private val updateWorkModel: () -> Unit,
    private val refreshInputComposerVisual: () -> Unit,
    private val chatGptWebLifecycle: MainChatGptWebLifecycle,
    private val serverUrl: () -> String,
    private val userId: () -> String,
) {
    private var onWebChatNavigationChanged: () -> Unit = {}
    private val webChatInteractionCache = WebChatProductionInteractionCache(
        storage = WebChatProductionInteractionSnapshotStore(activity),
    )
    private val realtimeVoiceLaunchCache =
        WebChatRealtimeVoiceLaunchCache(WebChatRealtimeVoiceLaunchSnapshotStore(activity))
    private val providerDraftStore = WebChatProviderDraftStore(activity)
    private val providerDrafts = providerDraftStore.restore()
    private val persistProviderDrafts = Runnable { providerDraftStore.save(providerDrafts) }
    private var composerOperationFeedback: WebChatConsumerComposerFeedback? = null
    private var composerOperationFeedbackEpoch = 0
    private var activeQuickComposerAction: WebChatProductionQuickComposerAction? = null
    private val projectMoveRecovery = WebChatProjectMoveRecoveryGate()
    private val composerDrafts = SocialAiComposerDraftCoordinator(
        providerDrafts = providerDrafts,
        readText = { binding.inputEdit.text },
        writeText = { draft ->
            binding.inputEdit.setText(draft)
            binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
        },
        onProviderDraftChanged = ::scheduleProviderDraftSave,
    )
    private val chatGptControllerDelegate: Lazy<ChatGptSocialChatController> = lazy {
        ChatGptSocialChatController(
            activity = activity,
            binding = binding,
            setChatAdapter = setChatAdapter,
            showMessageActions = showMessageActions,
            clearPendingSendState = clearPendingSendState,
            collapseInputComposer = collapseInputComposer,
            openProviderPicker = { providerPicker.show() },
            openOfficialFallback = { modeController.openOfficialFallback() },
            onCreateImageRequested = { productionImageActions.requestCreateImage() },
            onConversationIndexChanged = ::handleConversationIndexChanged,
            onComposerStateChanged = ::refreshConsumerComposerUi,
            onConsumerStateObserved = ::handleChatGptConsumerStateObserved,
            onDictationCommandResult = ::handleChatGptDictationCommandResult,
            interactionCache = webChatInteractionCache,
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
            onConversationIndexChanged = ::handleConversationIndexChanged,
            onComposerStateChanged = ::refreshConsumerComposerUi,
        )
    }
    private val googleController by googleControllerDelegate
    private val providerSwitchCoordinator by lazy {
        WebChatProviderSwitchCoordinator(
            activity = activity,
            currentProvider = ::providerId,
            chatModeActive = ::isChatModeActive,
            pendingAttachmentCount = pendingInputAttachmentCount,
            prepareInputHandoff = prepareInputForProviderSwitch,
            commitProvider = modeController::selectChatProvider,
        )
    }
    private val providerPicker by lazy {
        WebChatProviderPicker(
            activity = activity,
            currentProvider = ::providerId,
            currentModel = ::webChatModel,
            currentState = ::webChatState,
            authenticated = ::webChatAuthenticated,
            composerReady = ::webChatComposerReady,
            selectProvider = providerSwitchCoordinator::requestFromConsumer,
            requestModelOptions = {
                prioritizeConsumerInteraction()
                activeController().requestModelOptions()
            },
            openWebSkin = {
                prioritizeConsumerInteraction()
                activeController().showWebSkin()
            },
            openOfficialFallback = ::openOfficialFallback,
        )
    }
    private val consumerStatusBannerDelegate = lazy {
        WebChatConsumerStatusBanner(
            activity = activity,
            onRetry = {
                if (isChatModeActive()) {
                    retryConsumerSession()
                    refreshConsumerComposerUi()
                }
            },
            onOfficialPage = ::openOfficialFallback,
        )
    }
    private val consumerStatusBanner by consumerStatusBannerDelegate
    private val productionSuggestionsDelegate = lazy {
        WebChatProductionSuggestionsCoordinator(activity)
    }
    private val productionSuggestions by productionSuggestionsDelegate
    private val realtimeVoicesDelegate: Lazy<MainRealtimeVoiceTransports> = lazy {
        MainRealtimeVoiceTransports(
            activity = activity,
            controller = { chatGptController },
            webLifecycle = chatGptWebLifecycle,
            modeController = modeController,
            nativeRoot = binding.root,
            activeProvider = ::activeProviderOrNull,
            serverUrl = serverUrl,
            userId = userId,
            launchCache = realtimeVoiceLaunchCache,
        )
    }
    private val realtimeVoices by realtimeVoicesDelegate

    private fun handleChatGptConsumerStateObserved(state: WebChatConsumerState) {
        realtimeVoiceLaunchCache.observe(WebChatProviderId.CHATGPT_WEB, state)
        if (realtimeVoicesDelegate.isInitialized()) {
            realtimeVoices.onConsumerStateChanged(state)
        }
    }

    private fun handleChatGptDictationCommandResult(action: String, ok: Boolean) {
        if (productionVoiceControlsDelegate.isInitialized()) {
            productionVoiceControls.onDomCommandResult(action, ok)
        }
    }

    private val productionComposerToolsDelegate = lazy {
        WebChatProductionComposerToolsCoordinator(
            activity = activity,
            host = binding.root,
            consumerPort = ::activeConsumerPortOrNull,
            activeProvider = ::activeProviderOrNull,
            openOfficialFallback = ::openOfficialFallback,
            startWebRealtimeVoice = realtimeVoices::startDefaultOfficialWebRtc,
            onOperationFeedback = ::showComposerOperationFeedback,
            onQuickActionChanged = ::onQuickComposerActionChanged,
            interactionCache = webChatInteractionCache,
            sessionReady = {
                isChatModeActive() && activeController().stateWireValue() == "ready" &&
                    activeController().composerReady()
            },
            requestSessionRecovery = {
                if (isChatModeActive()) activeController().onHostResumed()
            },
        )
    }
    private val productionComposerTools by productionComposerToolsDelegate
    private val productionImageActions by lazy { ChatGptProductionImageActions(productionComposerTools, ::activeController) }
    private val productionVoiceControlsDelegate = lazy {
        WebChatProductionVoiceControls(
            dp = ::dp,
            inputComposerViews = inputComposerViews,
            executeCommand = productionComposerTools::executeCommand,
            privateDictation = chatGptController.privateDictationPort(),
            sharedDictation = nativeDictation,
            onNativeStateChanged = ::refreshConsumerComposerUi,
            prepareDictationCapture = collapseInputComposer,
            readDraft = { binding.inputEdit.text?.toString().orEmpty() },
            writeDraft = { binding.inputEdit.setText(it) },
        )
    }
    private val productionVoiceControls by productionVoiceControlsDelegate
    private val productionFeatureNavigationDelegate = lazy {
        WebChatProductionFeatureNavigationCoordinator(
            activity = activity,
            host = binding.root,
            consumerPort = ::chatGptConsumerPort,
            activeProvider = ::activeProviderOrNull,
            openOfficialFallback = ::openOfficialFallback,
            openNativeFeature = productionImageActions::openNativeFeature,
            interactionCache = webChatInteractionCache,
        )
    }
    private val productionFeatureNavigation by productionFeatureNavigationDelegate
    private val productionPageActionsDelegate = lazy {
        WebChatProductionPageActionsCoordinator(
            activity = activity,
            host = binding.root,
            consumerPort = ::chatGptConsumerPort,
            activeProvider = ::activeProviderOrNull,
            openOfficialFallback = ::openOfficialFallback,
            interactionCache = webChatInteractionCache,
        )
    }
    private val productionPageActions by productionPageActionsDelegate
    private val productionHeaderActionsDelegate = lazy {
        WebChatProductionHeaderActionsCoordinator(
            activity = activity,
            host = binding.root,
            consumerPort = ::chatGptConsumerPort,
            activeProvider = ::activeProviderOrNull,
            currentSessionState = ::activeSessionState,
            currentConversationPath = ::activeConversationPath,
            openConversationSettings = {
                prioritizeConsumerInteraction()
                productionPageActions.show(WebChatProviderRegistry.get(providerId()))
            },
            openOfficialFallback = ::openOfficialFallback,
            interactionCache = webChatInteractionCache,
            onStateChanged = ::refreshConsumerComposerUi,
        )
    }
    private val productionHeaderActions by productionHeaderActionsDelegate
    private val productionCapabilityPrewarmerDelegate = lazy {
        WebChatProductionCapabilityPrewarmer(
            consumerPort = ::activeConsumerPortOrNull,
            activeProvider = ::activeProviderOrNull,
            interactionCache = webChatInteractionCache,
            scheduleAction = { delayMs, action -> binding.root.postDelayed(action, delayMs) },
        )
    }
    private val productionCapabilityPrewarmer by productionCapabilityPrewarmerDelegate
    private val productionConversationActions by lazy {
        WebChatProductionConversationActionsCoordinator(
            activity, binding.root,
            activeProvider = ::activeProviderOrNull,
            currentConversationPath = { activeController().currentConversationPath() },
            currentState = { activeController().stateWireValue() },
            openConversation = ::openWebChatConversation,
            consumerPort = ::chatGptConsumerPort,
            conversationIndex = ::webChatConversationIndex,
            refreshConversationIndex = ::refreshWebChatConversationIndex,
            probeConversationProject = chatGptController::probeConversationProject,
            suspendConversationRefresh = chatGptController::suspendConversationRefreshForUserAction,
            resumeConversationRefresh = chatGptController::resumeConversationRefreshAfterUserAction,
            showPageActions = {
                prioritizeConsumerInteraction()
                productionPageActions.show(WebChatProviderRegistry.get(providerId()))
            },
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
    private val sessionPrewarmerDelegate = lazy {
        createWebChatSessionPrewarmCoordinator(binding.root, modeController, ::controllerFor)
    }
    private val sessionPrewarmer by sessionPrewarmerDelegate
    private val webChatNavigationSessions: WebChatNavigationSessionRegistry by lazy {
        WebChatNavigationSessionRegistry(
            listOf(
                WebChatNavigationSession(
                    providerId = WebChatProviderId.CHATGPT_WEB,
                    capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION,
                    indexSource = chatGptController::conversationIndex,
                    refreshSource = chatGptController::requestConversationIndex,
                    newConversationSource = chatGptController::startNewConversation,
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

    fun startDefaultRealtimeVoice(): Boolean {
        if (providerId() != WebChatProviderId.CHATGPT_WEB &&
            !providerSwitchCoordinator.selectWithoutPrompt(WebChatProviderId.CHATGPT_WEB)
        ) return false
        return productionComposerTools.startRealtimeVoice(
            WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
        )
    }

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

    fun webChatDictationActive(): Boolean = isChatModeActive() &&
        productionVoiceControls.dictationActive(
            runCatching { activeController().consumerPort()?.state() }.getOrNull(),
        )

    fun webChatComposerCanSubmit(): Boolean {
        if (!isChatModeActive()) return true
        val controller = activeController()
        return WebChatConsumerComposerStateResolver.resolve(
            provider = WebChatProviderRegistry.get(providerId()),
            state = controller.stateWireValue(),
            composerReady = controller.composerReady(),
            attachmentSupported = controller.attachmentSupported(),
            warmSessionAvailable = controller.warmSessionAvailable(),
        ).submissionEnabled
    }

    fun onComposerTextChanged(value: CharSequence?) = composerDrafts.onTextChanged(value)

    fun webChatStreaming(): Boolean = isChatModeActive() && activeController().streaming()

    fun webChatLastCommandStatus() = if (isChatModeActive()) activeController().lastCommandStatus() else null
    fun webChatLastSendCommandStatus() = if (isChatModeActive()) activeController().lastSendCommandStatus() else null

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

    fun chatGptConsumerPort(): WebChatConsumerPort? {
        if (!isChatModeActive()) return null
        return if (providerId() == WebChatProviderId.CHATGPT_WEB) {
            activeController().consumerPort()
        } else {
            null
        }
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
    fun refreshWebChatConversationIndex(
        projectId: String? = null,
        conversationPath: String? = null,
    ): Boolean = MainSocialAiChatNavigationPolicy.refreshIndex(
        isChatModeActive(), providerId(), projectId, conversationPath,
        chatGptController::probeConversationProject,
    ) { webChatNavigationSession()?.refresh(it) == true }
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
        prioritizeConsumerInteraction()
        productionFeatureNavigation.show(WebChatProviderRegistry.get(providerId()))
    }

    private fun prioritizeConsumerInteraction() {
        if (productionCapabilityPrewarmerDelegate.isInitialized()) {
            productionCapabilityPrewarmer.cancel()
        }
    }

    fun selectInteractionMode(value: String): Boolean {
        val mode = SocialAiInteractionMode.parse(value) ?: return false
        return modeController.selectInteractionMode(mode)
    }

    fun selectProvider(value: String): Boolean {
        val id = WebChatProviderId.fromWireValue(value)
        return value == id.wireValue && providerSwitchCoordinator.selectWithoutPrompt(id)
    }

    fun onHostResumed(resumeWorkChat: () -> Unit) {
        if (isChatModeActive()) {
            activeController().onHostResumed()
            productionCapabilityPrewarmer.schedule(WebChatProviderRegistry.get(providerId()))
        } else {
            resumeWorkChat()
        }
        if (realtimeVoicesDelegate.isInitialized()) realtimeVoices.onHostResumed()
        sessionPrewarmer.onHostResumed()
    }

    fun onHostPaused() {
        composerDrafts.rememberCurrent()
        flushProviderDrafts()
        if (productionCapabilityPrewarmerDelegate.isInitialized()) {
            productionCapabilityPrewarmer.cancel()
        }
        if (sessionPrewarmerDelegate.isInitialized()) sessionPrewarmer.cancel()
        if (realtimeVoicesDelegate.isInitialized()) realtimeVoices.onHostPaused()
        if (chatGptControllerDelegate.isInitialized()) chatGptController.onHostPaused()
        if (googleControllerDelegate.isInitialized()) googleController.onHostPaused()
    }

    fun destroy() {
        composerDrafts.rememberCurrent()
        flushProviderDrafts()
        if (productionCapabilityPrewarmerDelegate.isInitialized()) {
            productionCapabilityPrewarmer.cancel()
        }
        if (sessionPrewarmerDelegate.isInitialized()) sessionPrewarmer.cancel()
        if (realtimeVoicesDelegate.isInitialized()) realtimeVoices.destroy()
        if (chatGptControllerDelegate.isInitialized()) chatGptController.destroy()
        if (googleControllerDelegate.isInitialized()) googleController.destroy()
        chatGptWebLifecycle.dispose()
    }

    private fun activateWorkMode() {
        composerDrafts.activateWorkMode()
        deactivateChatProvider(releaseComposerDraft = false)
        renderToolbarVoiceAction(webChatModeActive = false)
        binding.moreButton.visibility = View.GONE
        rebindWorkFriend()
    }

    private fun deactivateChatProvider(releaseComposerDraft: Boolean = true) { nativeDictation.cancel()
        if (releaseComposerDraft) composerDrafts.release()
        clearComposerOperationFeedback()
        activeQuickComposerAction = null
        if (productionComposerToolsDelegate.isInitialized()) productionComposerTools.cancelPending()
        if (productionVoiceControlsDelegate.isInitialized()) {
            productionVoiceControls.restoreLocalVoiceInput()
        }
        if (productionFeatureNavigationDelegate.isInitialized()) {
            productionFeatureNavigation.cancelPending()
        }
        if (productionHeaderActionsDelegate.isInitialized()) productionHeaderActions.cancelPending()
        if (productionPageActionsDelegate.isInitialized()) productionPageActions.cancelPending()
        if (productionCapabilityPrewarmerDelegate.isInitialized()) {
            productionCapabilityPrewarmer.cancel()
        }
        if (chatGptControllerDelegate.isInitialized()) chatGptController.deactivate()
        if (googleControllerDelegate.isInitialized()) googleController.deactivate()
        if (consumerStatusBannerDelegate.isInitialized()) consumerStatusBanner.hide()
        if (productionSuggestionsDelegate.isInitialized()) productionSuggestions.hide()
        binding.modelButton.tag = null
        WebChatComposerProviderPresentation.clear(binding.modelButton)
        binding.inputEdit.contentDescription = null
        binding.inputEdit.hint = "输入内容"
        inputComposerViews()?.let { views ->
            views.activeWebToolChip.render(null, ::clearQuickComposerAction)
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
        if (realtimeVoicesDelegate.isInitialized()) realtimeVoices.onActiveSurfaceChanged()
    }

    private fun activateChatProvider(provider: WebChatProviderIdentity) {
        suspendWorkFriend()
        renderToolbarVoiceAction(webChatModeActive = true)
        clearComposerOperationFeedback()
        activeQuickComposerAction = null
        composerDrafts.activateProvider(provider.id)
        if (productionComposerToolsDelegate.isInitialized()) productionComposerTools.cancelPending()
        if (productionFeatureNavigationDelegate.isInitialized()) {
            productionFeatureNavigation.cancelPending()
        }
        if (productionHeaderActionsDelegate.isInitialized()) productionHeaderActions.cancelPending()
        if (productionPageActionsDelegate.isInitialized()) productionPageActions.cancelPending()
        binding.modelButton.tag = WEB_CHAT_MODEL_BUTTON_OWNER
        val controller = controllerFor(provider.id)
        if (!controller.isActive()) {
            if (chatGptControllerDelegate.isInitialized()) chatGptController.deactivate()
            if (googleControllerDelegate.isInitialized()) googleController.deactivate()
            controller.activate(provider)
        }
        ensureConsumerEnhancementsAttached()
        binding.inputEdit.contentDescription = WebChatProductionSelectors.composerInput(provider.id)
        binding.moreButton.apply {
            visibility = View.GONE
            setImageResource(R.drawable.ic_temporary_chat)
            contentDescription = WebChatProductionSelectors.pageActions(provider.id)
            setOnClickListener {
                prioritizeConsumerInteraction()
                productionHeaderActions.show(provider)
            }
        }
        inputComposerViews()?.let { views ->
            views.modelButtonShell.tag = WEB_CHAT_MODEL_BUTTON_OWNER
            views.modelButtonShell.layoutParams = views.modelButtonShell.layoutParams.apply {
                width = dp(
                    if (provider.id == WebChatProviderId.CHATGPT_WEB) MODEL_BUTTON_CHATGPT_WIDTH_DP else MODEL_BUTTON_CHAT_WIDTH_DP,
                )
            }
            views.planModeButton.visibility = View.GONE
            views.webToolsButton.contentDescription = WebChatProductionSelectors.composerTools(provider.id)
            views.webToolsButton.visibility = if (productionComposerTools.quickActions(provider).isEmpty()) {
                View.GONE
            } else {
                View.VISIBLE
            }
            views.webToolsButton.setOnClickListener {
                prioritizeConsumerInteraction()
                productionComposerTools.show(provider)
            }
            views.attachmentButton.contentDescription = WebChatProductionSelectors.attachment(provider.id)
            val showModelControl = {
                prioritizeConsumerInteraction()
                if (provider.id == WebChatProviderId.CHATGPT_WEB) {
                    controller.requestModelOptions()
                } else {
                    providerPicker.show()
                }
            }
            views.modelButtonShell.setOnClickListener { showModelControl() }
            binding.modelButton.setOnClickListener { showModelControl() }
        }
        binding.root.post { controller.refreshComposerModel() }
        refreshConsumerComposerUi()
        productionCapabilityPrewarmer.schedule(provider)
        if (realtimeVoicesDelegate.isInitialized()) realtimeVoices.onActiveSurfaceChanged()
    }

    private fun renderToolbarVoiceAction(webChatModeActive: Boolean) {
        val showVoiceCall = SocialAiToolbarActionPolicy.showVoiceCall(
            directSocialAiChatActive = true,
            webChatModeActive = webChatModeActive,
        )
        binding.voiceCallButton.visibility = if (showVoiceCall) View.VISIBLE else View.GONE
    }

    private fun refreshConsumerComposerUi() {
        if (isChatModeActive()) {
            val provider = WebChatProviderRegistry.get(providerId())
            val controller = activeController()
            projectMoveRecovery.observe(provider.id, controller.stateWireValue()) {
                binding.root.post { productionConversationActions.recoverPending() }
            }
            controller.consumerPort()?.state()?.let { realtimeVoiceLaunchCache.observe(provider.id, it) }
            productionHeaderActions.render(binding.moreButton, provider, controller.stateWireValue())
            val state = WebChatConsumerComposerStateResolver.resolve(
                provider = provider,
                state = controller.stateWireValue(),
                composerReady = controller.composerReady(),
                attachmentSupported = controller.attachmentSupported(),
                warmSessionAvailable = controller.warmSessionAvailable(),
            )
            val consumerState = runCatching { controller.consumerPort()?.state() }.getOrNull()
            val officialDictationActive = consumerState?.dictationActive == true
            val officialDictationCaptureActive = consumerState?.dictationCaptureActive == true
            val dictationPresentation = productionVoiceControls.dictationPresentation(
                officialActive = officialDictationActive,
                officialCaptureActive = officialDictationCaptureActive,
            )
            binding.inputEdit.hint = WebChatProductionComposerContext.inputHint(
                dictationPresentation.inputHint ?: state.inputHint,
                WebChatProductionComposerContext.projectTitle(
                    webChatConversationIndex(),
                    controller.currentConversationPath(),
                ),
            )
            val recovery = controller.consumerRecoveryState(provider)
            consumerStatusBanner.render(
                if (recovery.visible) recovery else WebChatConsumerComposerOperationPolicy.resolve(
                    provider = provider,
                    attachmentPhase = controller.attachmentSendPhase(),
                    feedback = composerOperationFeedback,
                    dictationActive = dictationPresentation.active,
                    imageGenerationActive = activeQuickComposerAction ==
                        WebChatProductionQuickComposerAction.IMAGE_GENERATION,
                    streaming = controller.streaming(),
                    imagePreviewState = controller.imagePreviewState(),
                ),
            )
            inputComposerViews()?.let { views ->
                productionComposerTools.selectedQuickAction(provider)?.let {
                    activeQuickComposerAction = it
                }
                views.attachmentButton.visibility = if (state.attachmentVisible) View.VISIBLE else View.GONE
                views.webToolsButton.visibility = if (productionComposerTools.quickActions(provider).isEmpty()) {
                    View.GONE
                } else {
                    View.VISIBLE
                }
                views.activeWebToolChip.render(activeQuickComposerAction, ::clearQuickComposerAction)
            }
            productionVoiceControls.render(
                provider = provider,
                streaming = controller.streaming(),
                officialDictationActive = officialDictationActive,
                officialDictationCaptureActive = officialDictationCaptureActive,
            )
            productionComposerTools.onSessionStateChanged(provider)
            productionSuggestions.render(provider, controller.consumerPort())
            productionCapabilityPrewarmer.schedule(provider)
        } else if (consumerStatusBannerDelegate.isInitialized()) consumerStatusBanner.hide()
        refreshInputComposerVisual()
    }

    private fun showComposerOperationFeedback(feedback: WebChatConsumerComposerFeedback) {
        if (!isChatModeActive() || providerId() != feedback.providerId) return
        val epoch = ++composerOperationFeedbackEpoch
        composerOperationFeedback = feedback
        refreshConsumerComposerUi()
        binding.root.postDelayed({
            if (epoch != composerOperationFeedbackEpoch) return@postDelayed
            composerOperationFeedback = null
            refreshConsumerComposerUi()
        }, COMPOSER_FEEDBACK_DURATION_MS)
    }

    private fun clearComposerOperationFeedback() {
        composerOperationFeedbackEpoch += 1
        composerOperationFeedback = null
    }

    private fun onQuickComposerActionChanged(action: WebChatProductionQuickComposerAction?) {
        activeQuickComposerAction = action
        refreshConsumerComposerUi()
    }

    private fun handleConversationIndexChanged() {
        onWebChatNavigationChanged()
        if (isChatModeActive()) refreshConsumerComposerUi()
    }

    private fun clearQuickComposerAction(action: WebChatProductionQuickComposerAction) {
        if (!isChatModeActive() || activeQuickComposerAction != action) return
        activeQuickComposerAction = null
        inputComposerViews()?.activeWebToolChip?.render(null, ::clearQuickComposerAction)
        if (!productionComposerTools.clearQuickAction(
                WebChatProviderRegistry.get(providerId()),
                action,
            )
        ) {
            activeQuickComposerAction = action
            refreshConsumerComposerUi()
        }
    }

    private fun ensureConsumerEnhancementsAttached() {
        val banner = consumerStatusBanner
        if (banner.parent !== binding.inputLayout) {
            (banner.parent as? ViewGroup)?.removeView(banner)
            binding.inputLayout.addView(banner, 0)
        }
        productionSuggestions.attach(binding.inputLayout, 1)
    }

    private fun activeController(): WebChatSocialController = controllerFor(providerId())
    private fun activeProviderOrNull() = providerId().takeIf { isChatModeActive() }
    private fun activeConsumerPortOrNull() =
        if (isChatModeActive()) activeController().consumerPort() else null

    private fun activeSessionState() =
        if (isChatModeActive()) activeController().stateWireValue() else "inactive"
    private fun activeConversationPath() =
        if (isChatModeActive()) activeController().currentConversationPath() else null

    private fun retryConsumerSession() = retryWebChatConsumerSession(activeController())

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

}
