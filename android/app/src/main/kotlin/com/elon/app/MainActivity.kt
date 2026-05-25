package com.elon.app

import android.content.Intent
import android.os.Bundle
import android.text.InputType
import android.util.TypedValue
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.widget.EditText
import com.elon.app.BuildConfig
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
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
    private val conversations: MutableList<AppConversation> get() = activeProject().conversations
    private val projectEvents: MutableList<String> get() = activeProject().events
    private var currentProjectTitle: String
        get() = activeProject().title
        set(value) {
            activeProject().title = value
            activeProject().updatedAt = System.currentTimeMillis()
        }
    private var currentStage: String
        get() = activeProject().stage
        set(value) {
            activeProject().stage = value
            activeProject().updatedAt = System.currentTimeMillis()
        }
    private var activeConversationIndex: Int
        get() = activeProject().activeConversationIndex
        set(value) {
            activeProject().activeConversationIndex = value
        }
    private var conversationActions: MainConversationActions? = null
    private var homeRows: MainHomeRows? = null
    private var modelActions: MainModelActions? = null
    private var conversationOpenActions: MainConversationOpenActions? = null
    private var projectActions: MainProjectActions? = null
    private var stageHintShimmer: MainStageHintShimmer? = null
    private var actionPopup: PopupWindow? = null
    private var actionPopups: MainActionPopups? = null
    private var messageActions: MainMessageActions? = null
    private var codexPrewarm: MainCodexPrewarm? = null
    private var externalActions: MainExternalActions? = null
    private var attachmentPanelActions: MainAttachmentPanelActions? = null
    private var attachmentPickerActions: MainAttachmentPickerActions? = null
    private var attachmentSendActions: MainAttachmentSendActions? = null
    private var pendingAttachmentActions: MainPendingAttachmentActions? = null
    private var workflowStageActions: MainWorkflowStageActions? = null
    private var evidenceActions: MainEvidenceActions? = null
    private var progressNarrativeActions: MainProgressNarrativeActions? = null
    private var toolActionBubbles: MainToolActionBubbles? = null
    private var foldedCliLogActions: MainFoldedCliLogActions? = null
    private var workflowMessageCompactor: MainWorkflowMessageCompactor? = null
    private var projectHygieneActions: MainProjectHygieneActions? = null
    private var sendButtonVisualActions: MainSendButtonVisualActions? = null
    private var sendEnabledActions: MainSendEnabledActions? = null
    private var adaptiveInputHeightActions: MainAdaptiveInputHeightActions? = null
    private var collapsedInputPreviewActions: MainCollapsedInputPreviewActions? = null
    private var voiceModeActions: MainVoiceModeActions? = null
    private var inputFocusActions: MainInputFocusActions? = null
    private var assistantRawMessageActions: MainAssistantRawMessageActions? = null
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
    private var assistantTerminalActions: MainAssistantTerminalActions? = null
    private var assistantStreamEvents: MainAssistantStreamEvents? = null
    private var taskWorkEventActions: MainTaskWorkEventActions? = null
    private var taskWorkReceiverActions: MainTaskWorkReceiverActions? = null
    private var preparedMessageActions: MainPreparedMessageActions? = null
    private var sendTargetRestoreActions: MainSendTargetRestoreActions? = null
    private var sendMessageActions: MainSendMessageActions? = null
    private var serverResponseWatchdogActions: MainServerResponseWatchdogActions? = null
    private var navigationController: MainNavigationController? = null
    private lateinit var inputModeButton: ImageButton
    private lateinit var attachmentButton: ImageButton
    private lateinit var voiceHoldButton: TextView
    private lateinit var inputBarContainer: LinearLayout
    private lateinit var inputCenterContainer: FrameLayout
    private lateinit var expandedInputContainer: FrameLayout
    private lateinit var collapsedInputPreview: TextView
    private lateinit var pendingAttachmentPreviewStrip: PendingAttachmentPreviewStrip
    private lateinit var modelButtonShell: FrameLayout
    private lateinit var inputRightControls: FrameLayout
    private lateinit var inputComposerMotion: InputComposerMotion
    private lateinit var attachmentPanel: LinearLayout
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
            loadProjects = ::loadProjects,
            setupAttachmentLaunchers = { attachmentPickerActions().setupAttachmentLaunchers() },
            activeConversation = ::activeConversation,
            pauseCurrentWork = { activeWorkControlActions().pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions().showMessageActions(anchor, message) },
            setChatAdapter = { chatAdapter = it },
            setupNavigation = { navigationController().setupNavigation() },
            setupQuickActions = { profileQuickActions().setupQuickActions() },
            setupBackHandling = { navigationController().setupBackHandling() },
            setupInputComposer = ::setupInputComposer,
            restoreCachedModelSelection = { modelActions().restoreCachedModelSelection() },
            updateProjectViews = ::updateProjectViews,
            setTaskAppForeground = { foreground -> taskWorkServiceActions().setTaskAppForeground(foreground) },
            registerTaskWorkReceiver = { taskWorkReceiverActions().registerTaskWorkReceiver() },
            restorePendingActiveWork = { conversationTaskRegistryActions().restorePendingActiveWork() },
            checkAndOfferGuestImport = { accountActions().checkAndOfferGuestImport() },
            getWaitingForReply = { waitingForReply },
            getBackendConnected = { backendConnected },
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            startTaskWorkService = { action -> startTaskWorkService(action) },
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
            currentStage = { currentStage },
            updateStage = ::updateStage,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions().recordEvidence(kind, detail)
            },
            startTaskWorkService = { action -> startTaskWorkService(action) },
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            setSendEnabled = ::setSendEnabled,
            maybePrewarmCodexSession = ::maybePrewarmCodexSession
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
        saveProjects()
        super.onStop()
    }

    private fun startPreparedMessage(
        visibleText: String,
        outgoingText: String,
        attachmentRefs: com.google.gson.JsonArray,
        target: SendTarget,
        chatAttachments: List<ChatAttachment>
    ) {
        preparedMessageActions().startPreparedMessage(
            visibleText = visibleText,
            outgoingText = outgoingText,
            attachmentRefs = attachmentRefs,
            target = target,
            chatAttachments = chatAttachments
        )
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
            setSendEnabled = ::setSendEnabled,
            userId = { userId },
            selectedAgentForRequest = { modelActions().selectedAgentForRequest() },
            appendMessage = ::appendMessage,
            collapseInputComposer = { inputFocusActions().collapseInputComposer() },
            looksLikeDevelopmentRequest = ::looksLikeDevelopmentRequest,
            looksLikeDirectImageRequest = ::looksLikeDirectImageRequest,
            rememberConversationTask = conversationTaskRegistryActions()::rememberConversationTask,
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            resetRequestState = {
                pendingReconnectForActiveWork = false
                reconnectAttempts = 0
                conversationTaskRegistryActions().persistActiveWork()
                foldedCliLogActions().reset()
                evidenceActions().clearCurrentEvidence()
                toolActionBubbles().clear()
                progressNarrativeActions().clear()
            },
            acceptDevelopmentRequest = { text ->
                projectRecordActions().updateProjectTitleFromRequest(text)
                projectRecordActions().saveProjectTitle()
                projectRecordActions().addProjectEvent("提交需求：${summarize(text, 36)}")
                updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
            },
            updateProjectViews = ::updateProjectViews,
            nextServerResponseToken = { ++serverResponseToken },
            putTaskResponseToken = { traceId, token -> taskResponseTokens[traceId] = token },
            startTaskWorkService = ::startTaskWorkService,
            markTaskPendingReconnect = { target ->
                val key = conversationTaskRegistryActions().conversationTaskKey(target.projectId, target.conversationId)
                runningConversationTasks[key]?.pendingReconnect = true
            },
            refreshActiveTaskState = conversationTaskRegistryActions()::refreshActiveTaskState,
            persistActiveWork = conversationTaskRegistryActions()::persistActiveWork,
            updateStage = ::updateStage,
            scheduleFirstServerResponseWatchdog = { traceId, token ->
                serverResponseWatchdogActions().scheduleFirstServerResponseWatchdog(traceId, token)
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
            getCurrentStage = { currentStage },
            getPendingRequestPayload = { pendingRequestPayload },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            setWaitingForReply = { waitingForReply = it },
            persistActiveWork = conversationTaskRegistryActions()::persistActiveWork,
            clearPersistedActiveWork = conversationTaskRegistryActions()::clearPersistedActiveWork,
            refreshActiveTaskState = conversationTaskRegistryActions()::refreshActiveTaskState,
            stopWorkingEvidenceForActiveConversation = {
                evidenceActions().stopWorkingEvidenceForActiveConversation()
            },
            clearCurrentEvidence = { evidenceActions().clearCurrentEvidence() },
            clearToolActionBubbles = { toolActionBubbles().clear() },
            setSendEnabled = ::setSendEnabled,
            updateFirstConversationStatus = { text ->
                conversationPreviewActions().updateFirstConversationStatus(text)
            },
            updateStage = ::updateStage,
            updateProjectViews = ::updateProjectViews,
            addProjectEvent = ::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions().recordEvidence(kind, detail)
            },
            appendMessage = ::appendMessage,
            workflowStoppedMessage = { reason, wasDevelopment -> workflowStoppedMessage(reason, wasDevelopment) },
            startTaskWorkService = ::startTaskWorkService,
            nextServerResponseToken = { ++serverResponseToken },
            scheduleFirstServerResponseWatchdog = { traceId, token ->
                serverResponseWatchdogActions().scheduleFirstServerResponseWatchdog(traceId, token)
            }
        ).also { activeWorkControlActions = it }
    }

    private fun setupInputComposer() {
        val views = MainInputComposerSetup(
            activity = this,
            binding = binding,
            dp = ::dp,
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

        inputModeButton = views.inputModeButton
        attachmentButton = views.attachmentButton
        voiceHoldButton = views.voiceHoldButton
        inputBarContainer = views.inputBarContainer
        inputCenterContainer = views.inputCenterContainer
        expandedInputContainer = views.expandedInputContainer
        collapsedInputPreview = views.collapsedInputPreview
        modelButtonShell = views.modelButtonShell
        inputRightControls = views.inputRightControls
        inputComposerMotion = views.inputComposerMotion
        attachmentPanel = views.attachmentPanel
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

    private fun adaptiveInputHeightActions(): MainAdaptiveInputHeightActions {
        adaptiveInputHeightActions?.let { return it }
        return MainAdaptiveInputHeightActions(
            binding = binding,
            dp = ::dp,
            inputCenterContainer = { if (::inputCenterContainer.isInitialized) inputCenterContainer else null },
            inputBarContainer = { if (::inputBarContainer.isInitialized) inputBarContainer else null },
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
            isVoiceMode = { voiceMode }
        ).also { adaptiveInputHeightActions = it }
    }

    private fun inputFocusActions(): MainInputFocusActions {
        inputFocusActions?.let { return it }
        return MainInputFocusActions(
            activity = this,
            binding = binding,
            activeConversation = ::activeConversation,
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = { voiceModeActions().applyVoiceMode() },
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
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
            collapsedInputPreview = { if (::collapsedInputPreview.isInitialized) collapsedInputPreview else null }
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
            activeConversation = ::activeConversation,
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
            activeProject = ::activeProject,
            activeConversation = ::activeConversation,
            appendMessage = ::appendMessage,
            collapseInputComposer = { inputFocusActions().collapseInputComposer() },
            uploadAttachmentsThenSend = { visibleText, outgoingText, target ->
                attachmentSendActions().uploadAttachmentsThenSend(visibleText, outgoingText, target)
            },
            startPreparedMessage = ::startPreparedMessage
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
            setSendEnabled = ::setSendEnabled,
            startPreparedMessage = ::startPreparedMessage
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
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
            refreshPendingAttachmentPreview = ::refreshPendingAttachmentPreview
        ).also { pendingAttachmentActions = it }
    }

    private fun attachmentPanelActions(): MainAttachmentPanelActions {
        attachmentPanelActions?.let { return it }
        return MainAttachmentPanelActions(
            activity = this,
            dp = ::dp,
            selectableForeground = ::selectableForeground,
            activeConversation = ::activeConversation,
            attachmentPanel = { if (::attachmentPanel.isInitialized) attachmentPanel else null },
            attachmentButton = { if (::attachmentButton.isInitialized) attachmentButton else null },
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
            inputModeButton = { if (::inputModeButton.isInitialized) inputModeButton else null },
            voiceHoldButton = { if (::voiceHoldButton.isInitialized) voiceHoldButton else null },
            inputCenterContainer = { if (::inputCenterContainer.isInitialized) inputCenterContainer else null },
            expandedInputContainer = { if (::expandedInputContainer.isInitialized) expandedInputContainer else null },
            collapsedInputPreview = { if (::collapsedInputPreview.isInitialized) collapsedInputPreview else null },
            modelButtonShell = { if (::modelButtonShell.isInitialized) modelButtonShell else null },
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
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
            dp = ::dp,
            attachmentButton = { if (::attachmentButton.isInitialized) attachmentButton else null },
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
            isVoiceMode = { voiceMode },
            hasPendingAttachments = { pendingAttachments.isNotEmpty() },
            inputCanSend = { inputCanSend },
            activeConversation = ::activeConversation
        ).also { sendButtonVisualActions = it }
    }

    private fun speechInputActions(): MainSpeechInputActions {
        speechInputActions?.let { return it }
        return MainSpeechInputActions(
            activity = this,
            binding = binding,
            speechPermissionRequest = speechPermissionRequest,
            activeConversation = ::activeConversation,
            voiceHoldButton = { voiceHoldButton },
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
            activeConversationProvider = ::activeConversation,
            activeConversationIndexProvider = { activeConversationIndex },
            compactProjectTitle = { projectRecordActions().compactProjectTitle() },
            renderConversationList = ::renderConversationList,
            renderProjectList = ::renderProjectList,
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
            setSendEnabled = ::setSendEnabled,
            maybePrewarmCodexSession = ::maybePrewarmCodexSession
        ).also { navigationController = it }
    }

    private fun titleEditText(value: String): EditText {
        return EditText(this).apply {
            setText(value)
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
            maxLines = 1
            setSingleLine(true)
            setSelectAllOnFocus(true)
            setPadding(dp(18), dp(8), dp(18), dp(8))
        }
    }

    private fun conversationActions(): MainConversationActions {
        conversationActions?.let { return it }
        return MainConversationActions(
            activity = this,
            binding = binding,
            conversationsProvider = { conversations },
            activeProjectProvider = ::activeProject,
            activeConversationIndexProvider = { activeConversationIndex },
            setActiveConversationIndex = { activeConversationIndex = it },
            chatAdapterProvider = { chatAdapter },
            titleEditText = ::titleEditText,
            saveConversations = ::saveConversations,
            renderConversationList = ::renderConversationList,
            setSendEnabled = ::setSendEnabled
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
            modelButtonShellProvider = { if (::modelButtonShell.isInitialized) modelButtonShell else null },
            inputBarContainerProvider = { if (::inputBarContainer.isInitialized) inputBarContainer else null },
            getActionPopup = { actionPopup },
            setActionPopup = { actionPopup = it },
            openSettings = { quickCommandActions().openSettings() },
            dp = ::dp,
            selectableForeground = ::selectableForeground
        ).also { modelActions = it }
    }

    private fun conversationOpenActions(): MainConversationOpenActions {
        conversationOpenActions?.let { return it }
        return MainConversationOpenActions(
            binding = binding,
            projects = { projects },
            conversations = { conversations },
            activeConversation = ::activeConversation,
            activeConversationIndex = { activeConversationIndex },
            setActiveProjectIndex = { activeProjectIndex = it },
            setActiveConversationIndex = { activeConversationIndex = it },
            setChatAdapter = { chatAdapter = it },
            pauseCurrentWork = { activeWorkControlActions().pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions().showMessageActions(anchor, message) },
            showChat = { animate -> navigationController().showChat(animate = animate) },
            saveProjects = ::saveProjects
        ).also { conversationOpenActions = it }
    }

    private fun activeProject(): AppProject {
        if (projects.isEmpty()) {
            projects.add(newAppProject("一龙开发助手", "默认项目 · 点击进入会话"))
        }
        activeProjectIndex = activeProjectIndex.coerceIn(0, projects.lastIndex)
        val project = projects[activeProjectIndex]
        if (project.conversations.isEmpty()) project.conversations.add(defaultAppConversation())
        project.activeConversationIndex = project.activeConversationIndex.coerceIn(0, project.conversations.lastIndex)
        return project
    }

    private fun activeConversation(): AppConversation {
        if (conversations.isEmpty()) {
            conversations.add(defaultAppConversation())
        }
        activeConversationIndex = activeConversationIndex.coerceIn(0, conversations.lastIndex)
        return conversations[activeConversationIndex]
    }

    private fun loadProjects() {
        val loaded = loadStoredProjects(
            prefs,
            gson,
            { project -> projectHygieneActions().normalizeProject(project) }
        )
        projects.clear()
        projects.addAll(loaded.projects)
        activeProjectIndex = loaded.activeProjectIndex
        activeProject()
        saveProjects()
    }

    private fun saveConversations() {
        saveProjects()
    }

    private fun saveProjects() {
        saveStoredProjects(prefs, gson, projects, activeProjectIndex, activeProject().id)
    }

    private fun conversationTaskRegistryActions(): MainConversationTaskRegistryActions {
        conversationTaskRegistryActions?.let { return it }
        return MainConversationTaskRegistryActions(
            prefs = prefs,
            runningConversationTasks = runningConversationTasks,
            runningTraceToConversation = runningTraceToConversation,
            taskResponseTokens = taskResponseTokens,
            activeProject = ::activeProject,
            activeConversation = ::activeConversation,
            setWaitingForReply = { waitingForReply = it },
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            setPendingRequestPayload = { pendingRequestPayload = it },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            setSendEnabled = ::setSendEnabled,
            renderConversationList = ::renderConversationList,
            updateStage = ::updateStage,
            updateProjectViews = ::updateProjectViews
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
                if (activeRequestIsDevelopment) evidenceActions().recordEvidence(kind, detail)
            },
            setSendEnabled = ::setSendEnabled,
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            handleActiveWorkDisconnected = { task -> activeWorkControlActions().handleActiveWorkDisconnected(task) },
            updateIdleReadyStatus = { conversationPreviewActions().updateIdleReadyStatus() },
            appendTaskMessage = { raw, traceId, projectId, conversationId, isDevelopment ->
                traceId?.let { taskResponseTokens.remove(it) }
                appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
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

    private fun appendTaskMessage(
        raw: String,
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?
    ) {
        taskMessageRouterActions().appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
    }

    private fun taskMessageRouterActions(): MainTaskMessageRouterActions {
        taskMessageRouterActions?.let { return it }
        return MainTaskMessageRouterActions(
            keyForTrace = { traceId -> runningTraceToConversation[traceId] },
            conversationTaskKey = conversationTaskRegistryActions()::conversationTaskKey,
            activeConversationTaskKey = conversationTaskRegistryActions()::activeConversationTaskKey,
            taskIsDevelopment = { key -> runningConversationTasks[key]?.isDevelopment },
            appendActiveMessage = { raw -> appendMessage(raw) },
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

    private fun startTaskWorkService(
        action: String,
        payload: String? = null,
        isDevelopment: Boolean = activeRequestIsDevelopment,
        traceId: String? = null
    ): Boolean {
        return taskWorkServiceActions().startTaskWorkService(action, payload, isDevelopment, traceId)
    }

    private fun taskWorkServiceActions(): MainTaskWorkServiceActions {
        taskWorkServiceActions?.let { return it }
        return MainTaskWorkServiceActions(
            activity = this,
            prefs = prefs,
            appendTaskMessage = ::appendTaskMessage,
            appendRawMessage = { raw -> appendMessage(raw) }
        ).also { taskWorkServiceActions = it }
    }

    private fun conversationPreviewActions(): MainConversationPreviewActions {
        conversationPreviewActions?.let { return it }
        return MainConversationPreviewActions(
            binding = binding,
            projects = { projects },
            conversations = { conversations },
            activeProject = ::activeProject,
            activeConversation = ::activeConversation,
            activeProjectIndex = { activeProjectIndex },
            activeConversationIndex = { activeConversationIndex },
            chatAdapter = { chatAdapter },
            conversationTaskKey = conversationTaskRegistryActions()::conversationTaskKey,
            workflowTerminalRoles = workflowTerminalRoles,
            closeStaleWorkflowMessages = { messages ->
                workflowMessageCompactor().closeStaleWorkflowMessages(messages)
            },
            hasRunningTasks = { runningConversationTasks.isNotEmpty() },
            saveConversations = ::saveConversations,
            saveProjects = ::saveProjects,
            renderConversationList = ::renderConversationList,
            renderProjectList = ::renderProjectList
        ).also { conversationPreviewActions = it }
    }

    private fun renderConversationList() {
        homeListActions().renderConversationList()
    }

    private fun renderProjectList() {
        homeListActions().renderProjectList()
    }

    private fun homeListActions(): MainHomeListActions {
        homeListActions?.let { return it }
        return MainHomeListActions(
            activity = this,
            binding = binding,
            projects = { projects },
            conversations = { conversations },
            activeProject = ::activeProject,
            compactProjectTitle = { projectRecordActions().compactProjectTitle() },
            formatTime = { timeFormatter.format(Date(it)) },
            isTaskRunning = { projectId, conversationId ->
                val key = conversationTaskRegistryActions().conversationTaskKey(projectId, conversationId)
                runningConversationTasks.containsKey(key)
            },
            homeRows = { homeRows() },
            dp = ::dp,
            selectableForeground = ::selectableForeground,
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
            dp = ::dp,
            selectableForeground = ::selectableForeground
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
            setActiveConversationIndex = { activeConversationIndex = it },
            titleEditText = ::titleEditText,
            saveProjects = ::saveProjects,
            renderProjectList = ::renderProjectList,
            openProject = conversationOpenActions()::openProject,
            showGitProjectDialog = ::showGitProjectDialog
        ).also { projectActions = it }
    }

    private fun selectableForeground() = runCatching {
        val outValue = TypedValue()
        theme.resolveAttribute(android.R.attr.selectableItemBackground, outValue, true)
        getDrawable(outValue.resourceId)
    }.getOrNull()

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }

    private fun accountActions(): MainAccountActions {
        return MainAccountActions(
            activity = this,
            binding = binding,
            projects = projects,
            gson = gson,
            prefs = prefs,
            saveProjects = ::saveProjects,
            renderProjectList = ::renderProjectList
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
            shareActions = ::shareActions,
            fillPlanPrompt = { quickCommandActions().fillPlanPrompt() },
            sendQuickCommand = { text -> quickCommandActions().sendQuickCommand(text) },
            showProjectRecordDialog = { projectRecordActions().showProjectRecordDialog() },
            showGitProjectDialog = ::showGitProjectDialog,
            showCreateProjectDialog = { projectActions().showCreateProjectDialog() },
            showCreateConversationDialog = { conversationActions().showCreateConversationDialog() },
            openSettings = { quickCommandActions().openSettings() },
            deleteMessage = { message -> messageActions().deleteMessage(message) },
            quoteMessage = { text -> messageActions().quoteMessage(text) },
            dp = ::dp,
            selectableForeground = ::selectableForeground
        ).also { actionPopups = it }
    }

    private fun showGitProjectDialog() {
        MainProjectGitDialogs(
            activity = this,
            http = http,
            serverUrl = serverUrl,
            userId = userId,
            projectProvider = ::activeProject,
            projectTitleProvider = { currentProjectTitle },
            addProjectEvent = ::addProjectEvent,
            openUrl = { url -> externalActions().openUrl(url) },
            copyText = { label, text -> externalActions().copyText(label, text) }
        ).showGitProjectDialog()
    }

    private fun maybePrewarmCodexSession(reason: String) {
        codexPrewarm().maybePrewarmCodexSession(reason)
    }

    private fun codexPrewarm(): MainCodexPrewarm {
        codexPrewarm?.let { return it }
        return MainCodexPrewarm(
            http = http,
            serverUrl = serverUrl,
            userId = userId,
            activeProject = ::activeProject,
            activeConversation = ::activeConversation,
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            selectedAgentForRequest = { modelActions().selectedAgentForRequest() }
        ).also { codexPrewarm = it }
    }

    private fun externalActions(): MainExternalActions {
        externalActions?.let { return it }
        return MainExternalActions(this).also { externalActions = it }
    }

    private fun quickCommandActions(): MainQuickCommandActions {
        quickCommandActions?.let { return it }
        return MainQuickCommandActions(
            activity = this,
            binding = binding,
            activeConversation = ::activeConversation,
            showCreateConversationDialog = { conversationActions().showCreateConversationDialog() },
            showChat = { navigationController().showChat() },
            sendMessage = { sendMessageActions().sendMessage() }
        ).also { quickCommandActions = it }
    }

    private fun shareActions(): MainShareActions {
        return MainShareActions(this, ::dp)
    }

    private fun messageActions(): MainMessageActions {
        messageActions?.let { return it }
        return MainMessageActions(
            activity = this,
            binding = binding,
            activeConversation = ::activeConversation,
            chatAdapter = { chatAdapter },
            saveConversations = ::saveConversations,
            renderConversationList = ::renderConversationList,
            showChat = { navigationController().showChat() },
            showMessageActionPopup = { anchor, message, text ->
                actionPopups().showMessageActionPopup(anchor, message, text)
            },
            shareActions = ::shareActions,
            apkDownloadUrl = { apkDownloadUrl },
            apkDownloadPageUrl = { apkDownloadPageUrl }
        ).also { messageActions = it }
    }

    private fun workflowStoppedMessage(reason: String, wasDevelopment: Boolean = activeRequestIsDevelopment): String {
        val stage = if (wasDevelopment) "需要处理" else "回复中断"
        return "工作停止：$stage。原因：$reason"
    }

    private fun aiMessageWithCurrentEvidence(
        content: String,
        attachments: List<ChatAttachment> = emptyList()
    ): ChatMessage {
        return evidenceActions().aiMessageWithCurrentEvidence(content, attachments)
    }

    private fun evidenceActions(): MainEvidenceActions {
        evidenceActions?.let { return it }
        return MainEvidenceActions(
            activeConversation = ::activeConversation,
            chatAdapter = { chatAdapter },
            saveConversations = ::saveConversations,
            assistantEvidenceRoles = assistantEvidenceRoles
        ).also { evidenceActions = it }
    }

    private fun foldedCliLogActions(): MainFoldedCliLogActions {
        foldedCliLogActions?.let { return it }
        return MainFoldedCliLogActions(
            currentStage = { currentStage },
            updateStage = ::updateStage,
            maybeAppendVisibleCliSignal = { category, line ->
                progressNarrativeActions().maybeAppendVisibleCliSignal(category, line)
            },
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions().recordEvidence(kind, detail)
            }
        ).also { foldedCliLogActions = it }
    }

    private fun progressNarrativeActions(): MainProgressNarrativeActions {
        progressNarrativeActions?.let { return it }
        return MainProgressNarrativeActions(
            isDevelopmentRequest = { activeRequestIsDevelopment },
            finalizeEvidenceForLatestAssistant = { evidenceActions().finalizeEvidenceForLatestAssistant() },
            appendMessage = ::appendMessage,
            attachEvidenceToLatestAi = { evidenceActions().attachEvidenceToLatestAi() }
        ).also { progressNarrativeActions = it }
    }

    private fun toolActionBubbles(): MainToolActionBubbles {
        toolActionBubbles?.let { return it }
        return MainToolActionBubbles(
            activeConversation = ::activeConversation,
            chatAdapter = { chatAdapter },
            saveConversations = ::saveConversations,
            appendMessage = ::appendMessage
        ).also { toolActionBubbles = it }
    }

    private fun projectHygieneActions(): MainProjectHygieneActions {
        projectHygieneActions?.let { return it }
        return MainProjectHygieneActions(
            timeText = { timeFormatter.format(Date()) },
            removeLeakedAndRoutineWorkflowMessages = { messages ->
                workflowMessageCompactor().removeLeakedAndRoutineWorkflowMessages(messages)
            },
            compactWorkflowStatusMessages = { messages ->
                workflowMessageCompactor().compactWorkflowStatusMessages(messages)
            },
            closeStaleWorkflowMessages = { messages ->
                workflowMessageCompactor().closeStaleWorkflowMessages(messages)
            }
        ).also { projectHygieneActions = it }
    }

    private fun workflowMessageCompactor(): MainWorkflowMessageCompactor {
        workflowMessageCompactor?.let { return it }
        return MainWorkflowMessageCompactor(
            staleWorkflowRoles = staleWorkflowRoles,
            workflowHistoryStatusRoles = workflowHistoryStatusRoles,
            workflowTerminalRoles = workflowTerminalRoles
        ).also { workflowMessageCompactor = it }
    }

    private fun appendMessage(raw: String) {
        assistantRawMessageActions().appendMessage(raw)
    }

    private fun serverResponseWatchdogActions(): MainServerResponseWatchdogActions {
        serverResponseWatchdogActions?.let { return it }
        return MainServerResponseWatchdogActions(
            binding = binding,
            taskResponseTokens = taskResponseTokens,
            taskForTrace = { traceId -> runningTraceToConversation[traceId]?.let { runningConversationTasks[it] } },
            activeConversationTask = conversationTaskRegistryActions()::activeConversationTask,
            getCurrentStage = { currentStage },
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            refreshActiveTaskState = conversationTaskRegistryActions()::refreshActiveTaskState,
            updateStage = ::updateStage,
            addProjectEvent = ::addProjectEvent,
            startTaskWorkService = ::startTaskWorkService
        ).also { serverResponseWatchdogActions = it }
    }

    private fun assistantRawMessageActions(): MainAssistantRawMessageActions {
        assistantRawMessageActions?.let { return it }
        return MainAssistantRawMessageActions(
            activity = this,
            assistantStreamEvents = { assistantStreamEvents() },
            assistantTerminalActions = { assistantTerminalActions() },
            incrementServerResponseToken = { serverResponseToken += 1 },
            appendMessage = { msg -> appendMessage(msg) }
        ).also { assistantRawMessageActions = it }
    }

    private fun appendMessage(msg: ChatMessage) {
        if (msg.role in workflowTerminalRoles) {
            removeTransientWorkflowMessagesAfterLatestUser()
        }
        chatAdapter.addMessage(msg)
        conversationPreviewActions().updateActiveConversationPreview(msg)
        binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
    }

    private fun removeTransientWorkflowMessagesAfterLatestUser() {
        if (workflowMessageCompactor().removeTransientWorkflowMessagesAfterLatestUser(activeConversation().messages)) {
            chatAdapter.notifyDataSetChanged()
            saveConversations()
        }
    }

    private fun assistantStreamEvents(): MainAssistantStreamEvents {
        assistantStreamEvents?.let { return it }
        return MainAssistantStreamEvents(
            handleTaskEvent = { event, taskId, content ->
                workflowStageActions().handleTaskEvent(event, taskId, content)
            },
            maybeAppendTaskEventNarrative = { event, content ->
                progressNarrativeActions().maybeAppendTaskEventNarrative(event, content)
            },
            maybeAppendWorkflowProgressNarrative = { content ->
                progressNarrativeActions().maybeAppendWorkflowProgressNarrative(content)
            },
            maybeAppendToolCallNarrative = { tool ->
                progressNarrativeActions().maybeAppendToolCallNarrative(tool)
            },
            handleProgress = { content, recordProgressEvidence ->
                workflowStageActions().handleProgress(content, recordProgressEvidence)
            },
            handleFoldedCliOutput = { content -> foldedCliLogActions().handleFoldedCliOutput(content) },
            markToolCallStarted = { tool -> workflowStageActions().handleToolCall(tool) },
            appendToolCallBubble = { tool, args -> toolActionBubbles().appendToolCallBubble(tool, args) },
            markToolResultDone = { toolActionBubbles().markToolResultDone(it) },
            markToolResult = { workflowStageActions().markToolResult(it) },
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions().recordEvidence(kind, detail)
            },
            isDevelopmentRequest = { activeRequestIsDevelopment },
            addProjectEvent = ::addProjectEvent
        ).also { assistantStreamEvents = it }
    }

    private fun assistantTerminalActions(): MainAssistantTerminalActions {
        assistantTerminalActions?.let { return it }
        return MainAssistantTerminalActions(
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            setWaitingForReply = { waitingForReply = it },
            setSendEnabled = ::setSendEnabled,
            clearPendingRequestPayload = { pendingRequestPayload = null },
            clearPendingReconnectForActiveWork = { pendingReconnectForActiveWork = false },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            clearPersistedActiveWork = conversationTaskRegistryActions()::clearPersistedActiveWork,
            updateStage = ::updateStage,
            updateProjectViews = ::updateProjectViews,
            addProjectEvent = ::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions().recordEvidence(kind, detail)
            },
            stopWorkingEvidenceForActiveConversation = {
                evidenceActions().stopWorkingEvidenceForActiveConversation()
            },
            clearCurrentEvidence = { evidenceActions().clearCurrentEvidence() },
            resetFoldedCliLog = { foldedCliLogActions().reset() },
            aiMessageWithCurrentEvidence = ::aiMessageWithCurrentEvidence,
            appendMessage = ::appendMessage,
            workflowStoppedMessage = { workflowStoppedMessage(it) }
        ).also { assistantTerminalActions = it }
    }

    private fun workflowStageActions(): MainWorkflowStageActions {
        workflowStageActions?.let { return it }
        return MainWorkflowStageActions(
            currentStage = { currentStage },
            updateStage = ::updateStage,
            addProjectEvent = ::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) evidenceActions().recordEvidence(kind, detail)
            }
        ).also { workflowStageActions = it }
    }

    private fun stageHintShimmer(): MainStageHintShimmer {
        stageHintShimmer?.let { return it }
        return MainStageHintShimmer(
            binding = binding,
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking
        ).also { stageHintShimmer = it }
    }

    private fun updateStage(stage: String, hint: String) {
        currentStage = stage
        activeProject().subtitle = hint
        saveProjects()
        updateProjectViews(hint)
    }

    private fun updateProjectViews(hint: String) {
        projectViewActions().updateProjectViews(hint)
    }

    private fun projectViewActions(): MainProjectViewActions {
        projectViewActions?.let { return it }
        return MainProjectViewActions(
            activity = this,
            binding = binding,
            currentStage = { currentStage },
            currentProjectTitle = { currentProjectTitle },
            projectEvents = { projectEvents },
            currentTimeText = { timeFormatter.format(Date()) },
            renderConversationList = ::renderConversationList,
            renderProjectList = ::renderProjectList,
            updateStageHintShimmer = { stageHintShimmer().update() }
        ).also { projectViewActions = it }
    }

    private companion object {
        val assistantEvidenceRoles = setOf("ai", "ai-intent")
        val staleWorkflowRoles = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool")
        val workflowHistoryStatusRoles = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete")
        val workflowTerminalRoles = setOf("ai", "ai-intent", "error", "ai-stopped")
    }

    private fun addProjectEvent(text: String) {
        projectRecordActions().addProjectEvent(text)
    }

    private fun projectRecordActions(): MainProjectRecordActions {
        projectRecordActions?.let { return it }
        return MainProjectRecordActions(
            activity = this,
            appName = { getString(R.string.app_name) },
            currentProjectTitle = { currentProjectTitle },
            setCurrentProjectTitle = { currentProjectTitle = it },
            activeProject = ::activeProject,
            projectEvents = { projectEvents },
            currentStage = { currentStage },
            conversationCount = { conversations.size },
            currentTimeText = { timeFormatter.format(Date()) },
            currentStageHint = { binding.stageHintText.text.toString() },
            saveProjects = ::saveProjects,
            updateProjectViews = ::updateProjectViews
        ).also { projectRecordActions = it }
    }

    private fun setSendEnabled(enabled: Boolean) {
        sendEnabledActions().setSendEnabled(enabled)
    }

    private fun sendEnabledActions(): MainSendEnabledActions {
        sendEnabledActions?.let { return it }
        return MainSendEnabledActions(
            binding = binding,
            activeConversation = ::activeConversation,
            setInputCanSend = { inputCanSend = it },
            inputModeButton = { if (::inputModeButton.isInitialized) inputModeButton else null },
            voiceHoldButton = { if (::voiceHoldButton.isInitialized) voiceHoldButton else null },
            modelButtonShell = { if (::modelButtonShell.isInitialized) modelButtonShell else null },
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
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

