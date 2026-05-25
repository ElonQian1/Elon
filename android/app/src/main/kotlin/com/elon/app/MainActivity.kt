package com.elon.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.text.InputType
import android.util.TypedValue
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import com.elon.app.BuildConfig
import com.elon.app.update.AppUpdateManager
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
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
    private var workflowStepIndex = 0
    private var serverResponseToken = 0
    private var appInForeground = false
    private var pendingRequestPayload: String? = null
    private var pendingReconnectForActiveWork = false
    private var reconnectAttempts = 0
    private val runningConversationTasks = linkedMapOf<String, ConversationTaskState>()
    private val runningTraceToConversation = linkedMapOf<String, String>()
    private val taskResponseTokens = linkedMapOf<String, Int>()
    private var backendConnected = false
    private var taskWorkReceiverRegistered = false
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
    private var preparedMessageActions: MainPreparedMessageActions? = null
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
    private val taskWorkReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            handleTaskWorkEvent(intent)
        }
    }

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
            setupAttachmentLaunchers = ::setupAttachmentLaunchers,
            activeConversation = ::activeConversation,
            pauseCurrentWork = { activeWorkControlActions().pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions().showMessageActions(anchor, message) },
            setChatAdapter = { chatAdapter = it },
            setupNavigation = ::setupNavigation,
            setupQuickActions = ::setupQuickActions,
            setupBackHandling = ::setupBackHandling,
            setupInputComposer = ::setupInputComposer,
            restoreCachedModelSelection = ::restoreCachedModelSelection,
            updateProjectViews = ::updateProjectViews,
            setTaskAppForeground = ::setTaskAppForeground,
            registerTaskWorkReceiver = ::registerTaskWorkReceiver,
            restorePendingActiveWork = ::restorePendingActiveWork,
            checkAndOfferGuestImport = ::checkAndOfferGuestImport,
            getWaitingForReply = { waitingForReply },
            getBackendConnected = { backendConnected },
            isActiveConversationWorking = ::isActiveConversationWorking,
            startTaskWorkService = { action -> startTaskWorkService(action) },
            openConversation = ::openConversation,
            loadModelOptions = { loadModelOptions() },
            sendMessage = ::sendMessage,
            handleLaunchIntent = ::handleLaunchIntent
        ).also { createActions = it }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleLaunchIntent(intent)
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
            setTaskAppForeground = ::setTaskAppForeground,
            drainQueuedTaskEvents = ::drainQueuedTaskEvents,
            loadModelOptions = { loadModelOptions() },
            getBackendConnected = { backendConnected },
            getWaitingForReply = { waitingForReply },
            getPendingReconnectForActiveWork = { pendingReconnectForActiveWork },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            currentStage = { currentStage },
            updateStage = ::updateStage,
            recordEvidence = ::recordEvidence,
            startTaskWorkService = { action -> startTaskWorkService(action) },
            isActiveConversationWorking = ::isActiveConversationWorking,
            setSendEnabled = ::setSendEnabled,
            maybePrewarmCodexSession = ::maybePrewarmCodexSession
        ).also { resumeActions = it }
    }

    override fun onPause() {
        appInForeground = false
        setTaskAppForeground(false)
        super.onPause()
    }

    override fun onStop() {
        appInForeground = false
        setTaskAppForeground(false)
        saveProjects()
        super.onStop()
    }

    private fun sendMessage() {
        sendMessageActions().sendMessage()
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
            restoreSendTarget = ::restoreSendTarget,
            isConversationTaskRunning = { target ->
                runningConversationTasks.containsKey(conversationTaskKey(target.projectId, target.conversationId))
            },
            setSendEnabled = ::setSendEnabled,
            userId = { userId },
            selectedAgentForRequest = { modelActions().selectedAgentForRequest() },
            appendMessage = ::appendMessage,
            collapseInputComposer = { collapseInputComposer() },
            looksLikeDevelopmentRequest = ::looksLikeDevelopmentRequest,
            looksLikeDirectImageRequest = ::looksLikeDirectImageRequest,
            rememberConversationTask = ::rememberConversationTask,
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            resetRequestState = {
                pendingReconnectForActiveWork = false
                reconnectAttempts = 0
                persistActiveWork()
                workflowStepIndex = 0
                resetFoldedCliLog()
                clearCurrentEvidence()
                toolActionBubbles().clear()
                progressNarrativeActions().clear()
            },
            acceptDevelopmentRequest = { text ->
                updateProjectTitleFromRequest(text)
                saveProjectTitle()
                addProjectEvent("提交需求：${summarize(text, 36)}")
                updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
            },
            updateProjectViews = ::updateProjectViews,
            nextServerResponseToken = { ++serverResponseToken },
            putTaskResponseToken = { traceId, token -> taskResponseTokens[traceId] = token },
            startTaskWorkService = ::startTaskWorkService,
            markTaskPendingReconnect = { target ->
                runningConversationTasks[conversationTaskKey(target.projectId, target.conversationId)]?.pendingReconnect = true
            },
            refreshActiveTaskState = ::refreshActiveTaskState,
            persistActiveWork = ::persistActiveWork,
            updateStage = ::updateStage,
            scheduleFirstServerResponseWatchdog = { traceId, token ->
                serverResponseWatchdogActions().scheduleFirstServerResponseWatchdog(traceId, token)
            },
            clearPendingAttachments = { clearPendingAttachments(deleteFiles = false) }
        ).also { preparedMessageActions = it }
    }

    private fun activeWorkControlActions(): MainActiveWorkControlActions {
        activeWorkControlActions?.let { return it }
        return MainActiveWorkControlActions(
            binding = binding,
            activeConversationTask = ::activeConversationTask,
            removeConversationTask = ::removeConversationTask,
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
            persistActiveWork = ::persistActiveWork,
            clearPersistedActiveWork = ::clearPersistedActiveWork,
            refreshActiveTaskState = ::refreshActiveTaskState,
            stopWorkingEvidenceForActiveConversation = ::stopWorkingEvidenceForActiveConversation,
            clearCurrentEvidence = ::clearCurrentEvidence,
            clearToolActionBubbles = { toolActionBubbles().clear() },
            setSendEnabled = ::setSendEnabled,
            updateFirstConversationStatus = ::updateFirstConversationStatus,
            updateStage = ::updateStage,
            updateProjectViews = ::updateProjectViews,
            addProjectEvent = ::addProjectEvent,
            recordEvidence = ::recordEvidence,
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
            toggleVoiceMode = ::toggleVoiceMode,
            focusInputComposer = ::focusInputComposer,
            startSpeechToText = ::startSpeechToText,
            stopSpeechToText = ::stopSpeechToText,
            showModelPopupOrLoad = ::showModelPopupOrLoad,
            sendMessage = ::sendMessage,
            toggleAttachmentPanel = ::toggleAttachmentPanel,
            buildAttachmentPanel = ::buildAttachmentPanel,
            collapseAttachmentPanel = ::collapseAttachmentPanel,
            collapseInputComposer = { collapseInputComposer() },
            updateCollapsedInputPreview = ::updateCollapsedInputPreview,
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = ::updateAdaptiveInputHeight
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
            updateCollapsedInputPreview()
            updateSendButtonVisual()
        }
        binding.inputLayout.addView(pendingAttachmentPreviewStrip.view, 1)
        applyVoiceMode()
        updateCollapsedInputPreview()
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun updateAdaptiveInputHeight() {
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

    private fun updateCollapsedInputPreview() {
        if (!::collapsedInputPreview.isInitialized) return
        val draft = binding.inputEdit.text?.toString().orEmpty()
        val hasDraft = draft.isNotBlank()
        val hasAttachments = pendingAttachments.isNotEmpty()
        collapsedInputPreview.text = when {
            hasDraft -> draft
            hasAttachments -> pendingAttachmentSummary(pendingAttachments)
            else -> "文本内容在此输入。"
        }
        collapsedInputPreview.setTextColor(
            Color.parseColor(if (hasDraft || hasAttachments) "#DCDCDC" else "#A8D0D0D0")
        )
    }

    private fun focusInputComposer() {
        inputFocusActions().focusInputComposer()
    }

    private fun collapseInputComposer(animate: Boolean = true) {
        inputFocusActions().collapseInputComposer(animate)
    }

    private fun inputFocusActions(): MainInputFocusActions {
        inputFocusActions?.let { return it }
        return MainInputFocusActions(
            activity = this,
            binding = binding,
            activeConversation = ::activeConversation,
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = ::applyVoiceMode,
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
            setSuppressInputFocusAnimation = { suppressInputFocusAnimation = it },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = ::updateAdaptiveInputHeight
        ).also { inputFocusActions = it }
    }

    private fun setupAttachmentLaunchers() {
        attachmentPickerActions().setupAttachmentLaunchers()
    }

    private fun buildAttachmentPanel(): LinearLayout {
        return attachmentPanelActions().buildAttachmentPanel()
    }

    private fun refreshPendingAttachmentPreview() {
        if (!::pendingAttachmentPreviewStrip.isInitialized) return
        pendingAttachmentPreviewStrip.refresh()
        updateCollapsedInputPreview()
        updateSendButtonVisual()
    }

    private fun openCameraAttachment() {
        attachmentPickerActions().openCameraAttachment()
    }

    private fun openPhotoAttachment() {
        attachmentPickerActions().openPhotoAttachment()
    }

    private fun openDocumentAttachment() {
        attachmentPickerActions().openDocumentAttachment()
    }

    private fun attachmentPickerActions(): MainAttachmentPickerActions {
        attachmentPickerActions?.let { return it }
        return MainAttachmentPickerActions(
            activity = this,
            activeConversation = ::activeConversation,
            attachPickedFile = ::attachPickedFile
        ).also { attachmentPickerActions = it }
    }

    private fun attachPickedFile(kind: String, uri: Uri, fallbackName: String? = null) {
        pendingAttachmentActions().attachPickedFile(kind, uri, fallbackName)
    }

    private fun sendMessageActions(): MainSendMessageActions {
        sendMessageActions?.let { return it }
        return MainSendMessageActions(
            binding = binding,
            pendingAttachments = pendingAttachments,
            collapseAttachmentPanel = ::collapseAttachmentPanel,
            isActiveConversationWorking = ::isActiveConversationWorking,
            activeProject = ::activeProject,
            activeConversation = ::activeConversation,
            appendMessage = ::appendMessage,
            collapseInputComposer = { collapseInputComposer() },
            uploadAttachmentsThenSend = ::uploadAttachmentsThenSend,
            startPreparedMessage = ::startPreparedMessage
        ).also { sendMessageActions = it }
    }

    private fun restoreSendTarget(target: SendTarget): Boolean {
        val projectIndex = projects.indexOfFirst { it.id == target.projectId }
        if (projectIndex < 0) return false
        val project = projects[projectIndex]
        val conversationIndex = project.conversations.indexOfFirst { it.id == target.conversationId }
        if (conversationIndex < 0) return false
        activeProjectIndex = projectIndex
        project.activeConversationIndex = conversationIndex
        chatAdapter = ChatAdapter(
            project.conversations[conversationIndex].messages,
            { activeWorkControlActions().pauseCurrentWork() },
            { anchor, message -> messageActions().showMessageActions(anchor, message) }
        )
        binding.chatList.adapter = chatAdapter
        showChat()
        if (chatAdapter.itemCount > 0) {
            binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
        }
        return true
    }

    private fun uploadAttachmentsThenSend(visibleText: String, outgoingText: String, target: SendTarget) {
        attachmentSendActions().uploadAttachmentsThenSend(visibleText, outgoingText, target)
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

    private fun clearPendingAttachments(deleteFiles: Boolean = true) {
        pendingAttachmentActions().clearPendingAttachments(deleteFiles)
    }

    private fun pendingAttachmentActions(): MainPendingAttachmentActions {
        pendingAttachmentActions?.let { return it }
        return MainPendingAttachmentActions(
            activity = this,
            pendingAttachments = pendingAttachments,
            isVoiceMode = { voiceMode },
            setVoiceMode = { voiceMode = it },
            applyVoiceMode = ::applyVoiceMode,
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
            refreshPendingAttachmentPreview = ::refreshPendingAttachmentPreview
        ).also { pendingAttachmentActions = it }
    }

    private fun handleSendOrAttachment() {
        if (!voiceMode && (binding.inputEdit.text.toString().trim().isNotEmpty() || pendingAttachments.isNotEmpty())) {
            sendMessage()
        } else {
            toggleAttachmentPanel()
        }
    }

    private fun toggleAttachmentPanel() {
        attachmentPanelActions().toggleAttachmentPanel()
    }

    private fun expandAttachmentPanel() {
        attachmentPanelActions().expandAttachmentPanel()
    }

    private fun collapseAttachmentPanel() {
        attachmentPanelActions().collapseAttachmentPanel()
    }

    private fun attachmentPanelActions(): MainAttachmentPanelActions {
        attachmentPanelActions?.let { return it }
        return MainAttachmentPanelActions(
            activity = this,
            dp = ::dp,
            selectableForeground = ::selectableForeground,
            activeConversation = ::activeConversation,
            attachmentPanel = ::attachmentPanelOrNull,
            attachmentButton = ::attachmentButtonOrNull,
            collapseInputComposer = { collapseInputComposer() },
            openCameraAttachment = ::openCameraAttachment,
            openPhotoAttachment = ::openPhotoAttachment,
            openDocumentAttachment = ::openDocumentAttachment
        ).also { attachmentPanelActions = it }
    }

    private fun attachmentPanelOrNull(): LinearLayout? {
        return if (::attachmentPanel.isInitialized) attachmentPanel else null
    }

    private fun attachmentButtonOrNull(): ImageButton? {
        return if (::attachmentButton.isInitialized) attachmentButton else null
    }

    private fun toggleVoiceMode() {
        voiceMode = !voiceMode
        collapseAttachmentPanel()
        applyVoiceMode()
    }

    private fun applyVoiceMode() {
        voiceModeActions().applyVoiceMode()
    }

    private fun voiceModeActions(): MainVoiceModeActions {
        voiceModeActions?.let { return it }
        return MainVoiceModeActions(
            binding = binding,
            hideKeyboard = ::hideKeyboard,
            inputModeButton = { if (::inputModeButton.isInitialized) inputModeButton else null },
            voiceHoldButton = { if (::voiceHoldButton.isInitialized) voiceHoldButton else null },
            inputCenterContainer = { if (::inputCenterContainer.isInitialized) inputCenterContainer else null },
            expandedInputContainer = { if (::expandedInputContainer.isInitialized) expandedInputContainer else null },
            collapsedInputPreview = { if (::collapsedInputPreview.isInitialized) collapsedInputPreview else null },
            modelButtonShell = { if (::modelButtonShell.isInitialized) modelButtonShell else null },
            inputComposerMotion = { if (::inputComposerMotion.isInitialized) inputComposerMotion else null },
            isVoiceMode = { voiceMode },
            updateSendButtonVisual = ::updateSendButtonVisual,
            updateAdaptiveInputHeight = ::updateAdaptiveInputHeight
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

    private fun startSpeechToText() {
        speechInputActions().startSpeechToText()
    }

    private fun stopSpeechToText() {
        speechInputActions().stopSpeechToText()
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
            applyVoiceMode = ::applyVoiceMode
        ).also { speechInputActions = it }
    }

    private fun hideKeyboard() {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.root.windowToken, 0)
        binding.inputEdit.clearFocus()
    }

    private fun setupNavigation() {
        navigationController().setupNavigation()
    }

    private fun setupBackHandling() {
        navigationController().setupBackHandling()
    }

    private fun showConversationHome(animate: Boolean = false) {
        navigationController().showConversationHome(animate)
    }

    private fun showChat(animate: Boolean = false) {
        navigationController().showChat(animate)
    }

    private fun navigationController(): MainNavigationController {
        navigationController?.let { return it }
        return MainNavigationController(
            activity = this,
            binding = binding,
            actionPopupProvider = { actionPopup },
            activeConversationProvider = ::activeConversation,
            activeConversationIndexProvider = { activeConversationIndex },
            compactProjectTitle = ::compactProjectTitle,
            renderConversationList = ::renderConversationList,
            renderProjectList = ::renderProjectList,
            refreshServerVersion = ::refreshServerVersion,
            openConversation = ::openConversation,
            showConversationActions = ::showConversationActions,
            showHomeActionPopup = ::showHomeActionPopup,
            showChatActionPopup = ::showChatActionPopup,
            updateFirstConversationStatus = ::updateFirstConversationStatus,
            collapseInputComposer = ::collapseInputComposer,
            isActiveConversationWorking = ::isActiveConversationWorking,
            setSendEnabled = ::setSendEnabled,
            maybePrewarmCodexSession = ::maybePrewarmCodexSession
        ).also { navigationController = it }
    }

    private fun showCreateConversationDialog() {
        conversationActions().showCreateConversationDialog()
    }

    private fun showCreateProjectDialog() {
        projectActions().showCreateProjectDialog()
    }

    private fun showConversationActions(index: Int) {
        conversationActions().showConversationActions(index)
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

    private fun restoreCachedModelSelection() {
        modelActions().restoreCachedModelSelection()
    }

    private fun loadModelOptions(afterLoad: (() -> Unit)? = null) {
        modelActions().loadModelOptions(afterLoad)
    }

    private fun showModelPopupOrLoad() {
        modelActions().showModelPopupOrLoad()
    }

    private fun updateModelButton() {
        modelActions().updateModelButton()
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
            modelButtonShellProvider = ::modelButtonShellOrNull,
            inputBarContainerProvider = ::inputBarContainerOrNull,
            getActionPopup = { actionPopup },
            setActionPopup = { actionPopup = it },
            openSettings = { quickCommandActions().openSettings() },
            dp = ::dp,
            selectableForeground = ::selectableForeground
        ).also { modelActions = it }
    }

    private fun modelButtonShellOrNull(): FrameLayout? {
        return if (::modelButtonShell.isInitialized) modelButtonShell else null
    }

    private fun inputBarContainerOrNull(): LinearLayout? {
        return if (::inputBarContainer.isInitialized) inputBarContainer else null
    }

    private fun openConversation(index: Int) {
        if (conversations.isEmpty()) conversations.add(defaultAppConversation())
        activeConversationIndex = index.coerceIn(0, conversations.lastIndex)
        chatAdapter = ChatAdapter(
            activeConversation().messages,
            { activeWorkControlActions().pauseCurrentWork() },
            { anchor, message -> messageActions().showMessageActions(anchor, message) }
        )
        binding.chatList.adapter = chatAdapter
        showChat(animate = true)
        if (chatAdapter.itemCount > 0) {
            binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
        }
    }

    private fun openProject(index: Int) {
        if (index !in projects.indices) return
        activeProjectIndex = index
        if (conversations.isEmpty()) conversations.add(defaultAppConversation())
        activeConversationIndex = activeConversationIndex.coerceIn(0, conversations.lastIndex)
        saveProjects()
        binding.tabChat.performClick()
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
        val loaded = loadStoredProjects(prefs, gson, ::normalizeProject)
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

    private fun conversationTaskKey(projectId: String, conversationId: String): String {
        return conversationTaskRegistryActions().conversationTaskKey(projectId, conversationId)
    }

    private fun activeConversationTaskKey(): String {
        return conversationTaskRegistryActions().activeConversationTaskKey()
    }

    private fun isActiveConversationWorking(): Boolean {
        return conversationTaskRegistryActions().isActiveConversationWorking()
    }

    private fun activeConversationTask(): ConversationTaskState? {
        return conversationTaskRegistryActions().activeConversationTask()
    }

    private fun rememberConversationTask(
        target: SendTarget,
        traceId: String,
        payload: String,
        isDevelopment: Boolean
    ) {
        conversationTaskRegistryActions().rememberConversationTask(target, traceId, payload, isDevelopment)
    }

    private fun updateConversationTaskFromService(
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?,
        pendingReconnect: Boolean? = null
    ): ConversationTaskState? {
        return conversationTaskRegistryActions().updateConversationTaskFromService(
            traceId,
            projectId,
            conversationId,
            isDevelopment,
            pendingReconnect
        )
    }

    private fun removeConversationTask(
        traceId: String?,
        projectId: String?,
        conversationId: String?
    ): ConversationTaskState? {
        return conversationTaskRegistryActions().removeConversationTask(traceId, projectId, conversationId)
    }

    private fun refreshActiveTaskState() {
        conversationTaskRegistryActions().refreshActiveTaskState()
    }

    private fun persistActiveWork() {
        conversationTaskRegistryActions().persistActiveWork()
    }

    private fun clearPersistedActiveWork() {
        conversationTaskRegistryActions().clearPersistedActiveWork()
    }

    private fun restorePendingActiveWork() {
        conversationTaskRegistryActions().restorePendingActiveWork()
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

    private fun registerTaskWorkReceiver() {
        if (taskWorkReceiverRegistered) return
        val filter = IntentFilter().apply {
            addAction(TaskWorkService.ACTION_EVENT)
            addAction(TaskWorkService.ACTION_STATE)
        }
        ContextCompat.registerReceiver(
            this,
            taskWorkReceiver,
            filter,
            ContextCompat.RECEIVER_NOT_EXPORTED
        )
        taskWorkReceiverRegistered = true
    }

    private fun handleTaskWorkEvent(intent: Intent) {
        taskWorkEventActions().handleTaskWorkEvent(intent)
    }

    private fun taskWorkEventActions(): MainTaskWorkEventActions {
        taskWorkEventActions?.let { return it }
        return MainTaskWorkEventActions(
            getBackendConnected = { backendConnected },
            setBackendConnected = { backendConnected = it },
            getWaitingForReply = { waitingForReply },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            updateFirstConversationStatus = ::updateFirstConversationStatus,
            updateConversationTaskFromService = { traceId, projectId, conversationId, isDevelopment, pendingReconnect ->
                updateConversationTaskFromService(traceId, projectId, conversationId, isDevelopment, pendingReconnect)
            },
            activeConversationTask = ::activeConversationTask,
            recordEvidence = ::recordEvidence,
            setSendEnabled = ::setSendEnabled,
            isActiveConversationWorking = ::isActiveConversationWorking,
            handleActiveWorkDisconnected = { task -> activeWorkControlActions().handleActiveWorkDisconnected(task) },
            updateIdleReadyStatus = ::updateIdleReadyStatus,
            appendTaskMessage = { raw, traceId, projectId, conversationId, isDevelopment ->
                traceId?.let { taskResponseTokens.remove(it) }
                appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
            },
            removeConversationTask = ::removeConversationTask,
            syncActiveTasksFromServiceState = ::syncActiveTasksFromServiceState,
            clearTaskMaps = {
                runningConversationTasks.clear()
                runningTraceToConversation.clear()
                taskResponseTokens.clear()
            },
            refreshActiveTaskState = ::refreshActiveTaskState
        ).also { taskWorkEventActions = it }
    }

    private fun syncActiveTasksFromServiceState(activeTasksJson: String?) {
        conversationTaskRegistryActions().syncActiveTasksFromServiceState(activeTasksJson)
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
            conversationTaskKey = ::conversationTaskKey,
            activeConversationTaskKey = ::activeConversationTaskKey,
            taskIsDevelopment = { key -> runningConversationTasks[key]?.isDevelopment },
            appendActiveMessage = { raw -> appendMessage(raw) },
            appendBackgroundTaskMessage = ::appendBackgroundTaskMessage,
            removeConversationTask = ::removeConversationTask,
            persistActiveWork = ::persistActiveWork,
            updateConversationTaskFromService = { traceId, projectId, conversationId, isDevelopment, pendingReconnect ->
                updateConversationTaskFromService(traceId, projectId, conversationId, isDevelopment, pendingReconnect)
            }
        ).also { taskMessageRouterActions = it }
    }

    private fun appendBackgroundTaskMessage(raw: String, key: String?, isDevelopment: Boolean) {
        backgroundTaskMessageActions().appendBackgroundTaskMessage(raw, key, isDevelopment)
    }

    private fun backgroundTaskMessageActions(): MainBackgroundTaskMessageActions {
        backgroundTaskMessageActions?.let { return it }
        return MainBackgroundTaskMessageActions(
            activity = this,
            findConversationLocationByKey = ::findConversationLocationByKey,
            appendMessageToConversation = ::appendMessageToConversation
        ).also { backgroundTaskMessageActions = it }
    }

    private fun findConversationLocationByKey(key: String): Pair<Int, Int>? {
        return conversationPreviewActions().findConversationLocationByKey(key)
    }

    private fun appendMessageToConversation(
        projectIndex: Int,
        conversationIndex: Int,
        message: ChatMessage
    ) {
        conversationPreviewActions().appendMessageToConversation(projectIndex, conversationIndex, message)
    }

    private fun handleLaunchIntent(intent: Intent?) {
        if (intent?.getBooleanExtra(TaskWorkService.EXTRA_SHOW_APP_UPDATE, false) == true) {
            intent.removeExtra(TaskWorkService.EXTRA_SHOW_APP_UPDATE)
            AppUpdateManager(this).realtimeCheck()
        }
    }

    private fun startTaskWorkService(
        action: String,
        payload: String? = null,
        isDevelopment: Boolean = activeRequestIsDevelopment,
        traceId: String? = null
    ): Boolean {
        return taskWorkServiceActions().startTaskWorkService(action, payload, isDevelopment, traceId)
    }

    private fun setTaskAppForeground(foreground: Boolean) {
        taskWorkServiceActions().setTaskAppForeground(foreground)
    }

    private fun drainQueuedTaskEvents() {
        taskWorkServiceActions().drainQueuedTaskEvents()
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

    private fun normalizeProject(project: AppProject) {
        projectHygieneActions().normalizeProject(project)
    }

    private fun updateFirstConversationStatus(text: String) {
        conversationPreviewActions().updateFirstConversationStatus(text)
    }

    private fun updateIdleReadyStatus() {
        conversationPreviewActions().updateIdleReadyStatus()
    }

    private fun updateActiveConversationPreview(message: ChatMessage) {
        conversationPreviewActions().updateActiveConversationPreview(message)
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
            conversationTaskKey = ::conversationTaskKey,
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

    private fun showProjectActions(index: Int) {
        if (index !in projects.indices) return
        projectActions().showProjectActions(index)
    }

    private fun isConversationWorking(index: Int): Boolean {
        return homeListActions().isConversationWorking(index)
    }

    private fun homeListActions(): MainHomeListActions {
        homeListActions?.let { return it }
        return MainHomeListActions(
            activity = this,
            binding = binding,
            projects = { projects },
            conversations = { conversations },
            activeProject = ::activeProject,
            compactProjectTitle = ::compactProjectTitle,
            formatTime = { timeFormatter.format(Date(it)) },
            isTaskRunning = { projectId, conversationId ->
                runningConversationTasks.containsKey(conversationTaskKey(projectId, conversationId))
            },
            homeRows = { homeRows() },
            dp = ::dp,
            selectableForeground = ::selectableForeground,
            showCreateProjectDialog = ::showCreateProjectDialog
        ).also { homeListActions = it }
    }

    private fun homeRows(): MainHomeRows {
        homeRows?.let { return it }
        return MainHomeRows(
            activity = this,
            timeFormatter = timeFormatter,
            activeProjectIndexProvider = { activeProjectIndex },
            openProject = ::openProject,
            showProjectActions = ::showProjectActions,
            openConversation = ::openConversation,
            showConversationActions = ::showConversationActions,
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
            openProject = ::openProject,
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

    private fun setupQuickActions() {
        profileQuickActions().setupQuickActions()
    }

    private fun refreshAccountUi() {
        if (!::binding.isInitialized) return
        accountActions().refreshAccountUi()
    }

    private fun checkAndOfferGuestImport() {
        accountActions().checkAndOfferGuestImport()
    }

    private fun showGuestImportDialog() {
        accountActions().showGuestImportDialog()
    }

    private fun confirmLogout() {
        accountActions().confirmLogout()
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

    private fun refreshServerVersion() {
        profileQuickActions().refreshServerVersion()
    }

    private fun profileQuickActions(): MainProfileQuickActions {
        profileQuickActions?.let { return it }
        return MainProfileQuickActions(
            activity = this,
            binding = binding,
            http = http,
            serverVersionUrl = serverVersionUrl,
            isBindingInitialized = { ::binding.isInitialized },
            refreshAccountUi = ::refreshAccountUi,
            fillPlanPrompt = { quickCommandActions().fillPlanPrompt() },
            sendQuickCommand = { text -> quickCommandActions().sendQuickCommand(text) },
            showProjectRecordDialog = ::showProjectRecordDialog,
            showGitProjectDialog = ::showGitProjectDialog,
            openSettings = { quickCommandActions().openSettings() },
            showPromotionDialog = { messageActions().showPromotionDialog() },
            showGuestImportDialog = ::showGuestImportDialog,
            confirmLogout = ::confirmLogout
        ).also { profileQuickActions = it }
    }

    private fun showHomeActionPopup(anchor: View, tab: TextView) {
        actionPopups().showHomeActionPopup(anchor, tab)
    }

    private fun showChatActionPopup(anchor: View) {
        actionPopups().showChatActionPopup(anchor)
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
            showProjectRecordDialog = ::showProjectRecordDialog,
            showGitProjectDialog = ::showGitProjectDialog,
            showCreateProjectDialog = ::showCreateProjectDialog,
            showCreateConversationDialog = ::showCreateConversationDialog,
            openSettings = { quickCommandActions().openSettings() },
            deleteMessage = { message -> messageActions().deleteMessage(message) },
            quoteMessage = { text -> messageActions().quoteMessage(text) },
            dp = ::dp,
            selectableForeground = ::selectableForeground
        ).also { actionPopups = it }
    }

    private fun showProjectRecordDialog() {
        projectRecordActions().showProjectRecordDialog()
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
            isActiveConversationWorking = ::isActiveConversationWorking,
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
            showCreateConversationDialog = ::showCreateConversationDialog,
            showChat = { showChat() },
            sendMessage = ::sendMessage
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
            showChat = { showChat() },
            showMessageActionPopup = { anchor, message, text ->
                actionPopups().showMessageActionPopup(anchor, message, text)
            },
            shareActions = ::shareActions,
            apkDownloadUrl = { apkDownloadUrl },
            apkDownloadPageUrl = { apkDownloadPageUrl }
        ).also { messageActions = it }
    }

    private fun nextWorkflowStep(label: String): String {
        workflowStepIndex += 1
        return "步骤 $workflowStepIndex · $label"
    }

    private fun workflowStoppedMessage(reason: String, wasDevelopment: Boolean = activeRequestIsDevelopment): String {
        val stage = if (wasDevelopment) "需要处理" else "回复中断"
        return "工作停止：$stage。原因：$reason"
    }

    private fun resetFoldedCliLog() {
        foldedCliLogActions().reset()
    }

    private fun recordEvidence(kind: String, detail: String) {
        if (!activeRequestIsDevelopment) return
        evidenceActions().recordEvidence(kind, detail)
    }

    private fun attachEvidenceToLatestAi() {
        evidenceActions().attachEvidenceToLatestAi()
    }

    private fun finalizeEvidenceForLatestAssistant() {
        evidenceActions().finalizeEvidenceForLatestAssistant()
    }

    private fun aiMessageWithCurrentEvidence(
        content: String,
        attachments: List<ChatAttachment> = emptyList()
    ): ChatMessage {
        return evidenceActions().aiMessageWithCurrentEvidence(content, attachments)
    }

    private fun stopWorkingEvidenceForActiveConversation() {
        evidenceActions().stopWorkingEvidenceForActiveConversation()
    }

    private fun clearCurrentEvidence() {
        evidenceActions().clearCurrentEvidence()
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
            recordEvidence = ::recordEvidence
        ).also { foldedCliLogActions = it }
    }

    private fun progressNarrativeActions(): MainProgressNarrativeActions {
        progressNarrativeActions?.let { return it }
        return MainProgressNarrativeActions(
            isDevelopmentRequest = { activeRequestIsDevelopment },
            finalizeEvidenceForLatestAssistant = ::finalizeEvidenceForLatestAssistant,
            appendMessage = ::appendMessage,
            attachEvidenceToLatestAi = ::attachEvidenceToLatestAi
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
            activeConversationTask = ::activeConversationTask,
            getCurrentStage = { currentStage },
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            refreshActiveTaskState = ::refreshActiveTaskState,
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
        updateActiveConversationPreview(msg)
        binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
    }

    private fun removeTransientWorkflowMessagesAfterLatestUser() {
        if (workflowMessageCompactor().removeTransientWorkflowMessagesAfterLatestUser(activeConversation().messages)) {
            chatAdapter.notifyDataSetChanged()
            saveConversations()
        }
    }

    private fun handleProgress(content: String, recordProgressEvidence: Boolean = true) {
        workflowStageActions().handleProgress(content, recordProgressEvidence)
    }

    private fun handleTaskEvent(event: String, taskId: String?, content: String) {
        workflowStageActions().handleTaskEvent(event, taskId, content)
    }

    private fun handleToolCall(tool: String) {
        workflowStageActions().handleToolCall(tool)
    }

    private fun assistantStreamEvents(): MainAssistantStreamEvents {
        assistantStreamEvents?.let { return it }
        return MainAssistantStreamEvents(
            handleTaskEvent = ::handleTaskEvent,
            maybeAppendTaskEventNarrative = { event, content ->
                progressNarrativeActions().maybeAppendTaskEventNarrative(event, content)
            },
            maybeAppendWorkflowProgressNarrative = { content ->
                progressNarrativeActions().maybeAppendWorkflowProgressNarrative(content)
            },
            maybeAppendToolCallNarrative = { tool ->
                progressNarrativeActions().maybeAppendToolCallNarrative(tool)
            },
            handleProgress = ::handleProgress,
            handleFoldedCliOutput = { content -> foldedCliLogActions().handleFoldedCliOutput(content) },
            markToolCallStarted = ::handleToolCall,
            appendToolCallBubble = { tool, args -> toolActionBubbles().appendToolCallBubble(tool, args) },
            markToolResultDone = { toolActionBubbles().markToolResultDone(it) },
            markToolResult = { workflowStageActions().markToolResult(it) },
            recordEvidence = ::recordEvidence,
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
            clearPersistedActiveWork = ::clearPersistedActiveWork,
            updateStage = ::updateStage,
            updateProjectViews = ::updateProjectViews,
            addProjectEvent = ::addProjectEvent,
            recordEvidence = ::recordEvidence,
            stopWorkingEvidenceForActiveConversation = ::stopWorkingEvidenceForActiveConversation,
            clearCurrentEvidence = ::clearCurrentEvidence,
            resetFoldedCliLog = ::resetFoldedCliLog,
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
            recordEvidence = ::recordEvidence
        ).also { workflowStageActions = it }
    }

    private fun updateStageHintShimmer() {
        stageHintShimmer().update()
    }

    private fun stopStageHintShimmer() {
        stageHintShimmer?.stop()
    }

    private fun stageHintShimmer(): MainStageHintShimmer {
        stageHintShimmer?.let { return it }
        return MainStageHintShimmer(
            binding = binding,
            isActiveConversationWorking = ::isActiveConversationWorking
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
            updateStageHintShimmer = ::updateStageHintShimmer
        ).also { projectViewActions = it }
    }

    private fun compactProjectTitle(): String {
        return projectRecordActions().compactProjectTitle()
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

    private fun saveProjectTitle() {
        projectRecordActions().saveProjectTitle()
    }

    private fun updateProjectTitleFromRequest(text: String) {
        projectRecordActions().updateProjectTitleFromRequest(text)
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
            updateStageHintShimmer = ::updateStageHintShimmer
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
            stopStageHintShimmer = ::stopStageHintShimmer,
            cancelHomeRowShimmer = {
                homeRows?.cancelHomeRowShimmer()
                homeRows = null
            },
            destroySpeechInput = {
                speechInputActions?.destroy()
                speechInputActions = null
            },
            isTaskWorkReceiverRegistered = { taskWorkReceiverRegistered },
            unregisterTaskWorkReceiver = {
                unregisterReceiver(taskWorkReceiver)
                taskWorkReceiverRegistered = false
            }
        ).also { lifecycleEdgeActions = it }
    }
}

