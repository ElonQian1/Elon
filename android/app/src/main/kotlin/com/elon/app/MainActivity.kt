package com.elon.app

import android.content.Intent
import android.os.Bundle
import android.view.Menu
import android.view.MenuItem
import android.view.MotionEvent
import android.widget.PopupWindow
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.update.AppUpdateManager
import java.util.Date

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var chatAdapter: ChatAdapter
    private lateinit var agentPageController: AgentPageController

    /** 注入 APK 操作回调后再赋值 chatAdapter，统一替代原来的 `setChatAdapter = { chatAdapter = it }`。 */
    private fun setAdapterAndWireApkActions(adapter: ChatAdapter) {
        adapter.onApkAction = { action, url -> handleApkChatAction(action, url) }
        adapter.onVoiceAttachmentLongPress = { message, attachment ->
            inputActions.showVoiceAttachmentActions(message, attachment)
        }
        chatAdapter = adapter
    }

    private fun handleApkChatAction(action: String, url: String) {
        when (action) {
            "install" -> ApkChatInstaller.downloadAndInstall(this, url, s.http)
            "copy" -> {
                val cm = getSystemService(android.content.Context.CLIPBOARD_SERVICE)
                    as android.content.ClipboardManager
                cm.setPrimaryClip(android.content.ClipData.newPlainText("apk_url", url))
                android.widget.Toast.makeText(this, "链接已复制", android.widget.Toast.LENGTH_SHORT).show()
            }
            "share" -> startActivity(
                android.content.Intent(android.content.Intent.ACTION_SEND).apply {
                    type = "text/plain"
                    putExtra(android.content.Intent.EXTRA_TEXT, url)
                }
            )
        }
    }
    /** 运行时可变状态与工具实例（OkHttpClient 含超时配置）。 */
    private val s = MainActivityState()
    private val prefs by lazy { AuthManager.userDataPrefs(this) }
    private val serverUrl get() = ServerUrlManager.getActive(this)
    /** 返回当前活跃服务器对应的 token：备用服务器使用本地静态 token，云端使用 session token。 */
    private fun activeToken(): String? {
        val activeUrl = ServerUrlManager.getActive(this)
        return if (activeUrl == BuildConfig.SERVER_URL) {
            AuthManager.token(this)
        } else {
            getSharedPreferences("agent_config", Context.MODE_PRIVATE)
                .getString("fallback_server_token", null)
                ?.takeIf { it.isNotBlank() }
                ?: AuthManager.token(this)
        }
    }
    private val apkDownloadUrl: String get() = "$serverUrl/app/ElonSpeed-latest.apk"
    private val apkDownloadPageUrl: String get() = "$serverUrl/app/download"
    private val serverVersionUrl: String get() = "$serverUrl/api/server/version"
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
            activeRequestIsDevelopment = { s.activeRequestIsDevelopment },
            setActiveRequestIsDevelopment = { s.activeRequestIsDevelopment = it },
            setWaitingForReply = { s.waitingForReply = it },
            clearPendingRequestPayload = { s.pendingRequestPayload = null },
            clearPendingReconnectForActiveWork = { s.pendingReconnectForActiveWork = false },
            resetReconnectAttempts = { s.reconnectAttempts = 0 },
            incrementServerResponseToken = { s.serverResponseToken += 1 },
            currentTimeText = { s.timeFormatter.format(Date()) },
            taskResponseTokens = s.taskResponseTokens,
            runningTraceToConversation = s.runningTraceToConversation,
            runningConversationTasks = s.runningConversationTasks,
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
            backendConnected = { s.backendConnected },
            setBackendConnected = { s.backendConnected = it },
            waitingForReply = { s.waitingForReply },
            resetReconnectAttempts = { s.reconnectAttempts = 0 },
            taskResponseTokens = s.taskResponseTokens,
            runningTraceToConversation = s.runningTraceToConversation,
            runningConversationTasks = s.runningConversationTasks,
            activeRequestIsDevelopment = { s.activeRequestIsDevelopment },
            workflowActions = { workflowActions },
            conversationPreviewActions = { conversationPreviewActions },
            conversationTaskRegistryActions = { conversationTaskRegistryActions },
            activeWorkControlActions = { activeWorkControlActions },
            sendEnabledActions = { inputActions.sendEnabledActions },
            isProjectConversationVisible = {
                !friendChatActions.isActive() &&
                    !groupChatActions.isActive() &&
                    !projectSpaceController.isChannelActive()
            },
            drainNextQueuedMessage = { projectId, conversationId ->
                inputActions.runningInputActions.drainNextQueuedMessage(projectId, conversationId)
            }
        )
    }

    private val inputActions: MainInputActions by lazy {
        MainInputActions(
            activity = this,
            binding = binding,
            http = s.http,
            serverUrl = serverUrl,
            speechPermissionRequest = speechPermissionRequest,
            userId = { userId },
            projects = s.projects,
            setActiveProjectIndex = { s.activeProjectIndex = it },
            setChatAdapter = ::setAdapterAndWireApkActions,
            uiTools = { uiTools },
            modelActions = { modelActions },
            projectStateActions = { projectStateActions },
            conversationTaskRegistryActions = { conversationTaskRegistryActions },
            workflowActions = { workflowActions },
            preparedMessageActions = { preparedMessageActions },
            activeWorkControlActions = { activeWorkControlActions },
            messageActions = { messageActions },
            navigationController = { navigationController },
            stageHintShimmer = { stageHintShimmer() },
            isFriendChatActive = {
                friendChatActions.isActive() || groupChatActions.isActive() || projectSpaceController.isChannelActive()
            },
            isSocialAiChatActive = {
                friendChatActions.isActive() || groupChatActions.isActive()
            },
            trySendFriendMessage = { text, attachments ->
                projectSpaceController.trySendMessage(text, attachments.isNotEmpty()) ||
                    groupChatActions.trySendMessage(text, attachments) ||
                    friendChatActions.trySendMessage(text, attachments)
            },
            forkForRunningInput = { text, outgoingText ->
                conversationForkActions.forkForRunningInput(text, outgoingText)
            },
            startTaskWorkService = taskActions.taskWorkServiceActions::startTaskWorkService
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)
        createActions.onCreate(intent)
        messageSelectionActions.setup()
        agentPageController = AgentPageController(this, binding)
        agentPageController.setup()
        com.elon.app.VoiceEngineBootstrap.scheduleSilentProbeIfNeeded(this)
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
            setChatAdapter = ::setAdapterAndWireApkActions,
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
            syncProjectsFromServer = { accountActions().syncProjectsFromServer() },
            getWaitingForReply = { s.waitingForReply },
            getBackendConnected = { s.backendConnected },
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking,
            startTaskWorkService = { action ->
                taskActions.taskWorkServiceActions.startTaskWorkService(action, isDevelopment = s.activeRequestIsDevelopment)
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
        val gws = (application as ElonApplication).globalWs
        gws.addListener(globalWsListener)
        gws.start(this)
        if (::binding.isInitialized) {
            profileQuickActions.refreshProfileSummary()
            if (::chatAdapter.isInitialized) chatAdapter.refreshUserProfile()
        }
        friendChatActions.resumeIfActive()
        groupChatActions.resumeIfActive()
        projectSpaceController.resumeIfActive()
        if (::agentPageController.isInitialized) agentPageController.refresh()
    }

    private val resumeActions: MainResumeActions by lazy {
        MainResumeActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            isBindingInitialized = { ::binding.isInitialized },
            setAppInForeground = { s.appInForeground = it },
            setTaskAppForeground = { foreground -> taskActions.taskWorkServiceActions.setTaskAppForeground(foreground) },
            drainQueuedTaskEvents = { taskActions.taskWorkServiceActions.drainQueuedTaskEvents() },
            loadModelOptions = { modelActions.loadModelOptions() },
            getBackendConnected = { s.backendConnected },
            getWaitingForReply = { s.waitingForReply },
            getPendingReconnectForActiveWork = { s.pendingReconnectForActiveWork },
            setPendingReconnectForActiveWork = { s.pendingReconnectForActiveWork = it },
            currentStage = { projectStateActions.currentStage },
            updateStage = projectViewActions::updateStage,
            recordEvidence = { kind, detail ->
                if (s.activeRequestIsDevelopment) workflowActions.evidenceActions.recordEvidence(kind, detail)
            },
            startTaskWorkService = { action ->
                taskActions.taskWorkServiceActions.startTaskWorkService(action, isDevelopment = s.activeRequestIsDevelopment)
            },
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking,
            setSendEnabled = { enabled -> inputActions.sendEnabledActions.setSendEnabled(enabled) },
            maybePrewarmCodexSession = codexPrewarm::maybePrewarmCodexSession
        )
    }

    override fun onPause() {
        s.appInForeground = false
        friendChatActions.stopPolling()
        groupChatActions.stopPolling()
        projectSpaceController.stopPolling()
        taskActions.taskWorkServiceActions.setTaskAppForeground(false)
        super.onPause()
    }

    override fun onStop() {
        s.appInForeground = false
        taskActions.taskWorkServiceActions.setTaskAppForeground(false)
        projectStateActions.saveProjects()
        val gws = (application as ElonApplication).globalWs
        gws.removeListener(globalWsListener)
        super.onStop()
    }

    private val globalWsListener = object : GlobalWsManager.Listener {
        override fun onGlobalWsEvent(event: GlobalWsEvent) {
            when (event) {
                is GlobalWsEvent.AppUpdateAvailable ->
                    AppUpdateManager(this@MainActivity).realtimeCheck(event.versionCode)
                is GlobalWsEvent.FriendMessage -> {
                    friendChatActions.handleRealtimeMessage(event.fromUserId)
                    friendActions.loadFriends()
                }
                is GlobalWsEvent.GroupMessage -> {
                    groupChatActions.handleRealtimeMessage(event.groupId)
                    groupActions.loadGroups()
                }
                else -> Unit
            }
        }
    }

    private val preparedMessageActions: MainPreparedMessageActions by lazy {
        MainPreparedMessageActions(
            activity = this,
            binding = binding,
            restoreSendTarget = { target -> inputActions.sendTargetRestoreActions.restoreSendTarget(target) },
            isConversationTaskRunning = { target ->
                val key = conversationTaskRegistryActions.conversationTaskKey(target.projectId, target.conversationId)
                s.runningConversationTasks.containsKey(key)
            },
            setSendEnabled = { enabled -> inputActions.sendEnabledActions.setSendEnabled(enabled) },
            userId = { userId },
            selectedAgentForRequest = { modelActions.selectedAgentForRequest() },
            appendMessage = workflowActions.messageAppendActions::appendMessage,
            collapseInputComposer = { inputActions.inputFocusActions.collapseInputComposer() },
            looksLikeDevelopmentRequest = ::looksLikeDevelopmentRequest,
            looksLikeDirectImageRequest = ::looksLikeDirectImageRequest,
            rememberConversationTask = conversationTaskRegistryActions::rememberConversationTask,
            setActiveRequestIsDevelopment = { s.activeRequestIsDevelopment = it },
            resetRequestState = {
                s.pendingReconnectForActiveWork = false
                s.reconnectAttempts = 0
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
            nextServerResponseToken = { ++s.serverResponseToken },
            putTaskResponseToken = { traceId, token -> s.taskResponseTokens[traceId] = token },
            startTaskWorkService = taskActions.taskWorkServiceActions::startTaskWorkService,
            ensureBackgroundKeepAlive = { isDevelopment ->
                TaskBackgroundKeepAlive.maybePromptForDevelopmentTask(this, prefs, isDevelopment)
            },
            markTaskPendingReconnect = { target ->
                val key = conversationTaskRegistryActions.conversationTaskKey(target.projectId, target.conversationId)
                s.runningConversationTasks[key]?.pendingReconnect = true
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
            resetReconnectAttempts = { s.reconnectAttempts = 0 },
            incrementReconnectAttempts = {
                s.reconnectAttempts += 1
                s.reconnectAttempts
            },
            taskForTrace = { traceId ->
                s.runningTraceToConversation[traceId]?.let { s.runningConversationTasks[it] }
            },
            isBackendConnected = { s.backendConnected },
            getActiveRequestIsDevelopment = { s.activeRequestIsDevelopment },
            setActiveRequestIsDevelopment = { s.activeRequestIsDevelopment = it },
            getCurrentStage = { projectStateActions.currentStage },
            getPendingRequestPayload = { s.pendingRequestPayload },
            setPendingReconnectForActiveWork = { s.pendingReconnectForActiveWork = it },
            setWaitingForReply = { s.waitingForReply = it },
            persistActiveWork = conversationTaskRegistryActions::persistActiveWork,
            clearPersistedActiveWork = conversationTaskRegistryActions::clearPersistedActiveWork,
            refreshActiveTaskState = conversationTaskRegistryActions::refreshActiveTaskState,
            stopWorkingEvidenceForActiveConversation = {
                workflowActions.evidenceActions.stopWorkingEvidenceForActiveConversation()
            },
            clearCurrentEvidence = { workflowActions.evidenceActions.clearCurrentEvidence() },
            setSendEnabled = { enabled -> inputActions.sendEnabledActions.setSendEnabled(enabled) },
            updateFirstConversationStatus = { text ->
                conversationPreviewActions.updateFirstConversationStatus(text)
            },
            updateStage = projectViewActions::updateStage,
            updateProjectViews = projectViewActions::updateProjectViews,
            addProjectEvent = projectRecordActions::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (s.activeRequestIsDevelopment) workflowActions.evidenceActions.recordEvidence(kind, detail)
            },
            appendMessage = workflowActions.messageAppendActions::appendMessage,
            workflowStoppedMessage = ::mainWorkflowStoppedMessage,
            startTaskWorkService = taskActions.taskWorkServiceActions::startTaskWorkService,
            nextServerResponseToken = { ++s.serverResponseToken },
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
            renderProjectSpace = { projectSpaceController.renderActiveSpace() },
            refreshServerVersion = { profileQuickActions.refreshServerVersion() },
            openConversation = conversationOpenActions::openConversation,
            showConversationActions = { index -> conversationActions.showConversationActions(index) },
            showHomeActionPopup = { anchor, tab -> actionPopups.showHomeActionPopup(anchor, tab) },
            showChatActionPopup = { anchor -> actionPopups.showChatActionPopup(anchor) },
            showContactChatSettings = { showActiveContactChatSettings() },
            showAddFriendDialog = { friendActions.showAddFriendDialog() },
            refreshFriends = {
                friendActions.loadFriends()
                groupActions.loadGroups()
            },
            updateFirstConversationStatus = { text ->
                conversationPreviewActions.updateFirstConversationStatus(text)
            },
            collapseInputComposer = { animate -> inputActions.inputFocusActions.collapseInputComposer(animate) },
            collapseInputComposerForBack = { inputActions.hideInputOverlaysForBack() },
            isChatSideMenuOpen = { chatSideMenuController.isOpen },
            closeChatSideMenu = { animate -> chatSideMenuController.close(animate) },
            isActiveConversationWorking = conversationTaskRegistryActions::isActiveConversationWorking,
            isMessageSelectionActive = { messageSelectionActions.isSelectionActive() },
            clearMessageSelection = { messageSelectionActions.cancelSelection() },
            setSendEnabled = { enabled -> inputActions.sendEnabledActions.setSendEnabled(enabled) },
            maybePrewarmCodexSession = codexPrewarm::maybePrewarmCodexSession,
            onFriendChatClosed = {
                friendChatActions.closeFriendChat()
                groupChatActions.closeGroupChat()
                projectSpaceController.closeChannelChat()
            },
            onProjectChannelClosed = { projectSpaceController.closeChannelChat() },
            showProjectMembers = { projectSpaceController.showMembers() },
            loadMarketplace = { marketplaceActions.loadProjects() },
            onAgentTabSelected = { agentPageController.refresh() }
        )
    }

    private val chatSettingsActions: MainChatSettingsActions by lazy {
        MainChatSettingsActions(
            activity = this,
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            clearFriendMessages = { friendChatActions.clearCurrentMessages() },
            clearGroupMessages = { groupChatActions.clearCurrentMessages() },
            onAddGroupMember = { group, onDone -> groupActions.showAddMemberDialog(group, onDone) }
        )
    }

    private fun showActiveContactChatSettings() {
        groupChatActions.currentGroup()?.let {
            chatSettingsActions.showGroupSettings(it)
            return
        }
        friendChatActions.currentFriend()?.let {
            chatSettingsActions.showFriendSettings(it)
        }
    }

    private val marketplaceActions: MainMarketplaceActions by lazy {
        MainMarketplaceActions(
            activity = this,
            http = s.http,
            serverUrl = serverUrl,
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            getListContainer = { binding.marketplaceListContainer },
            openJoinedProject = { storeProject ->
                val newProject = storeProject.toJointAppProject()
                if (s.projects.none { it.id == storeProject.id }) {
                    s.projects.add(newProject)
                    projectStateActions.saveProjects()
                }
                val idx = s.projects.indexOfFirst { it.id == storeProject.id }.takeIf { it >= 0 }
                    ?: s.projects.lastIndex
                s.activeProjectIndex = idx
                projectStateActions.saveProjects()
                homeListActions.renderProjectList()
                projectSpaceController.openProjectSpace(storeProject.id, storeProject.name, true)
            }
        )
    }

    private val projectSpaceController: ProjectSpaceController by lazy {
        ProjectSpaceController(
            activity = this,
            binding = binding,
            http = s.http,
            serverUrl = serverUrl,
            setChatAdapter = ::setAdapterAndWireApkActions,
            showProjectSpace = { title, animate -> navigationController.showProjectSpace(title, animate) },
            showProjectChannelChat = { title, animate -> navigationController.showProjectChannelChat(title, animate) },
            showMessageActions = { anchor, message -> messageActions.showMessageActions(anchor, message) },
            onProjectShareAction = chatProjectShareActions::handleCardAction,
            collapseInputComposer = { inputActions.inputFocusActions.collapseInputComposer() },
            personalConversations = { projectStateActions.conversations },
            activePersonalConversationIndex = { projectStateActions.activeConversationIndex },
            openPersonalAiChat = { conversationIndex ->
                val idx = s.projects.indexOfFirst { it.id == projectStateActions.activeProject().id }
                    .takeIf { it >= 0 } ?: s.activeProjectIndex
                conversationOpenActions.openProjectSpaceConversation(idx, conversationIndex)
            },
            showPersonalConversationActions = { index -> conversationActions.showConversationActions(index) },
            showCreatePersonalConversation = { conversationActions.showCreateConversationDialog() },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground
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
            copyConversationIdentity = conversationIdentityActions::copyConversationIdentity,
            isConversationWorking = homeListActions::isConversationWorking,
            showProjectShareSideMenu = { friendChatActions.isActive() || groupChatActions.isActive() },
            projects = { s.projects },
            activeProjectIndex = { s.activeProjectIndex },
            openPersonalProject = { index ->
                conversationOpenActions.openProject(index)
            },
            openJointProject = { index ->
                if (index in s.projects.indices) {
                    s.activeProjectIndex = index
                    projectStateActions.saveProjects()
                    val project = s.projects[index]
                    projectSpaceController.openProjectSpace(project.projectSpaceId(), project.title, true)
                }
            },
            openProjectManagement = { navigationController.showProjectManagement(animate = true) },
            showCreateJointProjectDialog = { projectActions.showCreateJointProjectDialog() },
            sendProjectShare = chatProjectShareActions::sendToCurrentChat,
            showCreateConversationDialog = { conversationActions.showCreateConversationDialog() },
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
            setSendEnabled = { enabled -> inputActions.sendEnabledActions.setSendEnabled(enabled) },
            onConversationsChanged = { projectSpaceController.renderActiveSpace() }
        )
    }

    private val conversationForkActions: MainConversationForkActions by lazy {
        MainConversationForkActions(
            binding = binding,
            activeProject = projectStateActions::activeProject,
            activeConversation = projectStateActions::activeConversation,
            activeConversationTask = conversationTaskRegistryActions::activeConversationTask,
            setActiveConversationIndex = { projectStateActions.activeConversationIndex = it },
            saveProjects = projectStateActions::saveProjects,
            renderConversationList = homeListActions::renderConversationList,
            openConversation = conversationOpenActions::openConversation,
            renderProjectSpace = { projectSpaceController.renderActiveSpace() }
        )
    }

    private val conversationIdentityActions: MainConversationIdentityActions by lazy {
        MainConversationIdentityActions(
            activity = this,
            http = s.http,
            serverUrl = serverUrl,
            userId = userId,
            activeProject = projectStateActions::activeProject,
            saveProjects = projectStateActions::saveProjects,
            copyText = externalActions::copyText
        )
    }

    private val modelActions: MainModelActions by lazy {
        MainModelActions(
            activity = this,
            binding = binding,
            prefs = prefs,
            http = s.http,
            serverUrl = serverUrl,
            userIdProvider = { userId },
            modelButtonShellProvider = { inputActions.inputComposerViewsOrNull()?.modelButtonShell },
            modelChevronProvider = { inputActions.inputComposerViewsOrNull()?.modelChevron },
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
            projects = { s.projects },
            conversations = { projectStateActions.conversations },
            activeConversation = projectStateActions::activeConversation,
            setActiveProjectIndex = { s.activeProjectIndex = it },
            setActiveConversationIndex = { projectStateActions.activeConversationIndex = it },
            setChatAdapter = ::setAdapterAndWireApkActions,
            pauseCurrentWork = { activeWorkControlActions.pauseCurrentWork() },
            showMessageActions = { anchor, message -> messageActions.showMessageActions(anchor, message) },
            retryFailedAttachmentMessage = { message -> inputActions.retryFailedAttachmentMessage(message) },
            showChat = { animate -> navigationController.showChat(animate = animate) },
            showProjectChat = { animate -> navigationController.showProjectChat(animate = animate) },
            showProjectPersonalChat = { title, animate -> navigationController.showProjectPersonalChat(title, animate) },
            saveProjects = projectStateActions::saveProjects
        )
    }

    private val projectStateActions: MainProjectStateActions by lazy {
        MainProjectStateActions(
            prefs = prefs,
            gson = s.gson,
            projects = s.projects,
            activeProjectIndex = { s.activeProjectIndex },
            setActiveProjectIndex = { s.activeProjectIndex = it },
            normalizeProject = { project -> workflowActions.projectHygieneActions.normalizeProject(project) }
        )
    }

    private val conversationTaskRegistryActions: MainConversationTaskRegistryActions by lazy {
        MainConversationTaskRegistryActions(
            prefs = prefs,
            runningConversationTasks = s.runningConversationTasks,
            runningTraceToConversation = s.runningTraceToConversation,
            taskResponseTokens = s.taskResponseTokens,
            activeProject = projectStateActions::activeProject,
            activeConversation = projectStateActions::activeConversation,
            setWaitingForReply = { s.waitingForReply = it },
            setActiveRequestIsDevelopment = { s.activeRequestIsDevelopment = it },
            setPendingRequestPayload = { s.pendingRequestPayload = it },
            setPendingReconnectForActiveWork = { s.pendingReconnectForActiveWork = it },
            resetReconnectAttempts = { s.reconnectAttempts = 0 },
            getActiveRequestIsDevelopment = { s.activeRequestIsDevelopment },
            setSendEnabled = { enabled -> inputActions.sendEnabledActions.setSendEnabled(enabled) },
            renderConversationList = homeListActions::renderConversationList,
            updateStage = projectViewActions::updateStage,
            updateProjectViews = projectViewActions::updateProjectViews
        )
    }

    private val conversationPreviewActions: MainConversationPreviewActions by lazy {
        MainConversationPreviewActions(
            binding = binding,
            projects = { s.projects },
            conversations = { projectStateActions.conversations },
            activeProject = projectStateActions::activeProject,
            activeConversation = projectStateActions::activeConversation,
            activeProjectIndex = { s.activeProjectIndex },
            activeConversationIndex = { projectStateActions.activeConversationIndex },
            chatAdapter = { chatAdapter },
            conversationTaskKey = conversationTaskRegistryActions::conversationTaskKey,
            workflowTerminalRoles = MainWorkflowRoles.terminal,
            closeStaleWorkflowMessages = { messages ->
                workflowActions.workflowMessageCompactor.closeStaleWorkflowMessages(messages)
            },
            hasRunningTasks = { s.runningConversationTasks.isNotEmpty() },
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
            projects = { s.projects },
            conversations = { projectStateActions.conversations },
            friends = { s.friends },
            groups = { s.groups },
            activeProject = projectStateActions::activeProject,
            compactProjectTitle = { projectRecordActions.compactProjectTitle() },
            formatTime = { s.timeFormatter.format(Date(it)) },
            isTaskRunning = { projectId, conversationId ->
                val key = conversationTaskRegistryActions.conversationTaskKey(projectId, conversationId)
                s.runningConversationTasks.containsKey(key)
            },
            homeRows = { homeRows() },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            showCreateProjectDialog = { projectActions.showCreateProjectDialog() },
            showProjectPlaza = { navigationController.showProjectPlaza() },
            showAddFriendDialog = { friendActions.showAddFriendDialog() },
            openFriend = { friend ->
                groupChatActions.closeGroupChat()
                projectSpaceController.closeChannelChat()
                friendChatActions.openFriend(friend, animate = true)
            },
            openGroup = { group ->
                friendChatActions.closeFriendChat()
                projectSpaceController.closeChannelChat()
                groupChatActions.openGroup(group, animate = true)
            }
        )
    }

    private val friendChatActions: MainFriendChatActions by lazy {
        MainFriendChatActions(
            activity = this,
            binding = binding,
            http = s.http,
            serverUrl = serverUrl,
            setChatAdapter = ::setAdapterAndWireApkActions,
            showFriendChat = { title, animate -> navigationController.showFriendChat(title, animate) },
            showMessageActions = { anchor, message -> messageActions.showMessageActions(anchor, message) },
            onProjectShareAction = chatProjectShareActions::handleCardAction,
            onProjectShareLongPress = { anchor, message, share ->
                actionPopups.showProjectShareActionPopup(anchor, message, share)
            },
            userId = { AuthManager.effectiveUserId(this) },
            clearPendingAttachments = { inputActions.pendingAttachmentActions.clearPendingAttachments(deleteFiles = false) },
            collapseInputComposer = { inputActions.inputFocusActions.collapseInputComposer() },
            onFriendSummariesChanged = { friendActions.loadFriends() }
        )
    }

    private val groupChatActions: MainGroupChatActions by lazy {
        MainGroupChatActions(
            activity = this,
            binding = binding,
            http = s.http,
            serverUrl = serverUrl,
            setChatAdapter = ::setAdapterAndWireApkActions,
            showFriendChat = { title, animate -> navigationController.showFriendChat(title, animate) },
            showMessageActions = { anchor, message -> messageActions.showMessageActions(anchor, message) },
            onProjectShareAction = chatProjectShareActions::handleCardAction,
            onProjectShareLongPress = { anchor, message, share ->
                actionPopups.showProjectShareActionPopup(anchor, message, share)
            },
            userId = { AuthManager.effectiveUserId(this) },
            clearPendingAttachments = { inputActions.pendingAttachmentActions.clearPendingAttachments(deleteFiles = false) },
            collapseInputComposer = { inputActions.inputFocusActions.collapseInputComposer() },
            onGroupSummariesChanged = { groupActions.loadGroups() }
        )
    }

    private fun homeRows(): MainHomeRows {
        homeRows?.let { return it }
        return MainHomeRows(
            activity = this,
            timeFormatter = s.timeFormatter,
            activeProjectIndexProvider = { s.activeProjectIndex },
            openProject = { index ->
                if (index in s.projects.indices && s.projects[index].isJointDevelopmentProject()) {
                    val project = s.projects[index]
                    s.activeProjectIndex = index
                    projectStateActions.saveProjects()
                    projectSpaceController.openProjectSpace(project.projectSpaceId(), project.title, true)
                } else {
                    conversationOpenActions.openProject(index)
                }
            },
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
            projects = s.projects,
            activeProjectIndexProvider = { s.activeProjectIndex },
            setActiveProjectIndex = { s.activeProjectIndex = it },
            setActiveConversationIndex = { projectStateActions.activeConversationIndex = it },
            titleEditText = { value -> mainTitleEditText(this, value, uiTools::dp) },
            saveProjects = projectStateActions::saveProjects,
            renderProjectList = homeListActions::renderProjectList,
            openProject = conversationOpenActions::openProject,
            openProjectSpace = { id, title -> projectSpaceController.openProjectSpace(id, title, true) },
            showGitProjectDialog = ::showGitProjectDialog,
            http = s.http,
            serverUrl = serverUrl,
            tokenProvider = { activeToken() },
            isLoggedIn = { AuthManager.isLoggedIn(this) },
            removeSentProjectShareCards = { projectIds ->
                friendChatActions.removeProjectShareCards(projectIds) +
                    groupChatActions.removeProjectShareCards(projectIds)
            }
        )
    }

    private val uiTools: MainUiTools by lazy { MainUiTools(this) }

    private fun accountActions(): MainAccountActions {
        return MainAccountActions(
            activity = this,
            binding = binding,
            projects = s.projects,
            gson = s.gson,
            prefs = prefs,
            http = s.http,
            serverUrl = serverUrl,
            saveProjects = projectStateActions::saveProjects,
            renderProjectList = homeListActions::renderProjectList,
            refreshProfileSummary = {
                if (::binding.isInitialized) profileQuickActions.refreshProfileSummary()
            }
        )
    }

    private val profileQuickActions: MainProfileQuickActions by lazy {
        MainProfileQuickActions(
            activity = this,
            binding = binding,
            http = s.http,
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
            openProfileDetails = {
                startActivity(Intent(this, PersonalProfileActivity::class.java))
            },
            openAgentCenter = { navigationController.showAgentCenter() },
            showPromotionDialog = { messageActions.showPromotionDialog() },
            showGuestImportDialog = { accountActions().showGuestImportDialog() },
            confirmLogout = { accountActions().confirmLogout() }
        )
    }

    private val storeController: MainStoreController by lazy {
        MainStoreController(
            activity = this,
            http = s.http,
            serverUrl = serverUrl,
            tokenProvider = { activeToken() },
            isLoggedIn = { AuthManager.isLoggedIn(this) },
            addJoinedProject = { storeProject ->
                val newProject = storeProject.toJointAppProject()
                if (s.projects.none { it.id == storeProject.id }) s.projects.add(newProject)
                projectStateActions.saveProjects()
                val idx = s.projects.indexOfFirst { it.id == storeProject.id }.takeIf { it >= 0 }
                    ?: s.projects.lastIndex
                s.activeProjectIndex = idx
                projectStateActions.saveProjects()
                homeListActions.renderProjectList()
                projectSpaceController.openProjectSpace(storeProject.id, storeProject.name, true)
            },
            dp = uiTools::dp
        )
    }

    private val chatProjectShareActions: MainChatProjectShareActions by lazy {
        MainChatProjectShareActions(
            activity = this,
            binding = binding,
            http = s.http,
            serverUrl = serverUrl,
            projects = s.projects,
            setActiveProjectIndex = { s.activeProjectIndex = it },
            saveProjects = projectStateActions::saveProjects,
            renderProjectList = homeListActions::renderProjectList,
            openLocalProject = conversationOpenActions::openProject,
            openProjectSpace = { id, title -> projectSpaceController.openProjectSpace(id, title, true) },
            deleteActiveChatMessage = { message, onDeleted ->
                when {
                    friendChatActions.isActive() -> friendChatActions.deleteCurrentMessage(message, onDeleted)
                    groupChatActions.isActive() -> groupChatActions.deleteCurrentMessage(message, onDeleted)
                }
            },
            sendMessage = { inputActions.sendMessageActions.sendMessage() },
            isLoggedIn = { AuthManager.isLoggedIn(this) },
            tokenProvider = { activeToken() }
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
            showCreateGroupDialog = { groupActions.showCreateGroupDialog() },
            showAddFriendDialog = { friendActions.showAddFriendDialog() },
            openSettings = { quickCommandActions.openSettings() },
            deleteMessage = { message -> messageActions.deleteMessage(message) },
            startMultiSelect = { message -> messageSelectionActions.startSelection(message) },
            revokeProjectShare = { message, share -> chatProjectShareActions.revokePublishedShare(message, share) },
            quoteMessage = { text -> messageActions.quoteMessage(text) },
            canRequestAiReply = {
                friendChatActions.isActive() || groupChatActions.isActive()
            },
            requestAiReply = { message ->
                when {
                    friendChatActions.isActive() -> friendChatActions.requestAiReply(message)
                    groupChatActions.isActive() -> groupChatActions.requestAiReply(message)
                    else -> uiTools.shareActions().toastMessageAction("当前聊天暂不支持 AI回复")
                }
            },
            dp = uiTools::dp,
            selectableForeground = uiTools::selectableForeground,
            showStoreDialog = { storeController.showStoreDialog() }
        )
    }

    private val friendActions: MainFriendActions by lazy {
        MainFriendActions(
            activity = this,
            http = s.http,
            serverUrl = serverUrl,
            dp = uiTools::dp,
            setFriends = { list ->
                s.friends.clear()
                s.friends.addAll(list)
            },
            onFriendsChanged = {
                if (::binding.isInitialized) homeListActions.renderConversationList()
            }
        )
    }

    private val groupActions: MainGroupActions by lazy {
        MainGroupActions(
            activity = this,
            http = s.http,
            serverUrl = serverUrl,
            friends = { s.friends },
            dp = uiTools::dp,
            setGroups = { list ->
                s.groups.clear()
                s.groups.addAll(list)
            },
            onGroupsChanged = {
                if (::binding.isInitialized) homeListActions.renderConversationList()
            },
            openGroup = { group ->
                friendChatActions.closeFriendChat()
                projectSpaceController.closeChannelChat()
                groupChatActions.openGroup(group, animate = true)
            }
        )
    }

    private fun showGitProjectDialog() {
        MainProjectGitDialogs(
            activity = this,
            http = s.http,
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
            http = s.http,
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

    private val messageSelectionActions: MainChatSelectionActions by lazy {
        MainChatSelectionActions(
            activity = this,
            binding = binding,
            chatAdapter = { chatAdapter },
            activeConversation = projectStateActions::activeConversation,
            saveConversations = projectStateActions::saveConversations,
            renderConversationList = homeListActions::renderConversationList,
            shareActions = uiTools::shareActions,
            isProjectChannelActive = projectSpaceController::isChannelActive,
            summarizeInCurrentChannel = projectSpaceController::summarizeSelectedDiscussion,
            summarizeInPersonalChat = { prompt -> sendSelectedDiscussionToAi(prompt) },
            summarizeInNewPersonalChat = { prompt -> sendSelectedDiscussionToNewAiChat(prompt) }
        )
    }

    private fun sendSelectedDiscussionToNewAiChat(prompt: String) {
        val project = projectStateActions.activeProject()
        project.conversations.add(newAppConversation("多选讨论总结", "AI 总结多选聊天记录"))
        project.activeConversationIndex = project.conversations.lastIndex
        project.updatedAt = System.currentTimeMillis()
        projectStateActions.saveConversations()
        homeListActions.renderConversationList()
        sendSelectedDiscussionToAi(prompt)
    }

    private fun sendSelectedDiscussionToAi(prompt: String) {
        if (projectStateActions.activeConversation().ended) {
            conversationActions.showCreateConversationDialog()
            return
        }
        friendChatActions.closeFriendChat()
        groupChatActions.closeGroupChat()
        projectSpaceController.closeChannelChat()
        navigationController.showProjectChat()
        binding.inputEdit.setText(prompt)
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
        inputActions.sendMessageActions.sendMessage()
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
            currentTimeText = { s.timeFormatter.format(Date()) },
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
            currentTimeText = { s.timeFormatter.format(Date()) },
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
        friendChatActions.stopPolling()
        groupChatActions.stopPolling()
        projectSpaceController.stopPolling()
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
