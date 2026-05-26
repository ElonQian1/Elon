package com.elon.app

import android.content.Intent
import android.os.Bundle
import android.view.Menu
import android.view.MenuItem
import android.view.MotionEvent
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
    private val friends = mutableListOf<AppFriend>()
    private val gson = com.google.gson.Gson()
    private val http = OkHttpClient()
    private val timeFormatter = SimpleDateFormat("HH:mm", Locale.CHINA)
    private val prefs by lazy { AuthManager.userDataPrefs(this) }
    private val serverUrl = "http://43.139.149.158:8080"
    private val apkDownloadUrl: String get() = "$serverUrl/app/ElonSpeed-latest.apk"
    private val apkDownloadPageUrl: String get() = "$serverUrl/app/download"
    private val serverVersionUrl: String get() = "$serverUrl/api/server/version"
    private var activeProjectIndex = 0
    private var homeRows: MainHomeRows? = null
    private var stageHintShimmer: MainStageHintShimmer? = null
    private var actionPopup: PopupWindow? = null
    private val speechPermissionRequest = 4301
    private val notificationPermissionRequest = 4302
    /**
     * 当前会话使用的 user_id。
     * - 已登录：使用服务端返回的 user.id（跨设备稳定）。
     * - 未登录（游客）：使用本机随机 UUID（与老版本兼容）。
     * by lazy 在 Activity 实例生命周期内固定；登录/登出后会清掉栈重建 MainActivity。
     */
    private val userId: String by lazy { AuthManager.effectiveUserId(this) }

    private val workflowActions: MainWorkflowActions by lazy {
        MainWorkflowActions(
            activity = this,
            binding = binding,
            chatAdapter = { chatAdapter },
            activeRequestIsDevelopment = { activeRequestIsDevelopment },
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            setWaitingForReply = { waitingForReply = it },
            clearPendingRequestPayload = { pendingRequestPayload = null },
            clearPendingReconnectForActiveWork = { pendingReconnectForActiveWork = false },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            incrementServerResponseToken = { serverResponseToken += 1 },
            currentTimeText = { timeFormatter.format(Date()) },
            taskResponseTokens = taskResponseTokens,
            runningTraceToConversation = runningTraceToConversation,
            runningConversationTasks = runningConversationTasks,
            projectStateActions = { projectStateActions },
            projectViewActions = { projectViewActions },
            projectRecordActions = { projectRecordActions },
            conversationPreviewActions = { conversationPreviewActions },
            conversationTaskRegistryActions = { conversationTaskRegistryActions },
            taskWorkServiceActions = { taskActions.taskWorkServiceActions },
            sendEnabledActions = { inputActions.sendEnabledActions }
        )
    }

    private val taskActions: MainTaskActions by lazy {
        MainTaskActions(
            activity = this,
            prefs = prefs,
            backendConnected = { backendConnected },
            setBackendConnected = { backendConnected = it },
            waitingForReply = { waitingForReply },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            taskResponseTokens = taskResponseTokens,
            runningTraceToConversation = runningTraceToConversation,
            runningConversationTasks = runningConversationTasks,
            activeRequestIsDevelopment = { activeRequestIsDevelopment },
            workflowActions = { workflowActions },
            conversationPreviewActions = { conversationPreviewActions },
            conversationTaskRegistryActions = { conversationTaskRegistryActions },
            activeWorkControlActions = { activeWorkControlActions },
            sendEnabledActions = { inputActions.sendEnabledActions }
        )
    }

    private val inputActions: MainInputActions by lazy {
        MainInputActions(
            activity = this,
            binding = binding,
            http = http,
            serverUrl = serverUrl,
            speechPermissionRequest = speechPermissionRequest,
            userId = { userId },
            projects = projects,
            setActiveProjectIndex = { activeProjectIndex = it },
            setChatAdapter = { chatAdapter = it },
            uiTools = { uiTools },
            modelActions = { modelActions },
            projectStateActions = { projectStateActions },
            conversationTaskRegistryActions = { conversationTaskRegistryActions },
            workflowActions = { workflowActions },
            preparedMessageActions = { preparedMessageActions },
            activeWorkControlActions = { activeWorkControlActions },
            messageActions = { messageActions },
            navigationController = { navigationController },
            stageHintShimmer = { stageHintShimmer() }
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)
        createActions.onCreate(intent)
    }

    private val createActions: MainCreateActions by lazy {
        MainCreateActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            notificationPermissionRequest = notificationPermissionRequest,
            loadProjects = projectStateActions::loadProjects,
            setupAttachmentLaunchers = { inputActions.attachmentPickerActions.setupAttachmentLaunchers() },
            activeConversation = projectStateActions::activeConversation,
            pauseCurrentWork = { activeWorkControlActions.pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions.showMessageActions(anchor, message) },
            retryFailedAttachmentMessage = { message -> inputActions.retryFailedAttachmentMessage(message) },
            setChatAdapter = { chatAdapter = it },
            setupNavigation = { navigationController.setupNavigation() },
            setupQuickActions = { profileQuickActions.setupQuickActions() },
            setupBackHandling = { navigationController.setupBackHandling() },
            setupInputComposer = inputActions::setupInputComposer,
            setupChatSideMenu = { chatSideMenuController.setup() },
            restoreCachedModelSelection = { modelActions.restoreCachedModelSelection() },
            updateProjectViews = projectViewActions::updateProjectViews,
            setTaskAppForeground = { foreground -> taskActions.taskWorkServiceActions.setTaskAppForeground(foreground) },
            registerTaskWorkReceiver = { taskActions.taskWorkReceiverActions.registerTaskWorkReceiver() },
            restorePendingActiveWork = { conversationTaskRegistryActions.restorePendingActiveWork() },
            checkAndOfferGuestImport = { accountActions().checkAndOfferGuestImport() },
            getWaitingForReply = { waitingForReply },
            getBackendConnected = { backendConnected },
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking,
            startTaskWorkService = { action ->
                taskActions.taskWorkServiceActions.startTaskWorkService(action, isDevelopment = activeRequestIsDevelopment)
            },
            openConversation = conversationOpenActions::openConversation,
            loadModelOptions = { modelActions.loadModelOptions() },
            sendMessage = { inputActions.sendMessageActions.sendMessage() }
        )
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        createActions.handleLaunchIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        resumeActions.onResume()
    }

    private val resumeActions: MainResumeActions by lazy {
        MainResumeActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            isBindingInitialized = { ::binding.isInitialized },
            setAppInForeground = { appInForeground = it },
            setTaskAppForeground = { foreground -> taskActions.taskWorkServiceActions.setTaskAppForeground(foreground) },
            drainQueuedTaskEvents = { taskActions.taskWorkServiceActions.drainQueuedTaskEvents() },
            loadModelOptions = { modelActions.loadModelOptions() },
            getBackendConnected = { backendConnected },
            getWaitingForReply = { waitingForReply },
            getPendingReconnectForActiveWork = { pendingReconnectForActiveWork },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            currentStage = { projectStateActions.currentStage },
            updateStage = projectViewActions::updateStage,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) workflowActions.evidenceActions.recordEvidence(kind, detail)
            },
            startTaskWorkService = { action ->
                taskActions.taskWorkServiceActions.startTaskWorkService(action, isDevelopment = activeRequestIsDevelopment)
            },
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking,
            setSendEnabled = inputActions.sendEnabledActions::setSendEnabled,
            maybePrewarmCodexSession = codexPrewarm::maybePrewarmCodexSession
        )
    }

    override fun onPause() {
        appInForeground = false
        taskActions.taskWorkServiceActions.setTaskAppForeground(false)
        super.onPause()
    }

    override fun onStop() {
        appInForeground = false
        taskActions.taskWorkServiceActions.setTaskAppForeground(false)
        projectStateActions.saveProjects()
        super.onStop()
    }

    private val preparedMessageActions: MainPreparedMessageActions by lazy {
        MainPreparedMessageActions(
            activity = this,
            binding = binding,
            restoreSendTarget = { target -> inputActions.sendTargetRestoreActions.restoreSendTarget(target) },
            isConversationTaskRunning = { target ->
                val key = conversationTaskRegistryActions.conversationTaskKey(target.projectId, target.conversationId)
                runningConversationTasks.containsKey(key)
            },
            setSendEnabled = inputActions.sendEnabledActions::setSendEnabled,
            userId = { userId },
            selectedAgentForRequest = { modelActions.selectedAgentForRequest() },
            appendMessage = workflowActions.messageAppendActions::appendMessage,
            collapseInputComposer = { inputActions.inputFocusActions.collapseInputComposer() },
            looksLikeDevelopmentRequest = ::looksLikeDevelopmentRequest,
            looksLikeDirectImageRequest = ::looksLikeDirectImageRequest,
            rememberConversationTask = conversationTaskRegistryActions::rememberConversationTask,
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            resetRequestState = {
                pendingReconnectForActiveWork = false
                reconnectAttempts = 0
                conversationTaskRegistryActions.persistActiveWork()
                workflowActions.foldedCliLogActions.reset()
                workflowActions.evidenceActions.clearCurrentEvidence()
                workflowActions.progressNarrativeActions.clear()
            },
            acceptDevelopmentRequest = { text ->
                projectRecordActions.updateProjectTitleFromRequest(text)
                projectRecordActions.saveProjectTitle()
                projectRecordActions.addProjectEvent("提交需求：${summarize(text, 36)}")
                projectViewActions.updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
            },
            updateProjectViews = projectViewActions::updateProjectViews,
            nextServerResponseToken = { ++serverResponseToken },
            putTaskResponseToken = { traceId, token -> taskResponseTokens[traceId] = token },
            startTaskWorkService = taskActions.taskWorkServiceActions::startTaskWorkService,
            markTaskPendingReconnect = { target ->
                val key = conversationTaskRegistryActions.conversationTaskKey(target.projectId, target.conversationId)
                runningConversationTasks[key]?.pendingReconnect = true
            },
            refreshActiveTaskState = conversationTaskRegistryActions::refreshActiveTaskState,
            persistActiveWork = conversationTaskRegistryActions::persistActiveWork,
            updateStage = projectViewActions::updateStage,
            scheduleFirstServerResponseWatchdog = { traceId, token ->
                workflowActions.serverResponseWatchdogActions.scheduleFirstServerResponseWatchdog(traceId, token)
            },
            clearPendingAttachments = {
                inputActions.pendingAttachmentActions.clearPendingAttachments(deleteFiles = false)
            }
        )
    }

    private val activeWorkControlActions: MainActiveWorkControlActions by lazy {
        MainActiveWorkControlActions(
            binding = binding,
            activeConversationTask = conversationTaskRegistryActions::activeConversationTask,
            removeConversationTask = conversationTaskRegistryActions::removeConversationTask,
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
            getCurrentStage = { projectStateActions.currentStage },
            getPendingRequestPayload = { pendingRequestPayload },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            setWaitingForReply = { waitingForReply = it },
            persistActiveWork = conversationTaskRegistryActions::persistActiveWork,
            clearPersistedActiveWork = conversationTaskRegistryActions::clearPersistedActiveWork,
            refreshActiveTaskState = conversationTaskRegistryActions::refreshActiveTaskState,
            stopWorkingEvidenceForActiveConversation = {
                workflowActions.evidenceActions.stopWorkingEvidenceForActiveConversation()
            },
            clearCurrentEvidence = { workflowActions.evidenceActions.clearCurrentEvidence() },
            setSendEnabled = inputActions.sendEnabledActions::setSendEnabled,
            updateFirstConversationStatus = { text ->
                conversationPreviewActions.updateFirstConversationStatus(text)
            },
            updateStage = projectViewActions::updateStage,
            updateProjectViews = projectViewActions::updateProjectViews,
            addProjectEvent = projectRecordActions::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment) workflowActions.evidenceActions.recordEvidence(kind, detail)
            },
            appendMessage = workflowActions.messageAppendActions::appendMessage,
            workflowStoppedMessage = ::mainWorkflowStoppedMessage,
            startTaskWorkService = taskActions.taskWorkServiceActions::startTaskWorkService,
            nextServerResponseToken = { ++serverResponseToken },
            scheduleFirstServerResponseWatchdog = { traceId, token ->
                workflowActions.serverResponseWatchdogActions.scheduleFirstServerResponseWatchdog(traceId, token)
            }
        )
    }

    private val navigationController: MainNavigationController by lazy {
        MainNavigationController(
            activity = this,
            binding = binding,
            actionPopupProvider = { actionPopup },
            activeConversationProvider = projectStateActions::activeConversation,
            activeConversationIndexProvider = { projectStateActions.activeConversationIndex },
            compactProjectTitle = { projectRecordActions.compactProjectTitle() },
            renderConversationList = homeListActions::renderConversationList,
            renderProjectList = homeListActions::renderProjectList,
            refreshServerVersion = { profileQuickActions.refreshServerVersion() },
            openConversation = conversationOpenActions::openConversation,
            showConversationActions = { index -> conversationActions.showConversationActions(index) },
            showHomeActionPopup = { anchor, tab -> actionPopups.showHomeActionPopup(anchor, tab) },
            showChatActionPopup = { anchor -> actionPopups.showChatActionPopup(anchor) },
            showAddFriendDialog = { friendActions.showAddFriendDialog() },
            refreshFriends = { friendActions.loadFriends() },
            updateFirstConversationStatus = { text ->
                conversationPreviewActions.updateFirstConversationStatus(text)
            },
            collapseInputComposer = { animate -> inputActions.inputFocusActions.collapseInputComposer(animate) },
            isChatSideMenuOpen = { chatSideMenuController.isOpen },
            closeChatSideMenu = { animate -> chatSideMenuController.close(animate) },
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking,
            setSendEnabled = inputActions.sendEnabledActions::setSendEnabled,
            maybePrewarmCodexSession = codexPrewarm::maybePrewarmCodexSession
        )
    }

    private val chatSideMenuController: ChatSideMenuController by lazy {
        ChatSideMenuController(
            activity = this,
            binding = binding,
            activeConversation = projectStateActions::activeConversation,
            conversations = { projectStateActions.conversations },
            activeConversationIndex = { projectStateActions.activeConversationIndex },
            openConversation = conversationOpenActions::openConversation,
            isConversationWorking = homeListActions::isConversationWorking,
            confirmLogout = { accountActions().confirmLogout() },
            dismissActionPopup = {
                actionPopup?.dismiss()
                actionPopup = null
            },
            cancelChildTouch = ::cancelActiveChildTouch,
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground
        )
    }

    private val conversationActions: MainConversationActions by lazy {
        MainConversationActions(
            activity = this,
            binding = binding,
            conversationsProvider = { projectStateActions.conversations },
            activeProjectProvider = projectStateActions::activeProject,
            activeConversationIndexProvider = { projectStateActions.activeConversationIndex },
            setActiveConversationIndex = { projectStateActions.activeConversationIndex = it },
            chatAdapterProvider = { chatAdapter },
            titleEditText = { value -> mainTitleEditText(this, value, uiTools::dp) },
            saveConversations = projectStateActions::saveConversations,
            renderConversationList = homeListActions::renderConversationList,
            setSendEnabled = inputActions.sendEnabledActions::setSendEnabled
        )
    }

    private val modelActions: MainModelActions by lazy {
        MainModelActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            http = http,
            serverUrl = serverUrl,
            userIdProvider = { userId },
            modelButtonShellProvider = { inputActions.inputComposerViewsOrNull()?.modelButtonShell },
            inputBarContainerProvider = { inputActions.inputComposerViewsOrNull()?.inputBarContainer },
            getActionPopup = { actionPopup },
            setActionPopup = { actionPopup = it },
            openSettings = { quickCommandActions.openSettings() },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground
        )
    }

    private val conversationOpenActions: MainConversationOpenActions by lazy {
        MainConversationOpenActions(
            binding = binding,
            projects = { projects },
            conversations = { projectStateActions.conversations },
            activeConversation = projectStateActions::activeConversation,
            activeConversationIndex = { projectStateActions.activeConversationIndex },
            setActiveProjectIndex = { activeProjectIndex = it },
            setActiveConversationIndex = { projectStateActions.activeConversationIndex = it },
            setChatAdapter = { chatAdapter = it },
            pauseCurrentWork = { activeWorkControlActions.pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions.showMessageActions(anchor, message) },
            retryFailedAttachmentMessage = { message -> inputActions.retryFailedAttachmentMessage(message) },
            showChat = { animate -> navigationController.showChat(animate = animate) },
            saveProjects = projectStateActions::saveProjects
        )
    }

    private val projectStateActions: MainProjectStateActions by lazy {
        MainProjectStateActions(
            prefs = prefs,
            gson = gson,
            projects = projects,
            activeProjectIndex = { activeProjectIndex },
            setActiveProjectIndex = { activeProjectIndex = it },
            normalizeProject = { project -> workflowActions.projectHygieneActions.normalizeProject(project) }
        )
    }

    private val conversationTaskRegistryActions: MainConversationTaskRegistryActions by lazy {
        MainConversationTaskRegistryActions(
            prefs = prefs,
            runningConversationTasks = runningConversationTasks,
            runningTraceToConversation = runningTraceToConversation,
            taskResponseTokens = taskResponseTokens,
            activeProject = projectStateActions::activeProject,
            activeConversation = projectStateActions::activeConversation,
            setWaitingForReply = { waitingForReply = it },
            setActiveRequestIsDevelopment = { activeRequestIsDevelopment = it },
            setPendingRequestPayload = { pendingRequestPayload = it },
            setPendingReconnectForActiveWork = { pendingReconnectForActiveWork = it },
            resetReconnectAttempts = { reconnectAttempts = 0 },
            getActiveRequestIsDevelopment = { activeRequestIsDevelopment },
            setSendEnabled = inputActions.sendEnabledActions::setSendEnabled,
            renderConversationList = homeListActions::renderConversationList,
            updateStage = projectViewActions::updateStage,
            updateProjectViews = projectViewActions::updateProjectViews
        )
    }

    private val conversationPreviewActions: MainConversationPreviewActions by lazy {
        MainConversationPreviewActions(
            binding = binding,
            projects = { projects },
            conversations = { projectStateActions.conversations },
            activeProject = projectStateActions::activeProject,
            activeConversation = projectStateActions::activeConversation,
            activeProjectIndex = { activeProjectIndex },
            activeConversationIndex = { projectStateActions.activeConversationIndex },
            chatAdapter = { chatAdapter },
            conversationTaskKey = conversationTaskRegistryActions::conversationTaskKey,
            workflowTerminalRoles = MainWorkflowRoles.terminal,
            closeStaleWorkflowMessages = { messages ->
                workflowActions.workflowMessageCompactor.closeStaleWorkflowMessages(messages)
            },
            hasRunningTasks = { runningConversationTasks.isNotEmpty() },
            saveConversations = projectStateActions::saveConversations,
            saveProjects = projectStateActions::saveProjects,
            renderConversationList = homeListActions::renderConversationList,
            renderProjectList = homeListActions::renderProjectList
        )
    }

    private val homeListActions: MainHomeListActions by lazy {
        MainHomeListActions(
            activity = this,
            binding = binding,
            projects = { projects },
            conversations = { projectStateActions.conversations },
            friends = { friends },
            activeProject = projectStateActions::activeProject,
            compactProjectTitle = { projectRecordActions.compactProjectTitle() },
            formatTime = { timeFormatter.format(Date(it)) },
            isTaskRunning = { projectId, conversationId ->
                val key = conversationTaskRegistryActions.conversationTaskKey(projectId, conversationId)
                runningConversationTasks.containsKey(key)
            },
            homeRows = { homeRows() },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            showCreateProjectDialog = { projectActions.showCreateProjectDialog() },
            showAddFriendDialog = { friendActions.showAddFriendDialog() },
            openAssistantConversation = {
                conversationOpenActions.openConversation(projectStateActions.activeConversationIndex)
            }
        )
    }

    private fun homeRows(): MainHomeRows {
        homeRows?.let { return it }
        return MainHomeRows(
            activity = this,
            timeFormatter = timeFormatter,
            activeProjectIndexProvider = { activeProjectIndex },
            openProject = conversationOpenActions::openProject,
            showProjectActions = { index -> projectActions.showProjectActions(index) },
            openConversation = conversationOpenActions::openConversation,
            showConversationActions = { index -> conversationActions.showConversationActions(index) },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground
        ).also { homeRows = it }
    }

    private val projectActions: MainProjectActions by lazy {
        MainProjectActions(
            activity = this,
            binding = binding,
            projects = projects,
            activeProjectIndexProvider = { activeProjectIndex },
            setActiveProjectIndex = { activeProjectIndex = it },
            setActiveConversationIndex = { projectStateActions.activeConversationIndex = it },
            titleEditText = { value -> mainTitleEditText(this, value, uiTools::dp) },
            saveProjects = projectStateActions::saveProjects,
            renderProjectList = homeListActions::renderProjectList,
            openProject = conversationOpenActions::openProject,
            showGitProjectDialog = ::showGitProjectDialog,
            http = http,
            serverUrl = serverUrl,
            tokenProvider = { AuthManager.token(this) },
            isLoggedIn = { AuthManager.isLoggedIn(this) }
        )
    }

    private val uiTools: MainUiTools by lazy { MainUiTools(this) }

    private fun accountActions(): MainAccountActions {
        return MainAccountActions(
            activity = this,
            binding = binding,
            projects = projects,
            gson = gson,
            prefs = prefs,
            saveProjects = projectStateActions::saveProjects,
            renderProjectList = homeListActions::renderProjectList
        )
    }

    private val profileQuickActions: MainProfileQuickActions by lazy {
        MainProfileQuickActions(
            activity = this,
            binding = binding,
            http = http,
            serverVersionUrl = serverVersionUrl,
            isBindingInitialized = { ::binding.isInitialized },
            refreshAccountUi = {
                if (::binding.isInitialized) accountActions().refreshAccountUi()
            },
            fillPlanPrompt = { quickCommandActions.fillPlanPrompt() },
            sendQuickCommand = { text -> quickCommandActions.sendQuickCommand(text) },
            showProjectRecordDialog = { projectRecordActions.showProjectRecordDialog() },
            showGitProjectDialog = ::showGitProjectDialog,
            openSettings = { quickCommandActions.openSettings() },
            showPromotionDialog = { messageActions.showPromotionDialog() },
            showGuestImportDialog = { accountActions().showGuestImportDialog() },
            confirmLogout = { accountActions().confirmLogout() }
        )
    }

    private val storeController: MainStoreController by lazy {
        MainStoreController(
            activity = this,
            http = http,
            serverUrl = serverUrl,
            tokenProvider = { AuthManager.token(this) },
            isLoggedIn = { AuthManager.isLoggedIn(this) },
            addJoinedProject = { storeProject ->
                val newProject = newAppProject(storeProject.name, storeProject.description ?: "商店项目")
                    .copy(id = storeProject.id)
                projects.add(newProject)
                projectStateActions.saveProjects()
                val idx = projects.lastIndex
                conversationOpenActions.openProject(idx)
                homeListActions.renderProjectList()
            },
            dp = uiTools::dp
        )
    }

    private val actionPopups: MainActionPopups by lazy {
        MainActionPopups(
            activity = this,
            binding = binding,
            getActionPopup = { actionPopup },
            setActionPopup = { actionPopup = it },
            shareActions = uiTools::shareActions,
            fillPlanPrompt = { quickCommandActions.fillPlanPrompt() },
            sendQuickCommand = { text -> quickCommandActions.sendQuickCommand(text) },
            showProjectRecordDialog = { projectRecordActions.showProjectRecordDialog() },
            showGitProjectDialog = ::showGitProjectDialog,
            showCreateProjectDialog = { projectActions.showCreateProjectDialog() },
            showCreateConversationDialog = { conversationActions.showCreateConversationDialog() },
            showAddFriendDialog = { friendActions.showAddFriendDialog() },
            openSettings = { quickCommandActions.openSettings() },
            deleteMessage = { message -> messageActions.deleteMessage(message) },
            quoteMessage = { text -> messageActions.quoteMessage(text) },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            showStoreDialog = { storeController.showStoreDialog() }
        )
    }

    private val friendActions: MainFriendActions by lazy {
        MainFriendActions(
            activity = this,
            http = http,
            serverUrl = serverUrl,
            dp = uiTools::dp,
            setFriends = { list ->
                friends.clear()
                friends.addAll(list)
            },
            onFriendsChanged = {
                if (::binding.isInitialized) homeListActions.renderConversationList()
            }
        )
    }

    private fun showGitProjectDialog() {
        MainProjectGitDialogs(
            activity = this,
            http = http,
            serverUrl = serverUrl,
            userId = userId,
            projectProvider = projectStateActions::activeProject,
            projectTitleProvider = { projectStateActions.currentProjectTitle },
            addProjectEvent = projectRecordActions::addProjectEvent,
            openUrl = { url -> externalActions.openUrl(url) },
            copyText = { label, text -> externalActions.copyText(label, text) }
        ).showGitProjectDialog()
    }

    private val codexPrewarm: MainCodexPrewarm by lazy {
        MainCodexPrewarm(
            http = http,
            serverUrl = serverUrl,
            userId = userId,
            activeProject = projectStateActions::activeProject,
            activeConversation = projectStateActions::activeConversation,
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking,
            selectedAgentForRequest = { modelActions.selectedAgentForRequest() }
        )
    }

    private val externalActions: MainExternalActions by lazy { MainExternalActions(this) }

    private val quickCommandActions: MainQuickCommandActions by lazy {
        MainQuickCommandActions(
            activity = this,
            binding = binding,
            activeConversation = projectStateActions::activeConversation,
            showCreateConversationDialog = { conversationActions.showCreateConversationDialog() },
            showChat = { navigationController.showChat() },
            sendMessage = { inputActions.sendMessageActions.sendMessage() }
        )
    }

    private val messageActions: MainMessageActions by lazy {
        MainMessageActions(
            activity = this,
            binding = binding,
            activeConversation = projectStateActions::activeConversation,
            chatAdapter = { chatAdapter },
            saveConversations = projectStateActions::saveConversations,
            renderConversationList = homeListActions::renderConversationList,
            showChat = { navigationController.showChat() },
            showMessageActionPopup = { anchor, message, text ->
                actionPopups.showMessageActionPopup(anchor, message, text)
            },
            shareActions = uiTools::shareActions,
            apkDownloadUrl = { apkDownloadUrl },
            apkDownloadPageUrl = { apkDownloadPageUrl }
        )
    }

    private fun stageHintShimmer(): MainStageHintShimmer {
        stageHintShimmer?.let { return it }
        return MainStageHintShimmer(
            binding = binding,
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking
        ).also { stageHintShimmer = it }
    }

    private val projectViewActions: MainProjectViewActions by lazy {
        MainProjectViewActions(
            activity = this,
            binding = binding,
            currentStage = { projectStateActions.currentStage },
            setCurrentStage = { projectStateActions.currentStage = it },
            setActiveProjectSubtitle = { projectStateActions.activeProject().subtitle = it },
            currentProjectTitle = { projectStateActions.currentProjectTitle },
            projectEvents = { projectStateActions.projectEvents },
            currentTimeText = { timeFormatter.format(Date()) },
            saveProjects = projectStateActions::saveProjects,
            renderConversationList = homeListActions::renderConversationList,
            renderProjectList = homeListActions::renderProjectList,
            updateStageHintShimmer = { stageHintShimmer().update() }
        )
    }

    private val projectRecordActions: MainProjectRecordActions by lazy {
        MainProjectRecordActions(
            activity = this,
            appName = { getString(R.string.app_name) },
            currentProjectTitle = { projectStateActions.currentProjectTitle },
            setCurrentProjectTitle = { projectStateActions.currentProjectTitle = it },
            activeProject = projectStateActions::activeProject,
            projectEvents = { projectStateActions.projectEvents },
            currentStage = { projectStateActions.currentStage },
            conversationCount = { projectStateActions.conversations.size },
            currentTimeText = { timeFormatter.format(Date()) },
            currentStageHint = { binding.stageHintText.text.toString() },
            saveProjects = projectStateActions::saveProjects,
            updateProjectViews = projectViewActions::updateProjectViews
        )
    }

    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        return false
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == R.id.action_settings) {
            quickCommandActions.openSettings()
            return true
        }
        return super.onOptionsItemSelected(item)
    }

    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (::binding.isInitialized && chatSideMenuController.handleDispatchTouchEvent(event)) {
            return true
        }
        return super.dispatchTouchEvent(event)
    }

    private fun cancelActiveChildTouch(event: MotionEvent) {
        val cancelEvent = MotionEvent.obtain(event)
        cancelEvent.action = MotionEvent.ACTION_CANCEL
        super.dispatchTouchEvent(cancelEvent)
        cancelEvent.recycle()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        lifecycleEdgeActions.onRequestPermissionsResult(requestCode, grantResults)
    }

    override fun onDestroy() {
        lifecycleEdgeActions.onDestroy()
        super.onDestroy()
    }

    private val lifecycleEdgeActions: MainLifecycleEdgeActions by lazy {
        MainLifecycleEdgeActions(
            activity = this,
            speechPermissionRequest = speechPermissionRequest,
            notificationPermissionRequest = notificationPermissionRequest,
            stopStageHintShimmer = { stageHintShimmer?.stop() },
            cancelHomeRowShimmer = {
                homeRows?.cancelHomeRowShimmer()
                homeRows = null
            },
            destroySpeechInput = inputActions::destroySpeechInput,
            isTaskWorkReceiverRegistered = { taskActions.taskWorkReceiverActions.isRegistered },
            unregisterTaskWorkReceiver = { taskActions.taskWorkReceiverActions.unregisterTaskWorkReceiver() }
        )
    }
}
