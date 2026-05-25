package com.elon.app

import android.content.Intent
import android.os.Bundle
import android.view.Menu
import android.view.MenuItem
import android.view.View
import com.elon.app.BuildConfig
import android.widget.PopupWindow
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var chatAdapter: ChatAdapter
    private var waitingForReply = false
    private var activeRequestIsDevelopment = false
    private var serverResponseToken = 0
    private var appInForeground = false
    private var pendingRequestPayload: String? = null
    private var pendingReconnectForActiveWork = false
    private var reconnectAttempts = 0
    private val runningConversationTasks = linkedMapOf<String, ConversationTaskState>()
    private val runningTraceToConversation = linkedMapOf<String, String>()
    private val taskResponseTokens = linkedMapOf<String, Int>()
    private var backendConnected = false
    private val projects = mutableListOf<AppProject>()
    private val gson = com.google.gson.Gson()
    private val http = OkHttpClient()
    private val timeFormatter = SimpleDateFormat("HH:mm", Locale.CHINA)
    private val prefs by lazy { AuthManager.userDataPrefs(this) }
    private val serverUrl = "http://43.139.149.158:8080"
    private val apkDownloadUrl: String get() = "$serverUrl/app/ElonSpeed-latest.apk"
    private val apkDownloadPageUrl: String get() = "$serverUrl/app/download"
    private val serverVersionUrl: String get() = "$serverUrl/api/server/version"
    private var activeProjectIndex = 0
    private var conversationActions: MainConversationActions? = null
    private var homeRows: MainHomeRows? = null
    private var modelActions: MainModelActions? = null
    private var projectStateActions: MainProjectStateActions? = null
    private var conversationOpenActions: MainConversationOpenActions? = null
    private var projectActions: MainProjectActions? = null
    private var stageHintShimmer: MainStageHintShimmer? = null
    private var actionPopup: PopupWindow? = null
    private var actionPopups: MainActionPopups? = null
    private var messageActions: MainMessageActions? = null
    private var codexPrewarm: MainCodexPrewarm? = null
    private var attachmentPanelActions: MainAttachmentPanelActions? = null
    private var attachmentPickerActions: MainAttachmentPickerActions? = null
    private var attachmentSendActions: MainAttachmentSendActions? = null
    private var pendingAttachmentActions: MainPendingAttachmentActions? = null
    private var sendButtonVisualActions: MainSendButtonVisualActions? = null
    private var sendEnabledActions: MainSendEnabledActions? = null
    private var adaptiveInputHeightActions: MainAdaptiveInputHeightActions? = null
    private var collapsedInputPreviewActions: MainCollapsedInputPreviewActions? = null
    private var voiceModeActions: MainVoiceModeActions? = null
    private var inputFocusActions: MainInputFocusActions? = null
    private var backgroundTaskMessageActions: MainBackgroundTaskMessageActions? = null
    private var taskMessageRouterActions: MainTaskMessageRouterActions? = null
    private var conversationTaskRegistryActions: MainConversationTaskRegistryActions? = null
    private var taskWorkServiceActions: MainTaskWorkServiceActions? = null
    private var activeWorkControlActions: MainActiveWorkControlActions? = null
    private var projectViewActions: MainProjectViewActions? = null
    private var projectRecordActions: MainProjectRecordActions? = null
    private var homeListActions: MainHomeListActions? = null
    private var conversationPreviewActions: MainConversationPreviewActions? = null
    private var profileQuickActions: MainProfileQuickActions? = null
    private var quickCommandActions: MainQuickCommandActions? = null
    private var createActions: MainCreateActions? = null
    private var resumeActions: MainResumeActions? = null
    private var lifecycleEdgeActions: MainLifecycleEdgeActions? = null
    private var taskWorkEventActions: MainTaskWorkEventActions? = null
    private var taskWorkReceiverActions: MainTaskWorkReceiverActions? = null
    private var preparedMessageActions: MainPreparedMessageActions? = null
    private var sendTargetRestoreActions: MainSendTargetRestoreActions? = null
    private var sendMessageActions: MainSendMessageActions? = null
    private var navigationController: MainNavigationController? = null
    private lateinit var inputComposerViews: MainInputComposerViews
    private lateinit var pendingAttachmentPreviewStrip: PendingAttachmentPreviewStrip
    private var voiceMode = false
    private var inputCanSend = true
    private var suppressInputFocusAnimation = false
    private var speechInputActions: MainSpeechInputActions? = null
    private val speechPermissionRequest = 4301
    private val notificationPermissionRequest = 4302
    private val pendingAttachments = mutableListOf<PendingAttachment>()
    /**
     * 当前会话使用的 user_id。
     * - 已登录：使用服务端返回的 user.id（跨设备稳定）。
     * - 未登录（游客）：使用本机随机 UUID（与老版本兼容）。
     * by lazy 在 Activity 实例生命周期内固定；登录/登出后会清掉栈重建 MainActivity。
     */
    private val userId: String by lazy { AuthManager.effectiveUserId(this) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)
        createActions().onCreate(intent)
    }

    private fun createActions(): MainCreateActions {
        createActions?.let { return it }
        return MainCreateActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            notificationPermissionRequest = notificationPermissionRequest,
            loadProjects = projectStateActions()::loadProjects,
            setupAttachmentLaunchers = { attachmentPickerActions().setupAttachmentLaunchers() },
            activeConversation = projectStateActions()::activeConversation,
            pauseCurrentWork = { activeWorkControlActions().pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions().showMessageActions(anchor, message) },
            setChatAdapter = { chatAdapter = it },
            setupNavigation = { navigationController().setupNavigation() },
            setupQuickActions = { profileQuickActions().setupQuickActions() },
            setupBackHandling = { navigationController().setupBackHandling() },
            setupInputComposer = ::setupInputComposer,
            restoreCachedModelSelection = { modelActions().restoreCachedModelSelection() },
            updateProjectViews = projectViewActions()::updateProjectViews,
            setTaskAppForeground = { foreground -> taskWorkServiceActions().setTaskAppForeground(foreground) },
            registerTaskWorkReceiver = { taskWorkReceiverActions().registerTaskWorkReceiver() },
            restorePendingActiveWork = { conversationTaskRegistryActions().restorePendingActiveWork() },
            checkAndOfferGuestImport = { accountActions().checkAndOfferGuestImport() },
            getWaitingForReply = { waitingForReply },
            getBackendConnected = { backendConnected },
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            startTaskWorkService = { action ->
                taskWorkServiceActions().startTaskWorkService(action, isDevelopment = activeRequestIsDevelopment)
            },
            openConversation = conversationOpenActions()::openConversation,
            loadModelOptions = { modelActions().loadModelOptions() },
            sendMessage = { sendMessageActions().sendMessage() }
        ).also { createActions = it }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        createActions().handleLaunchIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        resumeActions().onResume()
    }

    private fun resumeActions(): MainResumeActions {
        resumeActions?.let { return it }
        return MainResumeActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            isBindingInitialized = { ::binding.isInitialized },
            setAppInForeground = { appInForeground = it },
            setTaskAppForeground = { foreground -> taskWorkServiceActions().setTaskAppForeground(foreground) },
            drainQueuedTaskEvents = { taskWorkServiceActions().drainQueuedTaskEvents() },
            loadModelOptions = { modelActions().loadModelOptions() },
            getBackendConnected = { backendConnected },
            getWaitingForReply = { waitingForReply },
            getPendingReconnectForActiveWork = { pendingReconnectForActiveWork },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            currentStage = { projectStateActions().currentStage },
            updateStage = projectViewActions()::updateStage,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions.recordEvidence(kind, detail)
            },
            startTaskWorkService = { action ->
                taskWorkServiceActions().startTaskWorkService(action, isDevelopment = activeRequestIsDevelopment)
            },
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            maybePrewarmCodexSession = codexPrewarm()::maybePrewarmCodexSession
        ).also { resumeActions = it }
    }

    override fun onPause() {
        appInForeground = false
        taskWorkServiceActions().setTaskAppForeground(false)
        super.onPause()
    }

    override fun onStop() {
        appInForeground = false
        taskWorkServiceActions().setTaskAppForeground(false)
        projectStateActions().saveProjects()
        super.onStop()
    }

    private fun preparedMessageActions(): MainPreparedMessageActions {
        preparedMessageActions?.let { return it }
        return MainPreparedMessageActions(
            activity = this,
            binding = binding,
            restoreSendTarget = { target -> sendTargetRestoreActions().restoreSendTarget(target) },
            isConversationTaskRunning = { target ->
                val key = conversationTaskRegistryActions().conversationTaskKey(target.projectId, target.conversationId)
                runningConversationTasks.containsKey(key)
            },
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            userId = { userId },
            selectedAgentForRequest = { modelActions().selectedAgentForRequest() },
            appendMessage = messageAppendActions::appendMessage,
            collapseInputComposer = { inputFocusActions().collapseInputComposer() },
            looksLikeDevelopmentRequest = ::looksLikeDevelopmentRequest,
            looksLikeDirectImageRequest = ::looksLikeDirectImageRequest,
            rememberConversationTask = conversationTaskRegistryActions()::rememberConversationTask,
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            resetRequestState = {
                pendingReconnectForActiveWork = false
                reconnectAttempts = 0
                conversationTaskRegistryActions().persistActiveWork()
                foldedCliLogActions.reset()
                evidenceActions.clearCurrentEvidence()
                toolActionBubbles.clear()
                progressNarrativeActions.clear()
            },
            acceptDevelopmentRequest = { text ->
                projectRecordActions().updateProjectTitleFromRequest(text)
                projectRecordActions().saveProjectTitle()
                projectRecordActions().addProjectEvent("提交需求：${summarize(text, 36)}")
                projectViewActions().updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
            },
            updateProjectViews = projectViewActions()::updateProjectViews,
            nextServerResponseToken = { ++serverResponseToken },
            putTaskResponseToken = { traceId, token -> taskResponseTokens[traceId] = token },
            startTaskWorkService = taskWorkServiceActions()::startTaskWorkService,
            markTaskPendingReconnect = { target ->
                val key = conversationTaskRegistryActions().conversationTaskKey(target.projectId, target.conversationId)
                runningConversationTasks[key]?.pendingReconnect = true
            },
            refreshActiveTaskState = conversationTaskRegistryActions()::refreshActiveTaskState,
            persistActiveWork = conversationTaskRegistryActions()::persistActiveWork,
            updateStage = projectViewActions()::updateStage,
            scheduleFirstServerResponseWatchdog = { traceId, token ->
                serverResponseWatchdogActions.scheduleFirstServerResponseWatchdog(traceId, token)
            },
            clearPendingAttachments = {
                pendingAttachmentActions().clearPendingAttachments(deleteFiles = false)
            }
        ).also { preparedMessageActions = it }
    }

    private fun activeWorkControlActions(): MainActiveWorkControlActions {
        activeWorkControlActions?.let { return it }
        return MainActiveWorkControlActions(
            binding = binding,
            activeConversationTask = conversationTaskRegistryActions()::activeConversationTask,
            removeConversationTask = conversationTaskRegistryActions()::removeConversationTask,
            resetReconnectAttempts = { reconnectAttempts = 0 },
            incrementReconnectAttempts = {
                reconnectAttempts += 1
                reconnectAttempts
            },
            taskForTrace = { traceId ->
                runningTraceToConversation[traceId]?.let { runningConversationTasks[it] }
            },
            isBackendConnected = { backendConnected },
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            getCurrentStage = { projectStateActions().currentStage },
            getPendingRequestPayload = { pendingRequestPayload },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            setWaitingForReply = { waitingForReply = it },
            persistActiveWork = conversationTaskRegistryActions()::persistActiveWork,
            clearPersistedActiveWork = conversationTaskRegistryActions()::clearPersistedActiveWork,
            refreshActiveTaskState = conversationTaskRegistryActions()::refreshActiveTaskState,
            stopWorkingEvidenceForActiveConversation = {
                evidenceActions.stopWorkingEvidenceForActiveConversation()
            },
            clearCurrentEvidence = { evidenceActions.clearCurrentEvidence() },
            clearToolActionBubbles = { toolActionBubbles.clear() },
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            updateFirstConversationStatus = { text ->
                conversationPreviewActions().updateFirstConversationStatus(text)
            },
            updateStage = projectViewActions()::updateStage,
            updateProjectViews = projectViewActions()::updateProjectViews,
            addProjectEvent = projectRecordActions()::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions.recordEvidence(kind, detail)
            },
            appendMessage = messageAppendActions::appendMessage,
            workflowStoppedMessage = ::mainWorkflowStoppedMessage,
            startTaskWorkService = taskWorkServiceActions()::startTaskWorkService,
            nextServerResponseToken = { ++serverResponseToken },
            scheduleFirstServerResponseWatchdog = { traceId, token ->
                serverResponseWatchdogActions.scheduleFirstServerResponseWatchdog(traceId, token)
            }
        ).also { activeWorkControlActions = it }
    }

    private fun setupInputComposer() {
        val views = MainInputComposerSetup(
            activity = this,
            binding = binding,
            dp = uiTools::dp,
            currentModelLabel = { modelActions().currentModelLabel },
            isVoiceMode = { voiceMode },
            shouldAnimateInputFocus = { !suppressInputFocusAnimation },
            isAttachmentPanelOpen = { attachmentPanelActions().isOpen },
            toggleVoiceMode = { voiceModeActions().toggleVoiceMode() },
            focusInputComposer = { inputFocusActions().focusInputComposer() },
            startSpeechToText = { speechInputActions().startSpeechToText() },
            stopSpeechToText = { speechInputActions().stopSpeechToText() },
            showModelPopupOrLoad = { modelActions().showModelPopupOrLoad() },
            sendMessage = { sendMessageActions().sendMessage() },
            toggleAttachmentPanel = { attachmentPanelActions().toggleAttachmentPanel() },
            buildAttachmentPanel = { attachmentPanelActions().buildAttachmentPanel() },
            collapseAttachmentPanel = { attachmentPanelActions().collapseAttachmentPanel() },
            collapseInputComposer = { inputFocusActions().collapseInputComposer() },
            updateCollapsedInputPreview = { collapsedInputPreviewActions().updateCollapsedInputPreview() },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = { adaptiveInputHeightActions().updateAdaptiveInputHeight() }
        ).setup()

        inputComposerViews = views
        pendingAttachmentPreviewStrip = PendingAttachmentPreviewStrip(this, pendingAttachments) {
            collapsedInputPreviewActions().updateCollapsedInputPreview()
            updateSendButtonVisual()
        }
        binding.inputLayout.addView(pendingAttachmentPreviewStrip.view, 1)
        voiceModeActions().applyVoiceMode()
        collapsedInputPreviewActions().updateCollapsedInputPreview()
        updateSendButtonVisual()
        adaptiveInputHeightActions().updateAdaptiveInputHeight()
    }

    private fun inputComposerViewsOrNull(): MainInputComposerViews? {
        return if (::inputComposerViews.isInitialized) inputComposerViews else null
    }

    private fun adaptiveInputHeightActions(): MainAdaptiveInputHeightActions {
        adaptiveInputHeightActions?.let { return it }
        return MainAdaptiveInputHeightActions(
            binding = binding,
            dp = uiTools::dp,
            inputCenterContainer = { inputComposerViewsOrNull()?.inputCenterContainer },
            inputBarContainer = { inputComposerViewsOrNull()?.inputBarContainer },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            isVoiceMode = { voiceMode }
        ).also { adaptiveInputHeightActions = it }
    }

    private fun inputFocusActions(): MainInputFocusActions {
        inputFocusActions?.let { return it }
        return MainInputFocusActions(
            activity = this,
            binding = binding,
            activeConversation = projectStateActions()::activeConversation,
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = { voiceModeActions().applyVoiceMode() },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            setSuppressInputFocusAnimation = { suppressInputFocusAnimation = it },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = { adaptiveInputHeightActions().updateAdaptiveInputHeight() }
        ).also { inputFocusActions = it }
    }

    private fun collapsedInputPreviewActions(): MainCollapsedInputPreviewActions {
        collapsedInputPreviewActions?.let { return it }
        return MainCollapsedInputPreviewActions(
            binding = binding,
            pendingAttachments = { pendingAttachments },
            collapsedInputPreview = { inputComposerViewsOrNull()?.collapsedInputPreview }
        ).also { collapsedInputPreviewActions = it }
    }

    private fun refreshPendingAttachmentPreview() {
        if (!::pendingAttachmentPreviewStrip.isInitialized) return
        pendingAttachmentPreviewStrip.refresh()
        collapsedInputPreviewActions().updateCollapsedInputPreview()
        updateSendButtonVisual()
    }

    private fun attachmentPickerActions(): MainAttachmentPickerActions {
        attachmentPickerActions?.let { return it }
        return MainAttachmentPickerActions(
            activity = this,
            activeConversation = projectStateActions()::activeConversation,
            attachPickedFile = { kind, uri, fallbackName ->
                pendingAttachmentActions().attachPickedFile(kind, uri, fallbackName)
            }
        ).also { attachmentPickerActions = it }
    }

    private fun sendMessageActions(): MainSendMessageActions {
        sendMessageActions?.let { return it }
        return MainSendMessageActions(
            binding = binding,
            pendingAttachments = pendingAttachments,
            collapseAttachmentPanel = { attachmentPanelActions().collapseAttachmentPanel() },
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            activeProject = projectStateActions()::activeProject,
            activeConversation = projectStateActions()::activeConversation,
            appendMessage = messageAppendActions::appendMessage,
            collapseInputComposer = { inputFocusActions().collapseInputComposer() },
            uploadAttachmentsThenSend = { visibleText, outgoingText, target ->
                attachmentSendActions().uploadAttachmentsThenSend(visibleText, outgoingText, target)
            },
            startPreparedMessage = preparedMessageActions()::startPreparedMessage
        ).also { sendMessageActions = it }
    }

    private fun sendTargetRestoreActions(): MainSendTargetRestoreActions {
        sendTargetRestoreActions?.let { return it }
        return MainSendTargetRestoreActions(
            binding = binding,
            projects = projects,
            setActiveProjectIndex = { activeProjectIndex = it },
            setChatAdapter = { chatAdapter = it },
            pauseCurrentWork = { activeWorkControlActions().pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions().showMessageActions(anchor, message) },
            showChat = { navigationController().showChat() }
        ).also { sendTargetRestoreActions = it }
    }

    private fun attachmentSendActions(): MainAttachmentSendActions {
        attachmentSendActions?.let { return it }
        return MainAttachmentSendActions(
            activity = this,
            http = http,
            serverUrl = serverUrl,
            userId = { userId },
            pendingAttachments = { pendingAttachments.toList() },
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            startPreparedMessage = preparedMessageActions()::startPreparedMessage
        ).also { attachmentSendActions = it }
    }

    private fun pendingAttachmentActions(): MainPendingAttachmentActions {
        pendingAttachmentActions?.let { return it }
        return MainPendingAttachmentActions(
            activity = this,
            pendingAttachments = pendingAttachments,
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = { voiceModeActions().applyVoiceMode() },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            refreshPendingAttachmentPreview = ::refreshPendingAttachmentPreview
        ).also { pendingAttachmentActions = it }
    }

    private fun attachmentPanelActions(): MainAttachmentPanelActions {
        attachmentPanelActions?.let { return it }
        return MainAttachmentPanelActions(
            activity = this,
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            activeConversation = projectStateActions()::activeConversation,
            attachmentPanel = { inputComposerViewsOrNull()?.attachmentPanel },
            attachmentButton = { inputComposerViewsOrNull()?.attachmentButton },
            collapseInputComposer = { inputFocusActions().collapseInputComposer() },
            openCameraAttachment = { attachmentPickerActions().openCameraAttachment() },
            openPhotoAttachment = { attachmentPickerActions().openPhotoAttachment() },
            openDocumentAttachment = { attachmentPickerActions().openDocumentAttachment() }
        ).also { attachmentPanelActions = it }
    }

    private fun voiceModeActions(): MainVoiceModeActions {
        voiceModeActions?.let { return it }
        return MainVoiceModeActions(
            activity = this,
            binding = binding,
            inputModeButton = { inputComposerViewsOrNull()?.inputModeButton },
            voiceHoldButton = { inputComposerViewsOrNull()?.voiceHoldButton },
            inputCenterContainer = { inputComposerViewsOrNull()?.inputCenterContainer },
            expandedInputContainer = { inputComposerViewsOrNull()?.expandedInputContainer },
            collapsedInputPreview = { inputComposerViewsOrNull()?.collapsedInputPreview },
            modelButtonShell = { inputComposerViewsOrNull()?.modelButtonShell },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            collapseAttachmentPanel = { attachmentPanelActions().collapseAttachmentPanel() },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = { adaptiveInputHeightActions().updateAdaptiveInputHeight() }
        ).also { voiceModeActions = it }
    }

    private fun updateSendButtonVisual() {
        if (!::binding.isInitialized) return
        sendButtonVisualActions().updateSendButtonVisual()
    }

    private fun sendButtonVisualActions(): MainSendButtonVisualActions {
        sendButtonVisualActions?.let { return it }
        return MainSendButtonVisualActions(
            activity = this,
            binding = binding,
            dp = uiTools::dp,
            attachmentButton = { inputComposerViewsOrNull()?.attachmentButton },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            isVoiceMode = { voiceMode },
            hasPendingAttachments = { pendingAttachments.isNotEmpty() },
            inputCanSend = { inputCanSend },
            activeConversation = projectStateActions()::activeConversation
        ).also { sendButtonVisualActions = it }
    }

    private fun speechInputActions(): MainSpeechInputActions {
        speechInputActions?.let { return it }
        return MainSpeechInputActions(
            activity = this,
            binding = binding,
            speechPermissionRequest = speechPermissionRequest,
            activeConversation = projectStateActions()::activeConversation,
            voiceHoldButton = { inputComposerViews.voiceHoldButton },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = { voiceModeActions().applyVoiceMode() }
        ).also { speechInputActions = it }
    }

    private fun navigationController(): MainNavigationController {
        navigationController?.let { return it }
        return MainNavigationController(
            activity = this,
            binding = binding,
            actionPopupProvider = { actionPopup },
            activeConversationProvider = projectStateActions()::activeConversation,
            activeConversationIndexProvider = { projectStateActions().activeConversationIndex },
            compactProjectTitle = { projectRecordActions().compactProjectTitle() },
            renderConversationList = homeListActions()::renderConversationList,
            renderProjectList = homeListActions()::renderProjectList,
            refreshServerVersion = { profileQuickActions().refreshServerVersion() },
            openConversation = conversationOpenActions()::openConversation,
            showConversationActions = { index -> conversationActions().showConversationActions(index) },
            showHomeActionPopup = { anchor, tab -> actionPopups().showHomeActionPopup(anchor, tab) },
            showChatActionPopup = { anchor -> actionPopups().showChatActionPopup(anchor) },
            updateFirstConversationStatus = { text ->
                conversationPreviewActions().updateFirstConversationStatus(text)
            },
            collapseInputComposer = { animate -> inputFocusActions().collapseInputComposer(animate) },
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            maybePrewarmCodexSession = codexPrewarm()::maybePrewarmCodexSession
        ).also { navigationController = it }
    }

    private fun conversationActions(): MainConversationActions {
        conversationActions?.let { return it }
        return MainConversationActions(
            activity = this,
            binding = binding,
            conversationsProvider = { projectStateActions().conversations },
            activeProjectProvider = projectStateActions()::activeProject,
            activeConversationIndexProvider = { projectStateActions().activeConversationIndex },
            setActiveConversationIndex = { projectStateActions().activeConversationIndex = it },
            chatAdapterProvider = { chatAdapter },
            titleEditText = { value -> mainTitleEditText(this, value, uiTools::dp) },
            saveConversations = projectStateActions()::saveConversations,
            renderConversationList = homeListActions()::renderConversationList,
            setSendEnabled = sendEnabledActions()::setSendEnabled
        ).also { conversationActions = it }
    }

    private fun modelActions(): MainModelActions {
        modelActions?.let { return it }
        return MainModelActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            http = http,
            serverUrl = serverUrl,
            userIdProvider = { userId },
            modelButtonShellProvider = { inputComposerViewsOrNull()?.modelButtonShell },
            inputBarContainerProvider = { inputComposerViewsOrNull()?.inputBarContainer },
            getActionPopup = { actionPopup },
            setActionPopup = { actionPopup = it },
            openSettings = { quickCommandActions().openSettings() },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground
        ).also { modelActions = it }
    }

    private fun conversationOpenActions(): MainConversationOpenActions {
        conversationOpenActions?.let { return it }
        return MainConversationOpenActions(
            binding = binding,
            projects = { projects },
            conversations = { projectStateActions().conversations },
            activeConversation = projectStateActions()::activeConversation,
            activeConversationIndex = { projectStateActions().activeConversationIndex },
            setActiveProjectIndex = { activeProjectIndex = it },
            setActiveConversationIndex = { projectStateActions().activeConversationIndex = it },
            setChatAdapter = { chatAdapter = it },
            pauseCurrentWork = { activeWorkControlActions().pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions().showMessageActions(anchor, message) },
            showChat = { animate -> navigationController().showChat(animate = animate) },
            saveProjects = projectStateActions()::saveProjects
        ).also { conversationOpenActions = it }
    }

    private fun projectStateActions(): MainProjectStateActions {
        projectStateActions?.let { return it }
        return MainProjectStateActions(
            prefs = prefs,
            gson = gson,
            projects = projects,
            activeProjectIndex = { activeProjectIndex },
            setActiveProjectIndex = { activeProjectIndex = it },
            normalizeProject = { project -> projectHygieneActions.normalizeProject(project) }
        ).also { projectStateActions = it }
    }

    private fun conversationTaskRegistryActions(): MainConversationTaskRegistryActions {
        conversationTaskRegistryActions?.let { return it }
        return MainConversationTaskRegistryActions(
            prefs = prefs,
            runningConversationTasks = runningConversationTasks,
            runningTraceToConversation = runningTraceToConversation,
            taskResponseTokens = taskResponseTokens,
            activeProject = projectStateActions()::activeProject,
            activeConversation = projectStateActions()::activeConversation,
            setWaitingForReply = { waitingForReply = it },
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            setPendingRequestPayload = { pendingRequestPayload = it },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            renderConversationList = homeListActions()::renderConversationList,
            updateStage = projectViewActions()::updateStage,
            updateProjectViews = projectViewActions()::updateProjectViews
        ).also { conversationTaskRegistryActions = it }
    }

    private fun taskWorkEventActions(): MainTaskWorkEventActions {
        taskWorkEventActions?.let { return it }
        return MainTaskWorkEventActions(
            getBackendConnected = { backendConnected },
            setBackendConnected = { backendConnected = it },
            getWaitingForReply = { waitingForReply },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            updateFirstConversationStatus = { text ->
                conversationPreviewActions().updateFirstConversationStatus(text)
            },
            updateConversationTaskFromService =
                conversationTaskRegistryActions()::updateConversationTaskFromService,
            activeConversationTask = conversationTaskRegistryActions()::activeConversationTask,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions.recordEvidence(kind, detail)
            },
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            handleActiveWorkDisconnected = { task -> activeWorkControlActions().handleActiveWorkDisconnected(task) },
            updateIdleReadyStatus = { conversationPreviewActions().updateIdleReadyStatus() },
            appendTaskMessage = { raw, traceId, projectId, conversationId, isDevelopment ->
                traceId?.let { taskResponseTokens.remove(it) }
                taskMessageRouterActions().appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
            },
            removeConversationTask = conversationTaskRegistryActions()::removeConversationTask,
            syncActiveTasksFromServiceState = { activeTasksJson ->
                conversationTaskRegistryActions().syncActiveTasksFromServiceState(activeTasksJson)
            },
            clearTaskMaps = {
                runningConversationTasks.clear()
                runningTraceToConversation.clear()
                taskResponseTokens.clear()
            },
            refreshActiveTaskState = { conversationTaskRegistryActions().refreshActiveTaskState() }
        ).also { taskWorkEventActions = it }
    }

    private fun taskWorkReceiverActions(): MainTaskWorkReceiverActions {
        taskWorkReceiverActions?.let { return it }
        return MainTaskWorkReceiverActions(
            activity = this,
            handleTaskWorkEvent = { intent -> taskWorkEventActions().handleTaskWorkEvent(intent) }
        ).also { taskWorkReceiverActions = it }
    }

    private fun taskMessageRouterActions(): MainTaskMessageRouterActions {
        taskMessageRouterActions?.let { return it }
        return MainTaskMessageRouterActions(
            keyForTrace = { traceId -> runningTraceToConversation[traceId] },
            conversationTaskKey = conversationTaskRegistryActions()::conversationTaskKey,
            activeConversationTaskKey = conversationTaskRegistryActions()::activeConversationTaskKey,
            taskIsDevelopment = { key -> runningConversationTasks[key]?.isDevelopment },
            appendActiveMessage = { raw -> assistantRawMessageActions.appendMessage(raw) },
            appendBackgroundTaskMessage = { raw, key, isDevelopment ->
                backgroundTaskMessageActions().appendBackgroundTaskMessage(raw, key, isDevelopment)
            },
            removeConversationTask = conversationTaskRegistryActions()::removeConversationTask,
            persistActiveWork = conversationTaskRegistryActions()::persistActiveWork,
            updateConversationTaskFromService =
                conversationTaskRegistryActions()::updateConversationTaskFromService
        ).also { taskMessageRouterActions = it }
    }

    private fun backgroundTaskMessageActions(): MainBackgroundTaskMessageActions {
        backgroundTaskMessageActions?.let { return it }
        return MainBackgroundTaskMessageActions(
            activity = this,
            findConversationLocationByKey = { key -> conversationPreviewActions().findConversationLocationByKey(key) },
            appendMessageToConversation = { projectIndex, conversationIndex, message ->
                conversationPreviewActions().appendMessageToConversation(projectIndex, conversationIndex, message)
            }
        ).also { backgroundTaskMessageActions = it }
    }

    private fun taskWorkServiceActions(): MainTaskWorkServiceActions {
        taskWorkServiceActions?.let { return it }
        return MainTaskWorkServiceActions(
            activity = this,
            prefs = prefs,
            appendTaskMessage = taskMessageRouterActions()::appendTaskMessage,
            appendRawMessage = { raw -> assistantRawMessageActions.appendMessage(raw) }
        ).also { taskWorkServiceActions = it }
    }

    private fun conversationPreviewActions(): MainConversationPreviewActions {
        conversationPreviewActions?.let { return it }
        return MainConversationPreviewActions(
            binding = binding,
            projects = { projects },
            conversations = { projectStateActions().conversations },
            activeProject = projectStateActions()::activeProject,
            activeConversation = projectStateActions()::activeConversation,
            activeProjectIndex = { activeProjectIndex },
            activeConversationIndex = { projectStateActions().activeConversationIndex },
            chatAdapter = { chatAdapter },
            conversationTaskKey = conversationTaskRegistryActions()::conversationTaskKey,
            workflowTerminalRoles = MainWorkflowRoles.terminal,
            closeStaleWorkflowMessages = { messages ->
                workflowMessageCompactor.closeStaleWorkflowMessages(messages)
            },
            hasRunningTasks = { runningConversationTasks.isNotEmpty() },
            saveConversations = projectStateActions()::saveConversations,
            saveProjects = projectStateActions()::saveProjects,
            renderConversationList = homeListActions()::renderConversationList,
            renderProjectList = homeListActions()::renderProjectList
        ).also { conversationPreviewActions = it }
    }

    private fun homeListActions(): MainHomeListActions {
        homeListActions?.let { return it }
        return MainHomeListActions(
            activity = this,
            binding = binding,
            projects = { projects },
            conversations = { projectStateActions().conversations },
            activeProject = projectStateActions()::activeProject,
            compactProjectTitle = { projectRecordActions().compactProjectTitle() },
            formatTime = { timeFormatter.format(Date(it)) },
            isTaskRunning = { projectId, conversationId ->
                val key = conversationTaskRegistryActions().conversationTaskKey(projectId, conversationId)
                runningConversationTasks.containsKey(key)
            },
            homeRows = { homeRows() },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            showCreateProjectDialog = { projectActions().showCreateProjectDialog() }
        ).also { homeListActions = it }
    }

    private fun homeRows(): MainHomeRows {
        homeRows?.let { return it }
        return MainHomeRows(
            activity = this,
            timeFormatter = timeFormatter,
            activeProjectIndexProvider = { activeProjectIndex },
            openProject = conversationOpenActions()::openProject,
            showProjectActions = { index -> projectActions().showProjectActions(index) },
            openConversation = conversationOpenActions()::openConversation,
            showConversationActions = { index -> conversationActions().showConversationActions(index) },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground
        ).also { homeRows = it }
    }

    private fun projectActions(): MainProjectActions {
        projectActions?.let { return it }
        return MainProjectActions(
            activity = this,
            binding = binding,
            projects = projects,
            activeProjectIndexProvider = { activeProjectIndex },
            setActiveProjectIndex = { activeProjectIndex = it },
            setActiveConversationIndex = { projectStateActions().activeConversationIndex = it },
            titleEditText = { value -> mainTitleEditText(this, value, uiTools::dp) },
            saveProjects = projectStateActions()::saveProjects,
            renderProjectList = homeListActions()::renderProjectList,
            openProject = conversationOpenActions()::openProject,
            showGitProjectDialog = ::showGitProjectDialog
        ).also { projectActions = it }
    }

    private val uiTools: MainUiTools by lazy { MainUiTools(this) }

    private fun accountActions(): MainAccountActions {
        return MainAccountActions(
            activity = this,
            binding = binding,
            projects = projects,
            gson = gson,
            prefs = prefs,
            saveProjects = projectStateActions()::saveProjects,
            renderProjectList = homeListActions()::renderProjectList
        )
    }

    private fun profileQuickActions(): MainProfileQuickActions {
        profileQuickActions?.let { return it }
        return MainProfileQuickActions(
            activity = this,
            binding = binding,
            http = http,
            serverVersionUrl = serverVersionUrl,
            isBindingInitialized = { ::binding.isInitialized },
            refreshAccountUi = {
                if (::binding.isInitialized) accountActions().refreshAccountUi()
            },
            fillPlanPrompt = { quickCommandActions().fillPlanPrompt() },
            sendQuickCommand = { text -> quickCommandActions().sendQuickCommand(text) },
            showProjectRecordDialog = { projectRecordActions().showProjectRecordDialog() },
            showGitProjectDialog = ::showGitProjectDialog,
            openSettings = { quickCommandActions().openSettings() },
            showPromotionDialog = { messageActions().showPromotionDialog() },
            showGuestImportDialog = { accountActions().showGuestImportDialog() },
            confirmLogout = { accountActions().confirmLogout() }
        ).also { profileQuickActions = it }
    }

    private fun actionPopups(): MainActionPopups {
        actionPopups?.let { return it }
        return MainActionPopups(
            activity = this,
            binding = binding,
            getActionPopup = { actionPopup },
            setActionPopup = { actionPopup = it },
            shareActions = uiTools::shareActions,
            fillPlanPrompt = { quickCommandActions().fillPlanPrompt() },
            sendQuickCommand = { text -> quickCommandActions().sendQuickCommand(text) },
            showProjectRecordDialog = { projectRecordActions().showProjectRecordDialog() },
            showGitProjectDialog = ::showGitProjectDialog,
            showCreateProjectDialog = { projectActions().showCreateProjectDialog() },
            showCreateConversationDialog = { conversationActions().showCreateConversationDialog() },
            openSettings = { quickCommandActions().openSettings() },
            deleteMessage = { message -> messageActions().deleteMessage(message) },
            quoteMessage = { text -> messageActions().quoteMessage(text) },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground
        ).also { actionPopups = it }
    }

    private fun showGitProjectDialog() {
        MainProjectGitDialogs(
            activity = this,
            http = http,
            serverUrl = serverUrl,
            userId = userId,
            projectProvider = projectStateActions()::activeProject,
            projectTitleProvider = { projectStateActions().currentProjectTitle },
            addProjectEvent = projectRecordActions()::addProjectEvent,
            openUrl = { url -> externalActions.openUrl(url) },
            copyText = { label, text -> externalActions.copyText(label, text) }
        ).showGitProjectDialog()
    }

    private fun codexPrewarm(): MainCodexPrewarm {
        codexPrewarm?.let { return it }
        return MainCodexPrewarm(
            http = http,
            serverUrl = serverUrl,
            userId = userId,
            activeProject = projectStateActions()::activeProject,
            activeConversation = projectStateActions()::activeConversation,
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            selectedAgentForRequest = { modelActions().selectedAgentForRequest() }
        ).also { codexPrewarm = it }
    }

    private val externalActions: MainExternalActions by lazy { MainExternalActions(this) }

    private fun quickCommandActions(): MainQuickCommandActions {
        quickCommandActions?.let { return it }
        return MainQuickCommandActions(
            activity = this,
            binding = binding,
            activeConversation = projectStateActions()::activeConversation,
            showCreateConversationDialog = { conversationActions().showCreateConversationDialog() },
            showChat = { navigationController().showChat() },
            sendMessage = { sendMessageActions().sendMessage() }
        ).also { quickCommandActions = it }
    }

    private fun messageActions(): MainMessageActions {
        messageActions?.let { return it }
        return MainMessageActions(
            activity = this,
            binding = binding,
            activeConversation = projectStateActions()::activeConversation,
            chatAdapter = { chatAdapter },
            saveConversations = projectStateActions()::saveConversations,
            renderConversationList = homeListActions()::renderConversationList,
            showChat = { navigationController().showChat() },
            showMessageActionPopup = { anchor, message, text ->
                actionPopups().showMessageActionPopup(anchor, message, text)
            },
            shareActions = uiTools::shareActions,
            apkDownloadUrl = { apkDownloadUrl },
            apkDownloadPageUrl = { apkDownloadPageUrl }
        ).also { messageActions = it }
    }

    private val evidenceActions: MainEvidenceActions by lazy {
        MainEvidenceActions(
            activeConversation = projectStateActions()::activeConversation,
            chatAdapter = { chatAdapter },
            saveConversations = projectStateActions()::saveConversations,
            assistantEvidenceRoles = MainWorkflowRoles.assistantEvidence
        )
    }

    private val foldedCliLogActions: MainFoldedCliLogActions by lazy {
        MainFoldedCliLogActions(
            currentStage = { projectStateActions().currentStage },
            updateStage = projectViewActions()::updateStage,
            maybeAppendVisibleCliSignal = { category, line ->
                progressNarrativeActions.maybeAppendVisibleCliSignal(category, line)
            },
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions.recordEvidence(kind, detail)
            }
        )
    }

    private val progressNarrativeActions: MainProgressNarrativeActions by lazy {
        MainProgressNarrativeActions(
            isDevelopmentRequest = { activeRequestIsDevelopment },
            finalizeEvidenceForLatestAssistant = { evidenceActions.finalizeEvidenceForLatestAssistant() },
            appendMessage = messageAppendActions::appendMessage,
            attachEvidenceToLatestAi = { evidenceActions.attachEvidenceToLatestAi() }
        )
    }

    private val toolActionBubbles: MainToolActionBubbles by lazy {
        MainToolActionBubbles(
            activeConversation = projectStateActions()::activeConversation,
            chatAdapter = { chatAdapter },
            saveConversations = projectStateActions()::saveConversations,
            appendMessage = messageAppendActions::appendMessage
        )
    }

    private val projectHygieneActions: MainProjectHygieneActions by lazy {
        MainProjectHygieneActions(
            timeText = { timeFormatter.format(Date()) },
            removeLeakedAndRoutineWorkflowMessages = { messages ->
                workflowMessageCompactor.removeLeakedAndRoutineWorkflowMessages(messages)
            },
            compactWorkflowStatusMessages = { messages ->
                workflowMessageCompactor.compactWorkflowStatusMessages(messages)
            },
            closeStaleWorkflowMessages = { messages ->
                workflowMessageCompactor.closeStaleWorkflowMessages(messages)
            }
        )
    }

    private val workflowMessageCompactor: MainWorkflowMessageCompactor by lazy {
        MainWorkflowMessageCompactor(
            staleWorkflowRoles = MainWorkflowRoles.staleWorkflow,
            workflowHistoryStatusRoles = MainWorkflowRoles.historyStatus,
            workflowTerminalRoles = MainWorkflowRoles.terminal
        )
    }

    private val serverResponseWatchdogActions: MainServerResponseWatchdogActions by lazy {
        MainServerResponseWatchdogActions(
            binding = binding,
            taskResponseTokens = taskResponseTokens,
            taskForTrace = { traceId -> runningTraceToConversation[traceId]?.let { runningConversationTasks[it] } },
            activeConversationTask = conversationTaskRegistryActions()::activeConversationTask,
            getCurrentStage = { projectStateActions().currentStage },
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            refreshActiveTaskState = conversationTaskRegistryActions()::refreshActiveTaskState,
            updateStage = projectViewActions()::updateStage,
            addProjectEvent = projectRecordActions()::addProjectEvent,
            startTaskWorkService = taskWorkServiceActions()::startTaskWorkService
        )
    }

    private val assistantRawMessageActions: MainAssistantRawMessageActions by lazy {
        MainAssistantRawMessageActions(
            activity = this,
            assistantStreamEvents = { assistantStreamEvents },
            assistantTerminalActions = { assistantTerminalActions },
            incrementServerResponseToken = { serverResponseToken += 1 },
            appendMessage = messageAppendActions::appendMessage
        )
    }

    private val messageAppendActions: MainMessageAppendActions by lazy {
        MainMessageAppendActions(
            binding = binding,
            chatAdapter = { chatAdapter },
            activeConversation = projectStateActions()::activeConversation,
            workflowMessageCompactor = { workflowMessageCompactor },
            updateActiveConversationPreview = { message ->
                conversationPreviewActions().updateActiveConversationPreview(message)
            },
            saveConversations = projectStateActions()::saveConversations,
            workflowTerminalRoles = MainWorkflowRoles.terminal
        )
    }

    private val assistantStreamEvents: MainAssistantStreamEvents by lazy {
        MainAssistantStreamEvents(
            handleTaskEvent = { event, taskId, content ->
                workflowStageActions.handleTaskEvent(event, taskId, content)
            },
            maybeAppendTaskEventNarrative = { event, content ->
                progressNarrativeActions.maybeAppendTaskEventNarrative(event, content)
            },
            maybeAppendWorkflowProgressNarrative = { content ->
                progressNarrativeActions.maybeAppendWorkflowProgressNarrative(content)
            },
            maybeAppendToolCallNarrative = { tool ->
                progressNarrativeActions.maybeAppendToolCallNarrative(tool)
            },
            handleProgress = { content, recordProgressEvidence ->
                workflowStageActions.handleProgress(content, recordProgressEvidence)
            },
            handleFoldedCliOutput = { content -> foldedCliLogActions.handleFoldedCliOutput(content) },
            markToolCallStarted = { tool -> workflowStageActions.handleToolCall(tool) },
            appendToolCallBubble = { tool, args -> toolActionBubbles.appendToolCallBubble(tool, args) },
            markToolResultDone = { toolActionBubbles.markToolResultDone(it) },
            markToolResult = { workflowStageActions.markToolResult(it) },
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions.recordEvidence(kind, detail)
            },
            isDevelopmentRequest = { activeRequestIsDevelopment },
            addProjectEvent = projectRecordActions()::addProjectEvent
        )
    }

    private val assistantTerminalActions: MainAssistantTerminalActions by lazy {
        MainAssistantTerminalActions(
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            setWaitingForReply = { waitingForReply = it },
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            clearPendingRequestPayload = { pendingRequestPayload = null },
            clearPendingReconnectForActiveWork = { pendingReconnectForActiveWork = false },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            clearPersistedActiveWork = conversationTaskRegistryActions()::clearPersistedActiveWork,
            updateStage = projectViewActions()::updateStage,
            updateProjectViews = projectViewActions()::updateProjectViews,
            addProjectEvent = projectRecordActions()::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions.recordEvidence(kind, detail)
            },
            stopWorkingEvidenceForActiveConversation = {
                evidenceActions.stopWorkingEvidenceForActiveConversation()
            },
            clearCurrentEvidence = { evidenceActions.clearCurrentEvidence() },
            resetFoldedCliLog = { foldedCliLogActions.reset() },
            aiMessageWithCurrentEvidence = evidenceActions::aiMessageWithCurrentEvidence,
            appendMessage = messageAppendActions::appendMessage,
            workflowStoppedMessage = { reason -> mainWorkflowStoppedMessage(reason, activeRequestIsDevelopment) }
        )
    }

    private val workflowStageActions: MainWorkflowStageActions by lazy {
        MainWorkflowStageActions(
            currentStage = { projectStateActions().currentStage },
            updateStage = projectViewActions()::updateStage,
            addProjectEvent = projectRecordActions()::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions.recordEvidence(kind, detail)
            }
        )
    }

    private fun stageHintShimmer(): MainStageHintShimmer {
        stageHintShimmer?.let { return it }
        return MainStageHintShimmer(
            binding = binding,
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking
        ).also { stageHintShimmer = it }
    }

    private fun projectViewActions(): MainProjectViewActions {
        projectViewActions?.let { return it }
        return MainProjectViewActions(
            activity = this,
            binding = binding,
            currentStage = { projectStateActions().currentStage },
            setCurrentStage = { projectStateActions().currentStage = it },
            setActiveProjectSubtitle = { projectStateActions().activeProject().subtitle = it },
            currentProjectTitle = { projectStateActions().currentProjectTitle },
            projectEvents = { projectStateActions().projectEvents },
            currentTimeText = { timeFormatter.format(Date()) },
            saveProjects = projectStateActions()::saveProjects,
            renderConversationList = homeListActions()::renderConversationList,
            renderProjectList = homeListActions()::renderProjectList,
            updateStageHintShimmer = { stageHintShimmer().update() }
        ).also { projectViewActions = it }
    }

    private fun projectRecordActions(): MainProjectRecordActions {
        projectRecordActions?.let { return it }
        return MainProjectRecordActions(
            activity = this,
            appName = { getString(R.string.app_name) },
            currentProjectTitle = { projectStateActions().currentProjectTitle },
            setCurrentProjectTitle = { projectStateActions().currentProjectTitle = it },
            activeProject = projectStateActions()::activeProject,
            projectEvents = { projectStateActions().projectEvents },
            currentStage = { projectStateActions().currentStage },
            conversationCount = { projectStateActions().conversations.size },
            currentTimeText = { timeFormatter.format(Date()) },
            currentStageHint = { binding.stageHintText.text.toString() },
            saveProjects = projectStateActions()::saveProjects,
            updateProjectViews = projectViewActions()::updateProjectViews
        ).also { projectRecordActions = it }
    }

    private fun sendEnabledActions(): MainSendEnabledActions {
        sendEnabledActions?.let { return it }
        return MainSendEnabledActions(
            binding = binding,
            activeConversation = projectStateActions()::activeConversation,
            setInputCanSend = { inputCanSend = it },
            inputModeButton = { inputComposerViewsOrNull()?.inputModeButton },
            voiceHoldButton = { inputComposerViewsOrNull()?.voiceHoldButton },
            modelButtonShell = { inputComposerViewsOrNull()?.modelButtonShell },
            inputComposerMotion = { inputComposerViewsOrNull()?.inputComposerMotion },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateStageHintShimmer = { stageHintShimmer().update() }
        ).also { sendEnabledActions = it }
    }

    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        return false
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == R.id.action_settings) {
            quickCommandActions().openSettings()
            return true
        }
        return super.onOptionsItemSelected(item)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        lifecycleEdgeActions().onRequestPermissionsResult(requestCode, grantResults)
    }

    override fun onDestroy() {
        lifecycleEdgeActions().onDestroy()
        super.onDestroy()
    }

    private fun lifecycleEdgeActions(): MainLifecycleEdgeActions {
        lifecycleEdgeActions?.let { return it }
        return MainLifecycleEdgeActions(
            activity = this,
            speechPermissionRequest = speechPermissionRequest,
            notificationPermissionRequest = notificationPermissionRequest,
            stopStageHintShimmer = { stageHintShimmer?.stop() },
            cancelHomeRowShimmer = {
                homeRows?.cancelHomeRowShimmer()
                homeRows = null
            },
            destroySpeechInput = {
                speechInputActions?.destroy()
                speechInputActions = null
            },
            isTaskWorkReceiverRegistered = { taskWorkReceiverActions().isRegistered },
            unregisterTaskWorkReceiver = { taskWorkReceiverActions().unregisterTaskWorkReceiver() }
        ).also { lifecycleEdgeActions = it }
    }
}
