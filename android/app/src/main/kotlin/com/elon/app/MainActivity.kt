package com.elon.app

import android.Manifest
import android.annotation.SuppressLint
import android.animation.ValueAnimator
import android.content.BroadcastReceiver
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Shader
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.text.Editable
import android.text.InputType
import android.text.TextUtils
import android.text.TextWatcher
import android.util.TypedValue
import android.view.Gravity
import android.view.Menu
import android.view.MenuItem
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import com.elon.app.BuildConfig
import com.elon.app.update.AppUpdateManager
import com.elon.app.update.UpdateCheckWorker
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID
import kotlin.math.sin

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var chatAdapter: ChatAdapter
    private var waitingForReply = false
    private var activeRequestIsDevelopment = false
    private var workflowStepIndex = 0
    private var foldedCliLogCount = 0
    private val foldedCliLogSamples = ArrayDeque<String>()
    private val foldedCliLogCategories = linkedMapOf<String, Int>()
    private val currentEvidenceEntries = mutableListOf<EvidenceEntry>()
    // tool 名 -> 已派发但尚未 tool_result 的 ai-action 气泡在当前会话中的 index 队列
    private val pendingToolActionBubbles = linkedMapOf<String, ArrayDeque<Int>>()
    private val emittedProgressSignals = linkedSetOf<String>()
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
    private val prewarmCooldownMs = 120_000L
    private val prewarmLock = Any()
    private val prewarmingConversationKeys = mutableSetOf<String>()
    private val lastPrewarmAt = mutableMapOf<String, Long>()
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
    private var modelActions: MainModelActions? = null
    private var stageHintAnimator: ValueAnimator? = null
    private var stageHintShimmerToken = 0
    private var conversationHomeRowAnimator: ValueAnimator? = null
    private var exitConfirmDialog: AlertDialog? = null
    private var actionPopup: PopupWindow? = null
    private var pageTransitionRunning = false
    private lateinit var inputModeButton: ImageButton
    private lateinit var attachmentButton: ImageButton
    private lateinit var voiceHoldButton: TextView
    private lateinit var inputBarContainer: LinearLayout
    private lateinit var inputCenterContainer: FrameLayout
    private lateinit var expandedInputContainer: FrameLayout
    private lateinit var collapsedInputPreview: TextView
    private lateinit var modelButtonShell: FrameLayout
    private lateinit var inputRightControls: FrameLayout
    private lateinit var inputComposerMotion: InputComposerMotion
    private lateinit var attachmentPanel: LinearLayout
    private var attachmentPanelOpen = false
    private var attachmentIconAnimationToken = 0
    private var voiceMode = false
    private var inputCanSend = true
    private var suppressInputFocusAnimation = false
    private var speechRecognizer: SpeechRecognizer? = null
    private var isListeningForSpeech = false
    private val speechPermissionRequest = 4301
    private val notificationPermissionRequest = 4302
    private lateinit var cameraAttachmentLauncher: ActivityResultLauncher<Uri>
    private lateinit var photoAttachmentLauncher: ActivityResultLauncher<PickVisualMediaRequest>
    private lateinit var documentAttachmentLauncher: ActivityResultLauncher<Array<String>>
    private var pendingCameraUri: Uri? = null
    private var pendingCameraName: String? = null
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
        setupTaskCompletionAlerts(this, prefs, notificationPermissionRequest)

        loadProjects()

        setupAttachmentLaunchers()
        chatAdapter = ChatAdapter(activeConversation().messages, ::pauseCurrentWork, ::showMessageActions)
        binding.chatList.adapter = chatAdapter
        setupNavigation()
        setupQuickActions()
        setupBackHandling()
        setupInputComposer()
        restoreCachedModelSelection()
        updateProjectViews("像聊天一样发需求，我会同步整理开发进度和项目记录。")
        setTaskAppForeground(true)
        registerTaskWorkReceiver()
        restorePendingActiveWork()
        checkAndOfferGuestImport()
        startTaskWorkService(
            if (waitingForReply) TaskWorkService.ACTION_RESUME_PENDING else TaskWorkService.ACTION_CONNECT
        )

        // 重连按钮
        binding.statusText.setOnClickListener {
            if (backendConnected || !isActiveConversationWorking()) {
                openConversation(0)
            } else {
                startTaskWorkService(TaskWorkService.ACTION_CONNECT)
            }
        }

        loadModelOptions()

        // 键盘回车发送
        binding.inputEdit.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_SEND) {
                sendMessage()
                true
            } else false
        }

        // 自动检查更新（12 小时冷却，静默失败不打扰用户）
        AppUpdateManager(this).autoCheck()
        // 注册后台周期检查（APP 关闭时也能推送通知）
        UpdateCheckWorker.schedule(this)
        // 注册本机为同WiFi APK 种子节点（已安装用户帮助其他用户加速下载）
        com.elon.app.update.PeerSeederManager.start(this)
        handleLaunchIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleLaunchIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        appInForeground = true
        setTaskAppForeground(true)
        startMcpDebugKeepAlive()
        drainQueuedTaskEvents()
        clearCompletedTaskBadge(this, prefs)
        if (::binding.isInitialized) {
            loadModelOptions()
            if (!backendConnected) {
                if (waitingForReply && !pendingReconnectForActiveWork) {
                    pendingReconnectForActiveWork = true
                    updateStage(currentStage, "正在恢复连接，回来后会自动继续本轮任务。")
                    recordEvidence("connection", "连接恢复中，正在继续上次任务")
                }
                startTaskWorkService(
                    if (waitingForReply) TaskWorkService.ACTION_RESUME_PENDING else TaskWorkService.ACTION_CONNECT
                )
            } else if (!isActiveConversationWorking()) {
                setSendEnabled(true)
                if (binding.chatPage.visibility == View.VISIBLE) {
                    maybePrewarmCodexSession("resume_chat")
                }
            }
        }
    }

    private fun startMcpDebugKeepAlive() {
        if (!McpDebugKeepAliveService.shouldAutoStart(this)) return
        val intent = Intent(this, McpDebugKeepAliveService::class.java).apply {
            action = McpDebugKeepAliveService.ACTION_START
        }
        runCatching {
            ContextCompat.startForegroundService(this, intent)
        }.onSuccess {
            DebugTraceStore.record("mcp_keepalive_auto_start_requested")
        }.onFailure { error ->
            DebugTraceStore.record(
                "mcp_keepalive_auto_start_failed",
                mapOf("error" to (error.message ?: error.javaClass.simpleName))
            )
        }
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
        collapseAttachmentPanel()
        val rawText = binding.inputEdit.text.toString().trim()
        if (rawText.isEmpty() && pendingAttachments.isEmpty()) return
        if (isActiveConversationWorking()) return
        if (activeConversation().ended) {
            appendMessage(ChatMessage("error", "这个会话已结束，请新建会话继续。"))
            return
        }
        val text = if (pendingAttachments.isNotEmpty()) {
            visibleTextForPendingAttachments(rawText, pendingAttachments)
        } else {
            rawText
        }
        val outgoingText = expandShortDevelopmentCommand(text, activeConversation().messages)
        val target = currentSendTarget()
        collapseInputComposer()
        if (pendingAttachments.isNotEmpty()) {
            uploadAttachmentsThenSend(text, outgoingText, target)
            return
        }
        startPreparedMessage(text, outgoingText, com.google.gson.JsonArray(), target, emptyList())
    }

    private fun startPreparedMessage(
        visibleText: String,
        outgoingText: String,
        attachmentRefs: com.google.gson.JsonArray,
        target: SendTarget,
        chatAttachments: List<ChatAttachment>
    ) {
        if (!restoreSendTarget(target)) {
            Toast.makeText(this, "Target conversation no longer exists.", Toast.LENGTH_LONG).show()
            setSendEnabled(true)
            return
        }
        if (runningConversationTasks.containsKey(conversationTaskKey(target.projectId, target.conversationId))) {
            Toast.makeText(this, "这个会话正在工作中，请换一个会话并行开发。", Toast.LENGTH_LONG).show()
            setSendEnabled(true)
            return
        }
        val traceId = "ui_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}"

        val payload = com.google.gson.JsonObject().apply {
            addProperty("trace_id", traceId)
            addProperty("client_request_id", traceId)
            addProperty("user_id", userId)
            addProperty("project_id", target.projectId)
            addProperty("project_title", target.projectTitle)
            addProperty("conversation_id", target.conversationId)
            addProperty("conversation_title", target.conversationTitle)
            addProperty("message", outgoingText)
            modelActions().selectedAgentForRequest()?.let { addProperty("agent", it) }
            if (attachmentRefs.size() > 0) add("attachments", attachmentRefs)
        }

        // 显示用户消息
        appendMessage(
            ChatMessage(
                role = "user",
                content = visibleText,
                attachments = chatAttachments.takeIf { it.isNotEmpty() }
            )
        )
        DebugTraceStore.record(
            "ui_chat_send",
            mapOf(
                "trace_id" to traceId,
                "project_id" to target.projectId,
                "conversation_id" to target.conversationId,
                "chars" to outgoingText.length,
                "attachment_refs" to attachmentRefs.size()
            )
        )
        binding.inputEdit.text.clear()
        collapseInputComposer()
        val requestIsDevelopment = looksLikeDevelopmentRequest(outgoingText) && !looksLikeDirectImageRequest(outgoingText)
        rememberConversationTask(target, traceId, payload.toString(), requestIsDevelopment)
        setSendEnabled(false)
        activeRequestIsDevelopment = requestIsDevelopment
        pendingReconnectForActiveWork = false
        reconnectAttempts = 0
        persistActiveWork()
        workflowStepIndex = 0
        resetFoldedCliLog()
        currentEvidenceEntries.clear()
        pendingToolActionBubbles.clear()
        emittedProgressSignals.clear()
        if (requestIsDevelopment) {
            updateProjectTitleFromRequest(visibleText)
            saveProjectTitle()
            addProjectEvent("提交需求：${summarize(visibleText, 36)}")
            updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
        } else {
            updateProjectViews("普通消息已发送，开发项目记录保持不变。")
        }
        appendMessage(
            CodexInteractionPresentation.intentMessage(
                visibleText = visibleText,
                outgoingText = outgoingText,
                isDevelopment = requestIsDevelopment,
                hasAttachments = attachmentRefs.size() > 0
            )
        )
        appendMessage(ChatMessage("ai-working", initialWorkflowMessage(requestIsDevelopment)))

        // 通过前台任务服务发送 JSON（包含 user_id，服务端据此隔离工作区）
        val responseToken = ++serverResponseToken
        taskResponseTokens[traceId] = responseToken
        if (!startTaskWorkService(TaskWorkService.ACTION_START_WORK, payload.toString(), requestIsDevelopment, traceId)) {
            runningConversationTasks[conversationTaskKey(target.projectId, target.conversationId)]?.pendingReconnect = true
            refreshActiveTaskState()
            persistActiveWork()
            if (requestIsDevelopment) {
                updateStage("连接恢复", "任务请求已保留，正在重新连接服务器。")
            }
            scheduleFirstServerResponseWatchdog(traceId, responseToken)
        } else {
            clearPendingAttachments()
            scheduleFirstServerResponseWatchdog(traceId, responseToken)
        }
    }

    private fun pauseCurrentWork() {
        val task = activeConversationTask() ?: return
        val wasDevelopment = task.isDevelopment
        removeConversationTask(task.traceId, task.projectId, task.conversationId)
        reconnectAttempts = 0
        persistActiveWork()
        stopWorkingEvidenceForActiveConversation()
        currentEvidenceEntries.clear()
        pendingToolActionBubbles.clear()
        setSendEnabled(true)
        if (wasDevelopment) {
            updateStage("工作暂停", "你已暂停当前任务，可以调整需求后继续发送。")
            addProjectEvent("暂停当前工作")
        } else {
            updateProjectViews("当前回复已暂停，你可以继续输入新的消息。")
        }
        appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("你已暂停当前工作。", wasDevelopment)))
        startTaskWorkService(TaskWorkService.ACTION_PAUSE, traceId = task.traceId)
    }

    private fun handleActiveWorkDisconnected(task: ConversationTaskState) {
        task.pendingReconnect = true
        refreshActiveTaskState()
        persistActiveWork()
        setSendEnabled(false)
        updateFirstConversationStatus("连接恢复中 · 回来后继续")
        if (activeRequestIsDevelopment) {
            updateStage(currentStage, "连接暂时断开，正在保留本轮任务并准备自动恢复。")
            recordEvidence("connection", "连接暂时断开，正在自动恢复任务")
        }

        scheduleReconnectForActiveWork(task.traceId)
    }

    private fun scheduleReconnectForActiveWork(traceId: String? = activeConversationTask()?.traceId) {
        val task = traceId?.let { runningTraceToConversation[it] }?.let { runningConversationTasks[it] } ?: return
        if (!task.pendingReconnect) return
        reconnectAttempts += 1
        val delay = (800L * reconnectAttempts).coerceAtMost(5_000L)
        binding.root.postDelayed({
            val current = runningTraceToConversation[traceId]?.let { runningConversationTasks[it] } ?: return@postDelayed
            if (!current.pendingReconnect || backendConnected) return@postDelayed
            startTaskWorkService(TaskWorkService.ACTION_RESUME_PENDING, traceId = current.traceId)
        }, delay)
    }

    private fun resumePendingWorkAfterReconnect() {
        val payload = pendingRequestPayload
        if (payload.isNullOrBlank()) {
            pendingReconnectForActiveWork = false
            waitingForReply = false
            activeRequestIsDevelopment = false
            stopWorkingEvidenceForActiveConversation()
            clearPersistedActiveWork()
            setSendEnabled(true)
            appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("连接已恢复，但没有找到可继续的请求。请重新发送一次。")))
            return
        }

        pendingReconnectForActiveWork = false
        recordEvidence("connection", "连接已恢复，已自动继续上次任务")
        if (activeRequestIsDevelopment) {
            updateStage(currentStage, "连接已恢复，正在继续本轮开发任务。")
            addProjectEvent("连接恢复，自动继续任务")
        }

        val responseToken = ++serverResponseToken
        if (!startTaskWorkService(TaskWorkService.ACTION_RESUME_PENDING)) {
            pendingReconnectForActiveWork = true
            persistActiveWork()
            scheduleReconnectForActiveWork()
        } else {
            activeConversationTask()?.let { scheduleFirstServerResponseWatchdog(it.traceId, responseToken) }
        }
    }

    @SuppressLint("ClickableViewAccessibility")
    private fun setupInputComposer() {
        val root = binding.inputLayout
        val inputEdit = binding.inputEdit
        val modelButton = binding.modelButton
        val sendButton = binding.sendButton

        inputEdit.detachFromParent()
        modelButton.detachFromParent()
        sendButton.detachFromParent()
        root.removeAllViews()
        root.orientation = LinearLayout.VERTICAL
        root.setPadding(0, 0, 0, 0)
        root.setBackgroundColor(Color.parseColor("#1E1E1E"))

        expandedInputContainer = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0
            ).apply {
                marginStart = dp(24)
                marginEnd = dp(24)
            }
        }

        inputBarContainer = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(60)
            )
            minimumHeight = dp(60)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), dp(6), dp(14), dp(6))
        }

        // WeChat-scale circular controls: compact 42dp touch area with a 30dp visual icon.
        inputModeButton = ImageButton(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                marginEnd = dp(10)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_voice_circle)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "切换语音输入"
            setOnClickListener { toggleVoiceMode() }
        }

        inputCenterContainer = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                dp(40),
                1f
            )
            setBackgroundResource(R.drawable.bg_input_pill)
            minimumHeight = dp(40)
            isClickable = true
            isFocusable = true
            setOnClickListener { focusInputComposer() }
        }

        collapsedInputPreview = TextView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setPadding(dp(18), 0, dp(18), 0)
            text = "文本内容在此输入。"
            setTextColor(Color.parseColor("#E1E1E1"))
            textSize = 15f
            isClickable = true
            setOnClickListener { focusInputComposer() }
        }

        inputEdit.apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            background = ColorDrawable(Color.TRANSPARENT)
            hint = "文本内容在此输入。"
            minLines = 1
            maxLines = 4
            setSingleLine(false)
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
            isVerticalScrollBarEnabled = false
            includeFontPadding = true
            setHorizontallyScrolling(false)
            setPadding(0, dp(8), 0, dp(6))
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#E1E1E1"))
            textSize = 15f
            setOnFocusChangeListener { _, hasFocus ->
                if (!voiceMode) {
                    inputComposerMotion.setExpanded(hasFocus, animate = !suppressInputFocusAnimation)
                    updateSendButtonVisual()
                    updateAdaptiveInputHeight()
                }
            }
        }

        voiceHoldButton = TextView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "按住 说话"
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 15f
            visibility = View.GONE
            setOnTouchListener { _, event ->
                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        startSpeechToText()
                        true
                    }
                    MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                        stopSpeechToText()
                        true
                    }
                    else -> true
                }
            }
        }

        modelButtonShell = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(86), dp(32)).apply {
                marginEnd = dp(10)
            }
            background = getDrawable(R.drawable.bg_model_pill_light)
            alpha = 0f
            visibility = View.GONE
            isClickable = true
            isFocusable = true
            contentDescription = "选择模型：${modelActions().currentModelLabel}"
            setOnClickListener { showModelPopupOrLoad() }
        }

        modelButton.apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setPadding(dp(16), 0, dp(30), 0)
            setCompoundDrawablesRelativeWithIntrinsicBounds(0, 0, 0, 0)
            setTextColor(Color.parseColor("#2D2D2D"))
            textSize = 12.5f
            setOnClickListener { showModelPopupOrLoad() }
        }

        val modelChevron = ImageView(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(13), dp(13), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                marginEnd = dp(13)
            }
            setImageResource(R.drawable.ic_input_model_chevron)
            scaleType = ImageView.ScaleType.CENTER
            alpha = 0.9f
            isClickable = false
            isFocusable = false
        }
        modelButtonShell.addView(modelButton)
        modelButtonShell.addView(modelChevron)

        inputCenterContainer.addView(collapsedInputPreview)
        expandedInputContainer.addView(inputEdit)
        expandedInputContainer.addView(voiceHoldButton)

        inputRightControls = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(42), dp(42))
        }

        attachmentButton = ImageButton(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(42), dp(42), Gravity.START or Gravity.CENTER_VERTICAL)
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_add_circle_simple)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "展开更多输入功能"
            setOnClickListener { toggleAttachmentPanel() }
        }

        sendButton.apply {
            layoutParams = FrameLayout.LayoutParams(dp(42), dp(42), Gravity.END or Gravity.CENTER_VERTICAL)
            background = getDrawable(R.drawable.ic_input_send_arrow_circle)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = ""
            setOnClickListener { sendMessage() }
        }

        inputBarContainer.addView(inputModeButton)
        inputBarContainer.addView(modelButtonShell)
        inputBarContainer.addView(inputCenterContainer)
        inputRightControls.addView(attachmentButton)
        inputRightControls.addView(sendButton)
        inputBarContainer.addView(inputRightControls)

        attachmentPanel = buildAttachmentPanel()
        root.addView(expandedInputContainer)
        root.addView(inputBarContainer)
        root.addView(attachmentPanel)

        inputComposerMotion = InputComposerMotion(
            expandedInputContainer = expandedInputContainer,
            collapsedInputContainer = inputCenterContainer,
            collapsedText = collapsedInputPreview,
            modelButton = modelButtonShell,
            rightControls = inputRightControls
        )
        inputEdit.setOnClickListener {
            if (!inputComposerMotion.isExpanded && !voiceMode) {
                inputComposerMotion.setExpanded(true, animate = true)
            }
        }

        inputEdit.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                updateCollapsedInputPreview()
                updateSendButtonVisual()
                updateAdaptiveInputHeight()
            }
            override fun afterTextChanged(s: Editable?) = Unit
        })

        binding.chatList.setOnTouchListener { _, event ->
            if (event.action == MotionEvent.ACTION_DOWN) {
                if (attachmentPanelOpen) {
                    collapseAttachmentPanel()
                }
                collapseInputComposer()
            }
            false
        }
        binding.stageHintText.setOnClickListener {
            collapseAttachmentPanel()
            collapseInputComposer()
        }
        applyVoiceMode()
        updateCollapsedInputPreview()
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun updateAdaptiveInputHeight() {
        if (!::inputCenterContainer.isInitialized || !::inputBarContainer.isInitialized) return
        val inputEdit = binding.inputEdit
        inputEdit.post {
            if (!::inputCenterContainer.isInitialized || !::inputBarContainer.isInitialized) return@post
            val collapsedHeight = dp(40)
            val minTextHeight = dp(46)
            val maxTextHeight = dp(112)
            val rawLineCount = inputEdit.lineCount.coerceAtLeast(1)
            val desiredTextHeight = if (voiceMode) {
                0
            } else {
                val multilineTopGuard = if (rawLineCount > 1) dp(8) else 0
                (rawLineCount.coerceAtMost(4) * inputEdit.lineHeight +
                    inputEdit.paddingTop +
                    inputEdit.paddingBottom +
                    multilineTopGuard).coerceIn(minTextHeight, maxTextHeight)
            }

            val centerParams = inputCenterContainer.layoutParams as LinearLayout.LayoutParams
            if (centerParams.height != collapsedHeight) {
                centerParams.height = collapsedHeight
                inputCenterContainer.layoutParams = centerParams
            }

            if (::inputComposerMotion.isInitialized) {
                inputComposerMotion.updateExpandedTextHeight(
                    desiredTextHeight,
                    animate = inputComposerMotion.isExpanded
                )
            }

            val multiline = !voiceMode && rawLineCount > 1
            inputEdit.gravity = (if (multiline) Gravity.TOP else Gravity.CENTER_VERTICAL) or Gravity.START
            inputEdit.isVerticalScrollBarEnabled = !voiceMode && rawLineCount > 4
        }
    }

    private fun updateCollapsedInputPreview() {
        if (!::collapsedInputPreview.isInitialized) return
        val draft = binding.inputEdit.text?.toString().orEmpty()
        collapsedInputPreview.text = draft.ifBlank { "文本内容在此输入。" }
    }

    private fun focusInputComposer() {
        if (activeConversation().ended) return
        if (voiceMode) {
            voiceMode = false
            applyVoiceMode()
        }
        if (::inputComposerMotion.isInitialized && !inputComposerMotion.isExpanded) {
            inputComposerMotion.setExpanded(true, animate = true)
        }
        binding.inputEdit.requestFocus()
        binding.inputEdit.post {
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            imm?.showSoftInput(binding.inputEdit, InputMethodManager.SHOW_IMPLICIT)
        }
        binding.inputEdit.postDelayed({
            if (!binding.inputEdit.hasFocus()) return@postDelayed
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            imm?.showSoftInput(binding.inputEdit, InputMethodManager.SHOW_IMPLICIT)
        }, 120L)
    }

    private fun collapseInputComposer(animate: Boolean = true) {
        if (!::inputComposerMotion.isInitialized) return
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.inputEdit.windowToken, 0)
        if (binding.inputEdit.hasFocus()) {
            suppressInputFocusAnimation = !animate
            try {
                binding.inputEdit.clearFocus()
            } finally {
                suppressInputFocusAnimation = false
            }
        }
        if (inputComposerMotion.isExpanded) {
            inputComposerMotion.setExpanded(false, animate = animate)
        }
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun setupAttachmentLaunchers() {
        cameraAttachmentLauncher = registerForActivityResult(ActivityResultContracts.TakePicture()) { success ->
            val uri = pendingCameraUri
            val name = pendingCameraName
            pendingCameraUri = null
            pendingCameraName = null
            if (success && uri != null) {
                attachPickedFile("相机照片", uri, name)
            } else {
                Toast.makeText(this, "已取消拍摄", Toast.LENGTH_SHORT).show()
            }
        }
        photoAttachmentLauncher = registerForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
            if (uri != null) {
                attachPickedFile("相册图片", uri)
            } else {
                Toast.makeText(this, "已取消选择相册", Toast.LENGTH_SHORT).show()
            }
        }
        documentAttachmentLauncher = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (uri != null) {
                runCatching {
                    contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                attachPickedFile("文档", uri)
            } else {
                Toast.makeText(this, "已取消选择文档", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun buildAttachmentPanel(): LinearLayout {
        return LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(104)
            )
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(18), dp(8), dp(18), dp(8))
            visibility = View.GONE

            addView(createAttachmentAction("相机", R.drawable.ic_attach_camera, addEndMargin = true) {
                openCameraAttachment()
            })
            addView(createAttachmentAction("相册", R.drawable.ic_attach_photos, addEndMargin = true) {
                openPhotoAttachment()
            })
            addView(createAttachmentAction("文档", R.drawable.ic_attach_files, addEndMargin = false) {
                openDocumentAttachment()
            })
        }
    }

    private fun createAttachmentAction(
        label: String,
        iconRes: Int,
        addEndMargin: Boolean = true,
        action: () -> Unit
    ): View {
        return LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f).apply {
                if (addEndMargin) marginEnd = dp(8)
            }
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(Color.parseColor("#242424"))
                setStroke(dp(1), Color.parseColor("#444444"))
            }
            gravity = Gravity.CENTER
            orientation = LinearLayout.VERTICAL
            isClickable = true
            foreground = selectableForeground()

            addView(ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(30), dp(30))
                setImageResource(iconRes)
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(8)
                }
                includeFontPadding = false
                text = label
                setTextColor(Color.WHITE)
                textSize = 14f
            })
            setOnClickListener {
                collapseAttachmentPanel()
                action()
            }
        }
    }

    private fun openCameraAttachment() {
        if (activeConversation().ended) return
        val attachmentDir = File(cacheDir, "attachments").apply { mkdirs() }
        val fileName = "camera_${System.currentTimeMillis()}.jpg"
        val file = File(attachmentDir, fileName)
        val uri = FileProvider.getUriForFile(this, "com.elon.app.fileprovider", file)
        pendingCameraUri = uri
        pendingCameraName = fileName
        runCatching {
            cameraAttachmentLauncher.launch(uri)
        }.onFailure {
            pendingCameraUri = null
            pendingCameraName = null
            Toast.makeText(this, "无法打开相机", Toast.LENGTH_SHORT).show()
        }
    }

    private fun openPhotoAttachment() {
        if (activeConversation().ended) return
        runCatching {
            photoAttachmentLauncher.launch(
                PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)
            )
        }.onFailure {
            Toast.makeText(this, "无法打开相册", Toast.LENGTH_SHORT).show()
        }
    }

    private fun openDocumentAttachment() {
        if (activeConversation().ended) return
        runCatching {
            documentAttachmentLauncher.launch(arrayOf("*/*"))
        }.onFailure {
            Toast.makeText(this, "无法打开文档选择器", Toast.LENGTH_SHORT).show()
        }
    }

    private fun attachPickedFile(kind: String, uri: Uri, fallbackName: String? = null) {
        val name = fallbackName ?: displayNameForUri(this, uri) ?: uri.lastPathSegment ?: kind
        val attachment = runCatching {
            copyAttachmentToCache(this, kind, uri, name, pendingAttachments.size + 1)
        }.onFailure {
            Toast.makeText(this, "附件读取失败，请重新选择", Toast.LENGTH_SHORT).show()
        }.getOrNull() ?: return

        pendingAttachments.add(attachment)
        appendAttachmentLabel(attachment.displayLabel, attachment.displayName)
        Toast.makeText(this, "已添加${attachment.displayLabel}：${attachment.displayName}", Toast.LENGTH_SHORT).show()
    }

    private fun appendAttachmentLabel(kind: String, name: String) {
        if (voiceMode) {
            voiceMode = false
            applyVoiceMode()
        }
        val current = binding.inputEdit.text.toString()
        val prefix = if (current.isBlank()) "" else "\n"
        binding.inputEdit.append("${prefix}[$kind] $name")
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
        updateSendButtonVisual()
    }

    private fun currentSendTarget(): SendTarget {
        val project = activeProject()
        val conversation = activeConversation()
        return SendTarget(
            projectId = project.id,
            projectTitle = project.title,
            conversationId = conversation.id,
            conversationTitle = conversation.title
        )
    }

    private fun restoreSendTarget(target: SendTarget): Boolean {
        val projectIndex = projects.indexOfFirst { it.id == target.projectId }
        if (projectIndex < 0) return false
        val project = projects[projectIndex]
        val conversationIndex = project.conversations.indexOfFirst { it.id == target.conversationId }
        if (conversationIndex < 0) return false
        activeProjectIndex = projectIndex
        project.activeConversationIndex = conversationIndex
        chatAdapter = ChatAdapter(project.conversations[conversationIndex].messages, ::pauseCurrentWork, ::showMessageActions)
        binding.chatList.adapter = chatAdapter
        showChat()
        if (chatAdapter.itemCount > 0) {
            binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
        }
        return true
    }

    private fun uploadAttachmentsThenSend(visibleText: String, outgoingText: String, target: SendTarget) {
        val attachments = pendingAttachments.toList()
        setSendEnabled(false)
        DebugTraceStore.record(
            "ui_attachment_upload_start",
            mapOf("project_id" to target.projectId, "conversation_id" to target.conversationId, "count" to attachments.size)
        )
        Thread {
            val startedAt = System.currentTimeMillis()
            val refs = uploadAttachmentRefsOrNull(
                http = http,
                serverUrl = serverUrl,
                userId = userId,
                attachments = attachments,
                target = target,
                maxAttachmentBytes = MAX_ATTACHMENT_BYTES,
                showShortToast = { message ->
                    runOnUiThread { Toast.makeText(this, message, Toast.LENGTH_SHORT).show() }
                },
                showLongToast = { message ->
                    runOnUiThread { Toast.makeText(this, message, Toast.LENGTH_LONG).show() }
                }
            )
            runOnUiThread {
                if (refs == null) {
                    setSendEnabled(true)
                    return@runOnUiThread
                }
                DebugTraceStore.record(
                    "ui_attachment_upload_done",
                    mapOf(
                        "project_id" to target.projectId,
                        "conversation_id" to target.conversationId,
                        "count" to refs.size(),
                        "elapsed_ms" to (System.currentTimeMillis() - startedAt)
                    )
                )
                startPreparedMessage(visibleText, outgoingText, refs, target, chatAttachmentsFromRefs(refs))
            }
        }.start()
    }

    private fun clearPendingAttachments() {
        pendingAttachments.forEach { attachment ->
            runCatching { attachment.file.delete() }
        }
        pendingAttachments.clear()
    }

    private fun handleSendOrAttachment() {
        if (!voiceMode && binding.inputEdit.text.toString().trim().isNotEmpty()) {
            sendMessage()
        } else {
            toggleAttachmentPanel()
        }
    }

    private fun toggleAttachmentPanel() {
        if (attachmentPanelOpen) collapseAttachmentPanel() else expandAttachmentPanel()
    }

    private fun expandAttachmentPanel() {
        if (activeConversation().ended) return
        collapseInputComposer()
        if (attachmentPanelOpen) return
        attachmentPanelOpen = true
        attachmentPanel.visibility = View.VISIBLE
        animateAttachmentButtonIcon(expanded = true)
    }

    private fun collapseAttachmentPanel() {
        if (!::attachmentPanel.isInitialized) return
        val wasOpen = attachmentPanelOpen || attachmentPanel.visibility == View.VISIBLE
        attachmentPanelOpen = false
        attachmentPanel.visibility = View.GONE
        if (wasOpen) {
            animateAttachmentButtonIcon(expanded = false)
        } else {
            updateAttachmentButtonIcon(expanded = false)
        }
    }

    private fun animateAttachmentButtonIcon(expanded: Boolean) {
        if (!::attachmentButton.isInitialized) return
        val token = ++attachmentIconAnimationToken
        val targetAlpha = if (activeConversation().ended) 0.55f else 1f
        attachmentButton.animate().cancel()
        attachmentButton.rotation = 0f
        attachmentButton.scaleX = 1f
        attachmentButton.scaleY = 1f
        attachmentButton.animate()
            .alpha(0.55f)
            .setDuration(70L)
            .withEndAction {
                if (token != attachmentIconAnimationToken) return@withEndAction
                updateAttachmentButtonIcon(expanded)
                attachmentButton.animate()
                    .alpha(targetAlpha)
                    .setDuration(90L)
                    .start()
            }
            .start()
    }

    private fun updateAttachmentButtonIcon(expanded: Boolean) {
        if (!::attachmentButton.isInitialized) return
        attachmentButton.setImageResource(
            if (expanded) R.drawable.ic_input_chevron_down_circle else R.drawable.ic_add_circle_simple
        )
        attachmentButton.contentDescription = if (expanded) "收起更多输入功能" else "展开更多输入功能"
    }

    private fun toggleVoiceMode() {
        voiceMode = !voiceMode
        collapseAttachmentPanel()
        applyVoiceMode()
    }

    private fun applyVoiceMode() {
        if (!::voiceHoldButton.isInitialized) return
        if (voiceMode) {
            hideKeyboard()
            inputModeButton.setImageResource(R.drawable.ic_input_keyboard_circle)
            if (::inputComposerMotion.isInitialized) {
                inputComposerMotion.setExpanded(false, animate = true)
            }
            voiceHoldButton.detachFromParent()
            inputCenterContainer.removeAllViews()
            inputCenterContainer.addView(voiceHoldButton)
            binding.inputEdit.visibility = View.GONE
            if (::modelButtonShell.isInitialized) modelButtonShell.visibility = View.GONE
            voiceHoldButton.visibility = View.VISIBLE
        } else {
            inputModeButton.setImageResource(R.drawable.ic_input_voice_circle)
            collapsedInputPreview.detachFromParent()
            voiceHoldButton.detachFromParent()
            inputCenterContainer.removeAllViews()
            inputCenterContainer.addView(collapsedInputPreview)
            expandedInputContainer.addView(voiceHoldButton)
            binding.inputEdit.visibility = View.VISIBLE
            if (::modelButtonShell.isInitialized) modelButtonShell.visibility = if (::inputComposerMotion.isInitialized && inputComposerMotion.isExpanded) {
                View.VISIBLE
            } else {
                View.GONE
            }
            voiceHoldButton.visibility = View.GONE
        }
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun updateSendButtonVisual() {
        if (!::binding.isInitialized) return
        val hasText = binding.inputEdit.text.toString().trim().isNotEmpty()
        val composerExpanded = ::inputComposerMotion.isInitialized && inputComposerMotion.isExpanded
        val sendMode = hasText && !voiceMode && composerExpanded
        val params = binding.sendButton.layoutParams as? FrameLayout.LayoutParams
        if (sendMode) {
            params?.width = dp(42)
            binding.sendButton.background = getDrawable(R.drawable.ic_input_send_arrow_circle)
            binding.sendButton.text = ""
            binding.sendButton.visibility = View.VISIBLE
            if (::attachmentButton.isInitialized) {
                attachmentButton.visibility = View.VISIBLE
            }
        } else {
            binding.sendButton.visibility = View.GONE
            if (::attachmentButton.isInitialized) {
                attachmentButton.visibility = View.VISIBLE
            }
        }
        params?.height = dp(42)
        params?.gravity = Gravity.END or Gravity.CENTER_VERTICAL
        params?.let { binding.sendButton.layoutParams = it }
        if (::attachmentButton.isInitialized) {
            val attachmentParams = attachmentButton.layoutParams as? FrameLayout.LayoutParams
            attachmentParams?.gravity = if (sendMode) {
                Gravity.START or Gravity.CENTER_VERTICAL
            } else {
                Gravity.END or Gravity.CENTER_VERTICAL
            }
            attachmentParams?.let { attachmentButton.layoutParams = it }
        }
        val conversationEnded = activeConversation().ended
        binding.sendButton.isEnabled = !conversationEnded && (!sendMode || inputCanSend)
        binding.sendButton.alpha = if (!conversationEnded && (!sendMode || inputCanSend)) 1f else 0.55f
        if (::attachmentButton.isInitialized) {
            attachmentButton.isEnabled = !conversationEnded
            attachmentButton.alpha = if (conversationEnded) 0.55f else 1f
        }
    }

    private fun startSpeechToText() {
        if (activeConversation().ended) return
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(this, arrayOf(Manifest.permission.RECORD_AUDIO), speechPermissionRequest)
            return
        }
        if (!SpeechRecognizer.isRecognitionAvailable(this)) {
            Toast.makeText(this, "当前设备不可用语音识别", Toast.LENGTH_SHORT).show()
            return
        }
        if (speechRecognizer == null) {
            speechRecognizer = SpeechRecognizer.createSpeechRecognizer(this).apply {
                setRecognitionListener(createSpeechRecognitionListener())
            }
        }
        isListeningForSpeech = true
        voiceHoldButton.text = "松开 转文字"
        speechRecognizer?.startListening(Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.CHINA.toLanguageTag())
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
        })
    }

    private fun stopSpeechToText() {
        if (!isListeningForSpeech) return
        isListeningForSpeech = false
        voiceHoldButton.text = "识别中..."
        speechRecognizer?.stopListening()
    }

    private fun createSpeechRecognitionListener(): RecognitionListener {
        return object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                voiceHoldButton.text = "正在听..."
            }
            override fun onBeginningOfSpeech() = Unit
            override fun onRmsChanged(rmsdB: Float) = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
            override fun onEndOfSpeech() {
                voiceHoldButton.text = "识别中..."
            }
            override fun onError(error: Int) {
                isListeningForSpeech = false
                voiceHoldButton.text = "按住 说话"
                if (error != SpeechRecognizer.ERROR_NO_MATCH && error != SpeechRecognizer.ERROR_SPEECH_TIMEOUT) {
                    Toast.makeText(this@MainActivity, "语音识别失败，请重试", Toast.LENGTH_SHORT).show()
                }
            }
            override fun onResults(results: Bundle?) {
                isListeningForSpeech = false
                voiceHoldButton.text = "按住 说话"
                val spoken = results
                    ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                    ?.firstOrNull()
                    .orEmpty()
                    .trim()
                if (spoken.isNotBlank()) {
                    voiceMode = false
                    applyVoiceMode()
                    binding.inputEdit.setText(spoken)
                    binding.inputEdit.setSelection(binding.inputEdit.text.length)
                }
            }
            override fun onPartialResults(partialResults: Bundle?) = Unit
            override fun onEvent(eventType: Int, params: Bundle?) = Unit
        }
    }

    private fun hideKeyboard() {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.root.windowToken, 0)
        binding.inputEdit.clearFocus()
    }

    private fun setupNavigation() {
        val tabs = listOf(binding.tabChat, binding.tabProject, binding.tabProfile)

        fun select(tab: TextView) {
            pageTransitionRunning = false
            clearPageTranslations()
            tabs.forEach {
                it.setTextColor(Color.parseColor(if (it == tab) "#07C160" else "#A5A5A5"))
                it.textSize = if (it == tab) 12f else 11f
            }
            binding.conversationPage.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
            binding.chatPage.visibility = View.GONE
            binding.projectPage.visibility = if (tab == binding.tabProject) View.VISIBLE else View.GONE
            binding.profilePage.visibility = if (tab == binding.tabProfile) View.VISIBLE else View.GONE
            binding.inputLayout.visibility = View.GONE
            binding.pageTabs.visibility = View.VISIBLE
            binding.backButton.visibility = View.GONE
            binding.searchButton.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
            binding.addButton.visibility = if (tab == binding.tabChat || tab == binding.tabProject) View.VISIBLE else View.GONE
            binding.moreButton.visibility = View.GONE
            binding.addButton.setOnClickListener {
                showHomeActionPopup(binding.addButton, tab)
            }
            binding.topTitleText.setOnLongClickListener(null)
            binding.topTitleText.text = when (tab) {
                binding.tabProject -> "项目管理"
                binding.tabProfile -> "我的"
                else -> compactProjectTitle()
            }
            if (tab != binding.tabChat) {
                renderConversationList()
            }
            if (tab == binding.tabProject) {
                renderProjectList()
            } else if (tab == binding.tabChat) {
                renderConversationList()
            } else if (tab == binding.tabProfile) {
                refreshServerVersion()
            }
        }

        binding.tabChat.setOnClickListener { select(binding.tabChat) }
        binding.tabProject.setOnClickListener { select(binding.tabProject) }
        binding.tabProfile.setOnClickListener { select(binding.tabProfile) }
        binding.conversationItem.setOnClickListener { openConversation(0) }
        binding.conversationItem.setOnLongClickListener {
            showConversationActions(0)
            true
        }
        binding.searchButton.setOnClickListener { updateFirstConversationStatus("搜索功能准备中 · 点击进入开发会话") }
        binding.moreButton.setOnClickListener { showChatActionPopup(binding.moreButton) }
        binding.backButton.setOnClickListener { navigateBackOneLevel() }
        select(binding.tabChat)
    }

    private fun setupBackHandling() {
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                navigateBackOneLevel()
            }
        })
    }

    private fun navigateBackOneLevel() {
        if (pageTransitionRunning) return
        if (binding.chatPage.visibility == View.VISIBLE) {
            collapseInputComposer(animate = false)
            showConversationHome(animate = true)
            return
        }
        showExitConfirmation()
    }

    private fun showConversationHome(animate: Boolean = false) {
        if (animate && binding.chatPage.visibility == View.VISIBLE) {
            actionPopup?.dismiss()
            renderConversationList()
            applyConversationHomeChrome()
            pageTransitionRunning = true
            WechatPageTransition.exitToRight(
                container = binding.contentContainer,
                outgoing = listOf(binding.chatPage),
                incoming = listOf(binding.conversationPage),
                outgoingFull = listOf(binding.inputLayout),
                incomingFull = listOf(binding.pageTabs),
                onEnd = {
                    binding.chatPage.visibility = View.GONE
                    binding.inputLayout.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.conversationPage.visibility = View.VISIBLE
                    binding.pageTabs.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            binding.tabChat.performClick()
        }
    }

    private fun showExitConfirmation() {
        if (exitConfirmDialog?.isShowing == true) return
        exitConfirmDialog = AlertDialog.Builder(this)
            .setTitle("退出应用")
            .setMessage("确定要退出一龙吗？")
            .setNegativeButton("取消", null)
            .setPositiveButton("退出") { _, _ -> finish() }
            .create()
        exitConfirmDialog?.show()
    }

    private fun showChat(animate: Boolean = false) {
        if (pageTransitionRunning) return
        val shouldAnimate = animate && binding.conversationPage.visibility == View.VISIBLE
        actionPopup?.dismiss()
        applyChatChrome()
        if (shouldAnimate) {
            collapseInputComposer(animate = false)
            pageTransitionRunning = true
            WechatPageTransition.enterFromRight(
                container = binding.contentContainer,
                incoming = listOf(binding.chatPage),
                outgoing = listOf(binding.conversationPage),
                incomingFull = listOf(binding.inputLayout),
                outgoingFull = listOf(binding.pageTabs),
                onEnd = {
                    binding.conversationPage.visibility = View.GONE
                    binding.pageTabs.visibility = View.GONE
                    binding.projectPage.visibility = View.GONE
                    binding.profilePage.visibility = View.GONE
                    binding.chatPage.visibility = View.VISIBLE
                    binding.inputLayout.visibility = View.VISIBLE
                    clearPageTranslations()
                    pageTransitionRunning = false
                }
            )
        } else {
            binding.conversationPage.visibility = View.GONE
            binding.pageTabs.visibility = View.GONE
            binding.projectPage.visibility = View.GONE
            binding.profilePage.visibility = View.GONE
            binding.chatPage.visibility = View.VISIBLE
            binding.inputLayout.visibility = View.VISIBLE
            clearPageTranslations()
        }
        setSendEnabled(!isActiveConversationWorking())
        maybePrewarmCodexSession("show_chat")
    }

    private fun applyChatChrome() {
        binding.conversationPage.visibility = View.GONE
        binding.chatPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.inputLayout.visibility = View.VISIBLE
        binding.pageTabs.visibility = View.GONE
        binding.backButton.visibility = View.VISIBLE
        binding.searchButton.visibility = View.GONE
        binding.addButton.visibility = View.GONE
        binding.moreButton.visibility = View.VISIBLE
        renderConversationList()
        binding.topTitleText.text = activeConversation().title
        binding.topTitleText.setOnLongClickListener {
            showConversationActions(activeConversationIndex)
            true
        }
    }

    private fun applyConversationHomeChrome() {
        binding.tabChat.setTextColor(Color.parseColor("#07C160"))
        binding.tabChat.textSize = 12f
        binding.tabProject.setTextColor(Color.parseColor("#A5A5A5"))
        binding.tabProject.textSize = 11f
        binding.tabProfile.setTextColor(Color.parseColor("#A5A5A5"))
        binding.tabProfile.textSize = 11f
        binding.conversationPage.visibility = View.VISIBLE
        binding.projectPage.visibility = View.GONE
        binding.profilePage.visibility = View.GONE
        binding.inputLayout.visibility = View.GONE
        binding.pageTabs.visibility = View.VISIBLE
        binding.backButton.visibility = View.GONE
        binding.searchButton.visibility = View.VISIBLE
        binding.addButton.visibility = View.VISIBLE
        binding.moreButton.visibility = View.GONE
        binding.addButton.setOnClickListener {
            showHomeActionPopup(binding.addButton, binding.tabChat)
        }
        binding.topTitleText.setOnLongClickListener(null)
        binding.topTitleText.text = compactProjectTitle()
    }

    private fun clearPageTranslations() {
        binding.conversationPage.translationX = 0f
        binding.chatPage.translationX = 0f
        binding.projectPage.translationX = 0f
        binding.profilePage.translationX = 0f
        binding.inputLayout.translationX = 0f
        binding.pageTabs.translationX = 0f
    }

    private fun showCreateConversationDialog() {
        val input = titleEditText("新会话 ${conversations.size + 1}")
        val dialog = AlertDialog.Builder(this)
            .setTitle("新建会话")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入会话标题"
                    return@setOnClickListener
                }
                createConversation(title)
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    private fun showCreateProjectDialog() {
        val input = titleEditText("新项目 ${projects.size + 1}")
        val dialog = AlertDialog.Builder(this)
            .setTitle("新建项目")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                createProject(title)
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    private fun createConversation(title: String) {
        conversations.add(
            newAppConversation(title, "点击进入开发会话")
        )
        activeProject().updatedAt = System.currentTimeMillis()
        activeProject().subtitle = "${conversations.size} 个会话"
        saveConversations()
        renderConversationList()
    }

    private fun createProject(title: String) {
        projects.add(newAppProject(title, "新项目 · 点击进入会话"))
        activeProjectIndex = projects.lastIndex
        activeConversationIndex = 0
        saveProjects()
        renderProjectList()
        binding.tabChat.performClick()
    }

    private fun showConversationActions(index: Int) {
        if (index !in conversations.indices) return
        val conversation = conversations[index]
        val actions = if (conversation.ended) {
            arrayOf("编辑标题", "删除会话")
        } else {
            arrayOf("编辑标题", "结束会话", "删除会话")
        }

        AlertDialog.Builder(this)
            .setTitle(conversation.title)
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "编辑标题" -> showRenameConversationDialog(index)
                    "结束会话" -> confirmEndConversation(index)
                    "删除会话" -> confirmDeleteConversation(index)
                }
            }
            .show()
    }

    private fun showRenameConversationDialog(index: Int) {
        if (index !in conversations.indices) return
        val conversation = conversations[index]
        val input = titleEditText(conversation.title)
        val dialog = AlertDialog.Builder(this)
            .setTitle("编辑会话标题")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入会话标题"
                    return@setOnClickListener
                }
                conversation.title = summarize(title, 24)
                conversation.updatedAt = System.currentTimeMillis()
                saveConversations()
                renderConversationList()
                if (activeConversationIndex == index && binding.chatPage.visibility == View.VISIBLE) {
                    binding.topTitleText.text = conversation.title
                }
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    private fun confirmEndConversation(index: Int) {
        if (index !in conversations.indices) return
        AlertDialog.Builder(this)
            .setTitle("结束会话")
            .setMessage("结束后仍可查看记录，但不能继续发送消息。")
            .setNegativeButton("取消", null)
            .setPositiveButton("结束") { _, _ -> endConversation(index) }
            .show()
    }

    private fun endConversation(index: Int) {
        if (index !in conversations.indices) return
        val conversation = conversations[index]
        conversation.ended = true
        conversation.subtitle = "会话已结束"
        conversation.updatedAt = System.currentTimeMillis()
        activeProject().updatedAt = conversation.updatedAt
        conversation.messages.add(ChatMessage("ai", "本会话已结束，可以在会话列表长按删除，或新建会话继续。"))
        saveConversations()
        renderConversationList()

        if (activeConversationIndex == index && binding.chatPage.visibility == View.VISIBLE) {
            chatAdapter.notifyItemInserted(conversation.messages.lastIndex)
            binding.chatList.scrollToPosition(conversation.messages.lastIndex)
            setSendEnabled(false)
        }
    }

    private fun confirmDeleteConversation(index: Int) {
        if (index !in conversations.indices) return
        AlertDialog.Builder(this)
            .setTitle("删除会话")
            .setMessage("删除后这条会话记录会从本机移除。")
            .setNegativeButton("取消", null)
            .setPositiveButton("删除") { _, _ -> deleteConversation(index) }
            .show()
    }

    private fun deleteConversation(index: Int) {
        if (index !in conversations.indices) return
        conversations.removeAt(index)
        if (conversations.isEmpty()) {
            conversations.add(defaultAppConversation())
        }
        activeProject().subtitle = "${conversations.size} 个会话"
        activeProject().updatedAt = System.currentTimeMillis()
        activeConversationIndex = activeConversationIndex.coerceAtMost(conversations.lastIndex)
        saveConversations()
        renderConversationList()
        if (binding.chatPage.visibility == View.VISIBLE) {
            binding.tabChat.performClick()
        }
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
            openSettings = ::openSettings,
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
        chatAdapter = ChatAdapter(activeConversation().messages, ::pauseCurrentWork, ::showMessageActions)
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
        projects.clear()
        val savedProjects = prefs.getString("projects_json", null)
        val loadedProjects = runCatching {
            if (savedProjects.isNullOrBlank()) null
            else gson.fromJson(savedProjects, Array<AppProject>::class.java)?.toMutableList()
        }.getOrNull()

        loadedProjects.orEmpty()
            .filter { it.title.isNotBlank() }
            .forEach {
                normalizeProject(it)
                projects.add(it)
            }

        if (projects.isEmpty()) {
            projects.add(legacyAppProject(prefs, gson))
        }

        // 确保「一龙项目」（平台自身源码）始终作为第一个项目存在
        val elonSelfId = "elon-self"
        if (projects.none { it.id == elonSelfId }) {
            val elonProject = AppProject(
                id = elonSelfId,
                title = "一龙项目",
                subtitle = "修改平台自身 · AI 云端迭代",
                updatedAt = 0L,
                conversations = mutableListOf(AppConversation(
                    id = "elon-self-default",
                    title = "一龙项目",
                    subtitle = "连接中...",
                    updatedAt = 0L,
                    messages = mutableListOf(ChatMessage(
                        "ai",
                        "你可以直接告诉我想给 APK 加什么功能，例如「加一个深色模式切换」——我会先确认理解，再修改源码、检查结果并把新 APK 发给你。"
                    ))
                ))
            )
            projects.add(0, elonProject)
        }

        activeProjectIndex = prefs.getInt("active_project_index", 0).coerceIn(0, projects.lastIndex)
        activeProject()
        saveProjects()
    }

    private fun saveConversations() {
        saveProjects()
    }

    private fun saveProjects() {
        prefs.edit()
            .putString("projects_json", gson.toJson(projects))
            .putInt("active_project_index", activeProjectIndex)
            .putString(TaskWorkService.PREF_ACTIVE_PROJECT_ID, activeProject().id)
            .apply()
    }

    private fun conversationTaskKey(projectId: String, conversationId: String): String {
        return "$projectId\u001F$conversationId"
    }

    private fun activeConversationTaskKey(): String {
        return conversationTaskKey(activeProject().id, activeConversation().id)
    }

    private fun isActiveConversationWorking(): Boolean {
        return runningConversationTasks.containsKey(activeConversationTaskKey())
    }

    private fun activeConversationTask(): ConversationTaskState? {
        return runningConversationTasks[activeConversationTaskKey()]
    }

    private fun rememberConversationTask(
        target: SendTarget,
        traceId: String,
        payload: String,
        isDevelopment: Boolean
    ) {
        val key = conversationTaskKey(target.projectId, target.conversationId)
        runningConversationTasks[key] = ConversationTaskState(
            traceId = traceId,
            projectId = target.projectId,
            conversationId = target.conversationId,
            payload = payload,
            isDevelopment = isDevelopment
        )
        runningTraceToConversation[traceId] = key
        refreshActiveTaskState()
    }

    private fun updateConversationTaskFromService(
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?,
        pendingReconnect: Boolean? = null
    ): ConversationTaskState? {
        val key = when {
            !traceId.isNullOrBlank() && runningTraceToConversation.containsKey(traceId) ->
                runningTraceToConversation[traceId]
            !projectId.isNullOrBlank() && !conversationId.isNullOrBlank() ->
                conversationTaskKey(projectId, conversationId)
            else -> null
        } ?: return null
        val existing = runningConversationTasks[key] ?: return null
        if (!traceId.isNullOrBlank()) runningTraceToConversation[traceId] = key
        isDevelopment?.let { existing.isDevelopment = it }
        pendingReconnect?.let { existing.pendingReconnect = it }
        refreshActiveTaskState()
        return existing
    }

    private fun removeConversationTask(
        traceId: String?,
        projectId: String?,
        conversationId: String?
    ): ConversationTaskState? {
        val key = when {
            !traceId.isNullOrBlank() -> runningTraceToConversation.remove(traceId)
            !projectId.isNullOrBlank() && !conversationId.isNullOrBlank() ->
                conversationTaskKey(projectId, conversationId)
            else -> null
        } ?: return null
        val removed = runningConversationTasks.remove(key)
        removed?.let {
            runningTraceToConversation.entries.removeAll { entry -> entry.value == key }
            taskResponseTokens.remove(it.traceId)
        }
        refreshActiveTaskState()
        return removed
    }

    private fun refreshActiveTaskState() {
        waitingForReply = runningConversationTasks.isNotEmpty()
        val activeTask = activeConversationTask()
        activeRequestIsDevelopment = activeTask?.isDevelopment
            ?: runningConversationTasks.values.lastOrNull()?.isDevelopment
            ?: false
        pendingRequestPayload = activeTask?.payload
        pendingReconnectForActiveWork = activeTask?.pendingReconnect ?: false
        setSendEnabled(!isActiveConversationWorking())
        renderConversationList()
    }

    private fun persistActiveWork() {
        val array = JSONArray()
        runningConversationTasks.values.forEach { task ->
            array.put(
                JSONObject()
                    .put("payload", task.payload)
                    .put("is_development", task.isDevelopment)
                    .put("started_at", task.startedAt)
            )
        }
        if (array.length() == 0) {
            clearPersistedActiveWork()
            return
        }
        prefs.edit()
            .putString(TaskWorkService.PREF_PENDING_WORK_TASKS, array.toString())
            .remove(PREF_PENDING_WORK_PAYLOAD)
            .remove(PREF_PENDING_WORK_IS_DEVELOPMENT)
            .remove(PREF_PENDING_WORK_TIME)
            .apply()
    }

    private fun clearPersistedActiveWork() {
        prefs.edit()
            .remove(TaskWorkService.PREF_PENDING_WORK_TASKS)
            .remove(PREF_PENDING_WORK_PAYLOAD)
            .remove(PREF_PENDING_WORK_IS_DEVELOPMENT)
            .remove(PREF_PENDING_WORK_TIME)
            .apply()
    }

    private fun restorePendingActiveWork() {
        val now = System.currentTimeMillis()
        val tasksJson = prefs.getString(TaskWorkService.PREF_PENDING_WORK_TASKS, null)?.takeIf { it.isNotBlank() }
        if (tasksJson != null) {
            val array = runCatching { JSONArray(tasksJson) }.getOrNull()
            if (array != null) {
                for (index in 0 until array.length()) {
                    val item = array.optJSONObject(index) ?: continue
                    val payload = item.optString("payload").takeIf { it.isNotBlank() } ?: continue
                    val savedAt = item.optLong("started_at", now)
                    if (savedAt <= 0L || now - savedAt > PENDING_WORK_TTL_MS) continue
                    val parsed = runCatching { JSONObject(payload) }.getOrNull() ?: continue
                    val traceId = parsed.optString("trace_id").takeIf { it.isNotBlank() } ?: continue
                    val projectId = parsed.optString("project_id").takeIf { it.isNotBlank() } ?: activeProject().id
                    val conversationId = parsed.optString("conversation_id").takeIf { it.isNotBlank() } ?: "default"
                    val key = conversationTaskKey(projectId, conversationId)
                    runningConversationTasks[key] = ConversationTaskState(
                        traceId = traceId,
                        projectId = projectId,
                        conversationId = conversationId,
                        payload = payload,
                        isDevelopment = item.optBoolean("is_development", true),
                        pendingReconnect = true,
                        startedAt = savedAt
                    )
                    runningTraceToConversation[traceId] = key
                }
            }
        } else {
            val payload = prefs.getString(PREF_PENDING_WORK_PAYLOAD, null)?.takeIf { it.isNotBlank() }
                ?: return
            val savedAt = prefs.getLong(PREF_PENDING_WORK_TIME, 0L)
            val tooOld = savedAt <= 0L || now - savedAt > PENDING_WORK_TTL_MS
            if (tooOld) {
                clearPersistedActiveWork()
                return
            }
            val parsed = runCatching { JSONObject(payload) }.getOrNull() ?: return
            val traceId = parsed.optString("trace_id").takeIf { it.isNotBlank() } ?: return
            val projectId = parsed.optString("project_id").takeIf { it.isNotBlank() } ?: activeProject().id
            val conversationId = parsed.optString("conversation_id").takeIf { it.isNotBlank() } ?: activeConversation().id
            val key = conversationTaskKey(projectId, conversationId)
            runningConversationTasks[key] = ConversationTaskState(
                traceId = traceId,
                projectId = projectId,
                conversationId = conversationId,
                payload = payload,
                isDevelopment = prefs.getBoolean(PREF_PENDING_WORK_IS_DEVELOPMENT, true),
                pendingReconnect = true,
                startedAt = savedAt
            )
            runningTraceToConversation[traceId] = key
        }

        refreshActiveTaskState()
        reconnectAttempts = 0
        if (activeRequestIsDevelopment) {
            updateStage("后台继续", "任务仍在服务器继续处理，连接恢复后会同步最新进度。")
        } else {
            updateProjectViews("上一条回复仍在处理，连接恢复后会同步结果。")
        }
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
        when (intent.action) {
            TaskWorkService.ACTION_EVENT -> {
                backendConnected = intent.getBooleanExtra(TaskWorkService.EXTRA_CONNECTED, backendConnected)
                val traceId = intent.getStringExtra(TaskWorkService.EXTRA_TRACE_ID)?.takeIf { it.isNotBlank() }
                val projectId = intent.getStringExtra(TaskWorkService.EXTRA_PROJECT_ID)?.takeIf { it.isNotBlank() }
                val conversationId = intent.getStringExtra(TaskWorkService.EXTRA_CONVERSATION_ID)?.takeIf { it.isNotBlank() }
                val isDevelopment = if (intent.hasExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT)) {
                    intent.getBooleanExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, true)
                } else {
                    null
                }
                when (intent.getStringExtra(TaskWorkService.EXTRA_KIND)) {
                    "connected" -> {
                        reconnectAttempts = 0
                        updateFirstConversationStatus("已连接 · 点击进入开发会话")
                        val task = updateConversationTaskFromService(
                            traceId,
                            projectId,
                            conversationId,
                            isDevelopment,
                            pendingReconnect = false
                        )
                        if (task != null && activeConversationTask()?.traceId == task.traceId) {
                            recordEvidence("connection", "连接已恢复，后台任务继续运行")
                        }
                        setSendEnabled(!isActiveConversationWorking())
                    }
                    "disconnected" -> {
                        backendConnected = false
                        val task = updateConversationTaskFromService(
                            traceId,
                            projectId,
                            conversationId,
                            isDevelopment,
                            pendingReconnect = true
                        )
                        if (task != null && activeConversationTask()?.traceId == task.traceId) {
                            handleActiveWorkDisconnected(task)
                        } else {
                            updateIdleReadyStatus()
                            setSendEnabled(!isActiveConversationWorking())
                        }
                    }
                    "message" -> {
                        intent.getStringExtra(TaskWorkService.EXTRA_RAW_MESSAGE)?.let { raw ->
                            traceId?.let { taskResponseTokens.remove(it) }
                            appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
                        }
                    }
                    "paused" -> {
                        removeConversationTask(traceId, projectId, conversationId)
                        updateIdleReadyStatus()
                        setSendEnabled(!isActiveConversationWorking())
                    }
                }
            }
            TaskWorkService.ACTION_STATE -> {
                backendConnected = intent.getBooleanExtra(TaskWorkService.EXTRA_CONNECTED, backendConnected)
                val serviceWaiting = intent.getBooleanExtra(TaskWorkService.EXTRA_WAITING, waitingForReply)
                syncActiveTasksFromServiceState(intent.getStringExtra(TaskWorkService.EXTRA_ACTIVE_TASKS))
                if (!serviceWaiting) {
                    runningConversationTasks.clear()
                    runningTraceToConversation.clear()
                    taskResponseTokens.clear()
                    refreshActiveTaskState()
                    updateIdleReadyStatus()
                }
                setSendEnabled(!isActiveConversationWorking())
            }
        }
    }

    private fun syncActiveTasksFromServiceState(activeTasksJson: String?) {
        if (activeTasksJson.isNullOrBlank()) return
        val array = runCatching { JSONArray(activeTasksJson) }.getOrNull() ?: return
        for (index in 0 until array.length()) {
            val item = array.optJSONObject(index) ?: continue
            val traceId = item.optString("trace_id").takeIf { it.isNotBlank() } ?: continue
            val projectId = item.optString("project_id").takeIf { it.isNotBlank() } ?: continue
            val conversationId = item.optString("conversation_id").takeIf { it.isNotBlank() } ?: continue
            val key = conversationTaskKey(projectId, conversationId)
            val existing = runningConversationTasks[key] ?: continue
            runningTraceToConversation[traceId] = key
            existing.pendingReconnect = false
            existing.isDevelopment = item.optBoolean("is_development", existing.isDevelopment)
        }
        refreshActiveTaskState()
    }

    private fun appendTaskMessage(
        raw: String,
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?
    ) {
        val parsed = runCatching { JSONObject(raw) }.getOrNull()
        val type = parsed?.optString("type")?.takeIf { it.isNotBlank() }
        val key = when {
            !traceId.isNullOrBlank() && runningTraceToConversation.containsKey(traceId) ->
                runningTraceToConversation[traceId]
            !projectId.isNullOrBlank() && !conversationId.isNullOrBlank() ->
                conversationTaskKey(projectId, conversationId)
            else -> activeConversationTaskKey()
        }
        val isActiveTarget = key == activeConversationTaskKey()
        if (isActiveTarget) {
            appendMessage(raw)
        } else {
            val effectiveIsDevelopment = isDevelopment
                ?: key?.let { runningConversationTasks[it]?.isDevelopment }
                ?: false
            appendBackgroundTaskMessage(raw, key, effectiveIsDevelopment)
        }
        if (type == "done" || type == "error") {
            removeConversationTask(traceId, projectId, conversationId)
            persistActiveWork()
        } else {
            updateConversationTaskFromService(traceId, projectId, conversationId, isDevelopment, pendingReconnect = false)
        }
    }

    private fun appendBackgroundTaskMessage(raw: String, key: String?, isDevelopment: Boolean) {
        val location = key?.let { findConversationLocationByKey(it) } ?: return
        val parsed = runCatching { JSONObject(raw) }.getOrNull() ?: return
        val type = parsed.optString("type").takeIf { it.isNotBlank() } ?: return
        if (type == "app_update_available") {
            AppUpdateManager(this).realtimeCheck(parsed.optInt("versionCode", 0))
            return
        }
        val message = when (type) {
            "done" -> {
                val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务已完成。"
                val apkUrl = parsed.optString("apk_url").takeIf { it.isNotBlank() && it != "null" }
                val imageUrl = parsed.optString("image_url").takeIf { it.isNotBlank() && it != "null" }
                ChatMessage(
                    "ai",
                    finalReplyMessage(content, if (isDevelopment) apkUrl else null, imageUrl, isDevelopment)
                )
            }
            "error" -> ChatMessage(
                "error",
                friendlyErrorMessage(parsed.optString("message").takeIf { it.isNotBlank() } ?: "任务失败。")
            )
            "progress" -> {
                val content = parsed.optString("message").takeIf { it.isNotBlank() } ?: return
                val narrative = CodexProgressNarrative.fromWorkflowProgress(content)
                if (narrative == null && !shouldShowProgressBubble(content)) return
                narrative?.message
                    ?: ChatMessage("ai-progress", workflowProgressMessage(content))
            }
            else -> return
        }
        appendMessageToConversation(location.first, location.second, message)
    }

    private fun findConversationLocationByKey(key: String): Pair<Int, Int>? {
        projects.forEachIndexed { projectIndex, project ->
            project.conversations.forEachIndexed { conversationIndex, conversation ->
                if (conversationTaskKey(project.id, conversation.id) == key) {
                    return projectIndex to conversationIndex
                }
            }
        }
        return null
    }

    private fun appendMessageToConversation(
        projectIndex: Int,
        conversationIndex: Int,
        message: ChatMessage
    ) {
        val project = projects.getOrNull(projectIndex) ?: return
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        if (message.role in workflowTerminalRoles) {
            closeStaleWorkflowMessages(conversation.messages)
        }
        conversation.messages.add(message)
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        if (!conversation.ended) {
            conversation.subtitle = summarize(message.content, 30)
            project.subtitle = summarize(message.content, 34)
        }
        saveProjects()
        renderConversationList()
        if (projectIndex == activeProjectIndex && conversationIndex == activeConversationIndex) {
            chatAdapter.notifyDataSetChanged()
            binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
        }
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
        val intent = Intent(this, TaskWorkService::class.java).apply {
            this.action = action
            payload?.let { putExtra(TaskWorkService.EXTRA_PAYLOAD, it) }
            putExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, isDevelopment)
            traceId?.let { putExtra(TaskWorkService.EXTRA_TRACE_ID, it) }
        }
        return runCatching {
            if (action == TaskWorkService.ACTION_START_WORK || action == TaskWorkService.ACTION_RESUME_PENDING) {
                ContextCompat.startForegroundService(this, intent)
            } else {
                startService(intent)
            }
        }.recoverCatching {
            startService(intent)
        }.isSuccess
    }

    private fun setTaskAppForeground(foreground: Boolean) {
        prefs.edit().putBoolean(TaskWorkService.PREF_APP_IN_FOREGROUND, foreground).apply()
    }

    private fun drainQueuedTaskEvents() {
        val queued = prefs.getString(TaskWorkService.PREF_QUEUED_TASK_EVENTS, null)?.takeIf { it.isNotBlank() }
            ?: return
        prefs.edit().remove(TaskWorkService.PREF_QUEUED_TASK_EVENTS).apply()
        runCatching {
            val array = JSONArray(queued)
            for (index in 0 until array.length()) {
                val item = array.opt(index)
                if (item is JSONObject) {
                    val raw = item.optString("raw").takeIf { it.isNotBlank() }
                    if (raw != null) {
                        appendTaskMessage(
                            raw,
                            item.optString("trace_id").takeIf { it.isNotBlank() },
                            item.optString("project_id").takeIf { it.isNotBlank() },
                            item.optString("conversation_id").takeIf { it.isNotBlank() },
                            if (item.has("is_development")) item.optBoolean("is_development", true) else null
                        )
                    }
                } else {
                    array.optString(index).takeIf { it.isNotBlank() }?.let { appendMessage(it) }
                }
            }
        }
    }

    private fun normalizeProject(project: AppProject) {
        if (project.conversations.isEmpty()) project.conversations.add(defaultAppConversation())
        project.conversations.forEach {
            if (it.messages.isEmpty()) it.messages.add(welcomeChatMessage())
            it.messages.forEach { message -> message.evidenceWorking = false }
            compactCliTranscriptMessages(it.messages)
            sanitizeExistingCliLogMessages(it.messages)
            sanitizeExistingUserVisibleMessages(it.messages)
            removeLeakedAndRoutineWorkflowMessages(it.messages)
            compactWorkflowStatusMessages(it.messages)
            closeStaleWorkflowMessages(it.messages)
        }
        if (project.stage.isBlank()) project.stage = "待提交需求"
        if (project.subtitle.isBlank()) project.subtitle = "点击进入会话"
        compactCliProjectEvents(project.events)
        project.activeConversationIndex = project.activeConversationIndex.coerceIn(0, project.conversations.lastIndex)
    }

    private fun updateFirstConversationStatus(text: String) {
        if (conversations.isEmpty()) conversations.add(defaultAppConversation())
        if (conversations[0].ended) return
        conversations[0].subtitle = text
        conversations[0].updatedAt = System.currentTimeMillis()
        saveConversations()
        renderConversationList()
    }

    private fun updateIdleReadyStatus() {
        if (runningConversationTasks.isEmpty()) {
            updateFirstConversationStatus("已就绪 · 点击进入开发会话")
        }
    }

    private fun updateActiveConversationPreview(message: ChatMessage) {
        val conversation = activeConversation()
        val project = activeProject()
        conversation.updatedAt = System.currentTimeMillis()
        project.updatedAt = conversation.updatedAt
        when (message.role) {
            "user" -> {
                conversation.subtitle = summarize(message.content, 30)
                project.subtitle = summarize(message.content, 34)
                if (conversation.title.startsWith("新会话")) {
                    conversation.title = summarize(message.content, 12)
                    binding.topTitleText.text = conversation.title
                }
            }
            "ai", "ai-intent", "ai-working", "ai-progress", "ai-tool", "ai-complete", "ai-stopped", "error" -> {
                if (!conversation.ended) {
                    conversation.subtitle = summarize(message.content, 30)
                    project.subtitle = summarize(message.content, 34)
                }
            }
        }
        saveConversations()
        renderConversationList()
        if (binding.projectPage.visibility == View.VISIBLE) renderProjectList()
    }

    private fun renderConversationList() {
        if (conversations.isEmpty()) return
        val listVisible = binding.conversationPage.visibility == View.VISIBLE && binding.chatPage.visibility != View.VISIBLE
        if (listVisible) {
            binding.topTitleText.text = compactProjectTitle()
        }

        val first = conversations[0]
        binding.projectStatusText.text = first.title
        binding.statusText.text = first.subtitle
        binding.statusText.setTextColor(conversationSubtitleColor(first.subtitle))
        binding.conversationTimeText.text = timeFormatter.format(Date(first.updatedAt))
        updateConversationRowShimmer(binding.conversationItem, listVisible && isConversationWorking(0), true)

        while (binding.conversationPage.childCount > 1) {
            binding.conversationPage.removeViewAt(1)
        }
        for (index in 1 until conversations.size) {
            binding.conversationPage.addView(createConversationDivider())
            binding.conversationPage.addView(createConversationRow(index, conversations[index], listVisible))
        }
    }

    private fun renderProjectList() {
        val container = binding.projectContentLayout
        container.removeAllViews()

        container.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(52)
            ).apply {
                bottomMargin = dp(8)
            }
            setBackgroundColor(Color.parseColor("#202020"))
            gravity = Gravity.CENTER_VERTICAL
            text = "＋ 新建项目"
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 15f
            setPadding(dp(20), 0, dp(20), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showCreateProjectDialog() }
        })

        projects.forEachIndexed { index, project ->
            container.addView(createProjectRow(index, project))
        }
    }

    private fun createProjectRow(index: Int, project: AppProject): View {
        val wrapper = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(76)
            ).apply {
                topMargin = if (index == 0) 0 else 1
            }
        }

        val row = LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.parseColor(if (index == activeProjectIndex) "#292929" else "#202020"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(index) }
            setOnLongClickListener {
                showProjectActions(index)
                true
            }
        }

        row.addView(createAvatarView(project.title, 44, 18f))

        val middle = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(12)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = project.title
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 16f
        })
        middle.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(5)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = "${project.conversations.size} 个会话 · ${project.stage}"
            setTextColor(Color.parseColor("#A9A9A9"))
            textSize = 13f
        })
        row.addView(middle)

        row.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP
                marginStart = dp(8)
                topMargin = dp(17)
            }
            includeFontPadding = false
            text = timeFormatter.format(Date(project.updatedAt))
            setTextColor(Color.parseColor("#C4C4C4"))
            textSize = 13f
        })
        wrapper.addView(row)

        if (index == activeProjectIndex) {
            wrapper.addView(View(this).apply {
                layoutParams = FrameLayout.LayoutParams(dp(8), dp(8)).apply {
                    gravity = Gravity.START or Gravity.TOP
                    leftMargin = dp(10)
                    topMargin = dp(10)
                }
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor("#FF4D4F"))
                }
            })
        }

        return wrapper
    }

    private fun showProjectActions(index: Int) {
        if (index !in projects.indices) return
        val project = projects[index]
        val actions = if (projects.size <= 1) {
            arrayOf("编辑项目名称", "Git 仓库")
        } else {
            arrayOf("编辑项目名称", "Git 仓库", "删除项目")
        }

        AlertDialog.Builder(this)
            .setTitle(project.title)
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "编辑项目名称" -> showRenameProjectDialog(index)
                    "Git 仓库" -> {
                        openProject(index)
                        showGitProjectDialog()
                    }
                    "删除项目" -> confirmDeleteProject(index)
                }
            }
            .show()
    }

    private fun showRenameProjectDialog(index: Int) {
        if (index !in projects.indices) return
        val project = projects[index]
        val input = titleEditText(project.title)
        val dialog = AlertDialog.Builder(this)
            .setTitle("编辑项目名称")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                project.title = summarize(title, 24)
                project.updatedAt = System.currentTimeMillis()
                saveProjects()
                renderProjectList()
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    private fun confirmDeleteProject(index: Int) {
        if (index !in projects.indices || projects.size <= 1) return
        AlertDialog.Builder(this)
            .setTitle("删除项目")
            .setMessage("删除后这个项目下的会话和进度记录会从本机移除。")
            .setNegativeButton("取消", null)
            .setPositiveButton("删除") { _, _ -> deleteProject(index) }
            .show()
    }

    private fun deleteProject(index: Int) {
        if (index !in projects.indices || projects.size <= 1) return
        projects.removeAt(index)
        activeProjectIndex = activeProjectIndex.coerceAtMost(projects.lastIndex)
        activeConversationIndex = activeConversationIndex.coerceIn(0, conversations.lastIndex)
        saveProjects()
        renderProjectList()
    }

    private fun createConversationRow(index: Int, conversation: AppConversation, listVisible: Boolean): View {
        val row = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(66)
            )
            setBackgroundColor(Color.parseColor("#242424"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openConversation(index) }
            setOnLongClickListener {
                showConversationActions(index)
                true
            }
        }

        row.addView(createAvatarView(conversation.title, 44, 17f))

        val middle = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(10)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = conversation.title
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 16f
        })
        middle.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(4)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = conversation.subtitle
            setTextColor(conversationSubtitleColor(conversation.subtitle))
            textSize = 13f
        })
        row.addView(middle)

        row.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP
                marginStart = dp(7)
                topMargin = dp(16)
            }
            includeFontPadding = false
            text = timeFormatter.format(Date(conversation.updatedAt))
            setTextColor(Color.parseColor("#C4C4C4"))
            textSize = 12f
        })
        updateConversationRowShimmer(row, listVisible && isConversationWorking(index), false)
        return row
    }

    private fun updateConversationRowShimmer(row: View, active: Boolean, homeRow: Boolean) {
        if (active) {
            startConversationRowShimmer(row, homeRow)
        } else {
            stopConversationRowShimmer(row, homeRow)
        }
    }

    private fun startConversationRowShimmer(row: View, homeRow: Boolean) {
        if (homeRow && conversationHomeRowAnimator?.isRunning == true) {
            return
        }
        if (homeRow) {
            conversationHomeRowAnimator?.cancel()
        }

        val baseColor = Color.parseColor("#242424")
        val highlightColor = Color.parseColor("#303030")
        row.setBackgroundColor(baseColor)

        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1350L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener { valueAnimator ->
                val fraction = valueAnimator.animatedFraction
                val pulse = sin(Math.PI * fraction).toFloat()
                row.setBackgroundColor(blendColor(baseColor, highlightColor, pulse))
            }
        }

        if (homeRow) {
            conversationHomeRowAnimator = animator
        } else {
            row.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
                override fun onViewAttachedToWindow(v: View) = Unit
                override fun onViewDetachedFromWindow(v: View) {
                    animator.cancel()
                }
            })
        }
        animator.start()
    }

    private fun stopConversationRowShimmer(row: View, homeRow: Boolean) {
        if (homeRow) {
            conversationHomeRowAnimator?.cancel()
            conversationHomeRowAnimator = null
        }
        row.setBackgroundColor(Color.parseColor("#242424"))
    }

    private fun isConversationWorking(index: Int): Boolean {
        if (index !in conversations.indices || conversations[index].ended) return false
        return runningConversationTasks.containsKey(
            conversationTaskKey(activeProject().id, conversations[index].id)
        )
    }

    private fun createConversationDivider(): View {
        return View(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                marginStart = dp(68)
            }
            setBackgroundColor(Color.parseColor("#343434"))
        }
    }

    private fun createAvatarView(title: String, sizeDp: Int, textSizeSp: Float): View {
        val size = dp(sizeDp)
        if (title.startsWith(getString(R.string.app_name))) {
            return ImageView(this).apply {
                layoutParams = LinearLayout.LayoutParams(size, size)
                contentDescription = getString(R.string.app_name)
                scaleType = ImageView.ScaleType.FIT_CENTER
                setImageResource(R.drawable.ic_app_brand)
            }
        }

        return TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            setBackgroundResource(R.drawable.bg_mock_avatar)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = avatarText(title)
            setTextColor(Color.parseColor("#333333"))
            textSize = textSizeSp
            setTypeface(typeface, android.graphics.Typeface.BOLD)
        }
    }

    private fun selectableForeground() = runCatching {
        val outValue = TypedValue()
        theme.resolveAttribute(android.R.attr.selectableItemBackground, outValue, true)
        getDrawable(outValue.resourceId)
    }.getOrNull()

    private fun View.detachFromParent() {
        (parent as? ViewGroup)?.removeView(this)
    }

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }

    private fun setupQuickActions() {
        binding.quickPlanButton.setOnClickListener {
            fillPlanPrompt()
        }
        binding.quickContinueButton.setOnClickListener {
            sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。")
        }
        binding.quickBuildButton.setOnClickListener {
            sendQuickCommand("请编译当前项目并生成 APK 下载链接。")
        }
        binding.quickHistoryButton.setOnClickListener { showProjectRecordDialog() }
        binding.quickSettingsButton.setOnClickListener { openSettings() }

        binding.projectContinueButton.setOnClickListener {
            sendQuickCommand("请继续当前项目的开发，并先说明下一步要做什么。")
        }
        binding.projectBuildButton.setOnClickListener {
            sendQuickCommand("请打包当前项目，生成可以下载安装到手机的 APK。")
        }
        binding.projectRecordButton.setOnClickListener { showProjectRecordDialog() }
        binding.projectGitButton.setOnClickListener { showGitProjectDialog() }
        binding.projectSettingsButton.setOnClickListener { openSettings() }
        binding.profileSettingsButton.setOnClickListener { openSettings() }
        binding.profileCheckUpdateButton.setOnClickListener {
            AppUpdateManager(this).manualCheck()
        }
        binding.profileShareButton.setOnClickListener { showPromotionDialog() }
        binding.profileImportGuestButton.setOnClickListener { showGuestImportDialog() }
        binding.profileLoginButton.setOnClickListener {
            startActivity(Intent(this, LoginActivity::class.java))
        }
        binding.profileLogoutButton.setOnClickListener { confirmLogout() }
        refreshAccountUi()
        binding.profileVersionText.text =
            "${localAppVersionLine()}\n服务器版本读取中..."
        refreshServerVersion()
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
        Thread {
            val info = fetchServerVersionInfo(http, serverVersionUrl)
            val serverLine = info?.let { serverVersionLine(it) } ?: "服务器版本暂不可用"
            runOnUiThread {
                if (::binding.isInitialized) {
                    binding.profileVersionText.text = "${localAppVersionLine()}\n$serverLine"
                }
            }
        }.start()
    }

    private fun showMoreActions() {
        showChatActionPopup(binding.moreButton)
    }

    private fun showHomeActionPopup(anchor: View, tab: TextView) {
        val actions = if (tab == binding.tabProject) {
            listOf(
                TopAction("新建项目", R.drawable.ic_popup_project) { showCreateProjectDialog() },
                TopAction("项目记录", R.drawable.ic_popup_history) { showProjectRecordDialog() },
                TopAction("Git 仓库", R.drawable.ic_popup_settings) { showGitProjectDialog() },
                TopAction("打包 APK", R.drawable.ic_popup_build) { sendQuickCommand("请打包当前项目，生成可以下载安装到手机的 APK。") },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        } else {
            listOf(
                TopAction("新建会话", R.drawable.ic_popup_chat) { showCreateConversationDialog() },
                TopAction("新建项目", R.drawable.ic_popup_project) { showCreateProjectDialog() },
                TopAction("继续开发", R.drawable.ic_popup_plan) { sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。") },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        }
        showTopActionPopup(anchor, actions)
    }

    private fun showChatActionPopup(anchor: View) {
        showTopActionPopup(
            anchor,
            listOf(
                TopAction("需求规划", R.drawable.ic_popup_plan) { fillPlanPrompt() },
                TopAction("继续开发", R.drawable.ic_popup_chat) { sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。") },
                TopAction("打包 APK", R.drawable.ic_popup_build) { sendQuickCommand("请编译当前项目并生成 APK 下载链接。") },
                TopAction("项目记录", R.drawable.ic_popup_history) { showProjectRecordDialog() },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        )
    }

    private fun showTopActionPopup(anchor: View, actions: List<TopAction>) {
        actionPopup = popupRenderer().showTopActionPopup(anchor, actionPopup, actions)
    }

    private fun createPopupDivider(marginStart: Int = 0): View {
        return View(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                this.marginStart = marginStart
            }
            alpha = 0.55f
            setBackgroundColor(Color.parseColor(WECHAT_POPUP_DIVIDER_COLOR))
        }
    }

    private fun createPopupArrowView(
        pointsUp: Boolean = true,
        color: Int = Color.parseColor(WECHAT_POPUP_PANEL_COLOR)
    ): View {
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            this.color = color
            style = Paint.Style.FILL
        }
        return object : View(this) {
            override fun onDraw(canvas: Canvas) {
                super.onDraw(canvas)
                val path = Path().apply {
                    if (pointsUp) {
                        moveTo(width / 2f, 0f)
                        lineTo(width.toFloat(), height.toFloat())
                        lineTo(0f, height.toFloat())
                    } else {
                        moveTo(0f, 0f)
                        lineTo(width.toFloat(), 0f)
                        lineTo(width / 2f, height.toFloat())
                    }
                    close()
                }
                canvas.drawPath(path, paint)
            }
        }
    }

    private fun popupRenderer(): MainActionPopupRenderer {
        return MainActionPopupRenderer(
            activity = this,
            dp = ::dp,
            selectableForeground = ::selectableForeground
        )
    }

    private fun showProjectRecordDialog() {
        val recent = if (projectEvents.isEmpty()) {
            "暂无进度记录"
        } else {
            projectEvents.take(12).joinToString("\n")
        }
        AlertDialog.Builder(this)
            .setTitle("${currentProjectTitle} · 项目记录")
            .setMessage("阶段：$currentStage\n会话：${conversations.size} 个\n\n$recent")
            .setPositiveButton("知道了", null)
            .show()
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
            openUrl = ::openUrl,
            copyText = ::copyText
        ).showGitProjectDialog()
    }

    private fun maybePrewarmCodexSession(reason: String) {
        if (isActiveConversationWorking()) return
        val project = activeProject()
        val conversation = activeConversation()
        if (conversation.ended) return

        val key = "${project.id}:${conversation.id}"
        val now = System.currentTimeMillis()
        var shouldStart = false
        synchronized(prewarmLock) {
            val lastStartedAt = lastPrewarmAt[key] ?: 0L
            if (!prewarmingConversationKeys.contains(key) && now - lastStartedAt >= prewarmCooldownMs) {
                prewarmingConversationKeys.add(key)
                lastPrewarmAt[key] = now
                shouldStart = true
            }
        }
        if (!shouldStart) return

        val selectedAgent = modelActions().selectedAgentForRequest()
        val payload = JSONObject().apply {
            put("conversation_id", conversation.id)
            put("conversation_title", conversation.title)
            if (!selectedAgent.isNullOrBlank()) put("agent", selectedAgent)
        }
        val url = "$serverUrl/api/user/${urlPart(userId)}/projects/${urlPart(project.id)}/prewarm?title=${urlPart(project.title)}"
        DebugTraceStore.record(
            "ui_prewarm_start",
            mapOf("reason" to reason, "project_id" to project.id, "conversation_id" to conversation.id)
        )

        Thread {
            val startedAt = System.currentTimeMillis()
            try {
                val body = payload.toString().toRequestBody("application/json".toMediaType())
                http.newCall(
                    Request.Builder()
                        .url(url)
                        .post(body)
                        .build()
                ).execute().use { response ->
                    val responseBody = response.body?.string().orEmpty()
                    val status = runCatching {
                        JSONObject(responseBody).optString("status", "")
                    }.getOrDefault("")
                    DebugTraceStore.record(
                        if (response.isSuccessful) "ui_prewarm_done" else "ui_prewarm_failed",
                        mapOf(
                            "reason" to reason,
                            "project_id" to project.id,
                            "conversation_id" to conversation.id,
                            "http_code" to response.code,
                            "status" to status,
                            "elapsed_ms" to (System.currentTimeMillis() - startedAt)
                        )
                    )
                }
            } catch (e: Exception) {
                DebugTraceStore.record(
                    "ui_prewarm_failed",
                    mapOf(
                        "reason" to reason,
                        "project_id" to project.id,
                        "conversation_id" to conversation.id,
                        "error" to e.message
                    )
                )
            } finally {
                synchronized(prewarmLock) {
                    prewarmingConversationKeys.remove(key)
                }
            }
        }.start()
    }

    private fun copyText(label: String, text: String) {
        val clipboard = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
        Toast.makeText(this, "已复制", Toast.LENGTH_SHORT).show()
    }

    private fun openUrl(url: String) {
        runCatching {
            startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
        }.onFailure {
            Toast.makeText(this, "无法打开链接: ${it.message}", Toast.LENGTH_SHORT).show()
        }
    }

    private fun fillPlanPrompt() {
        binding.inputEdit.setText("我想开发一个 App，请先帮我拆解功能、页面和开发计划：")
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
    }

    private fun sendQuickCommand(text: String) {
        if (activeConversation().ended) {
            showCreateConversationDialog()
            return
        }
        showChat()
        binding.inputEdit.setText(text)
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
        sendMessage()
    }

    private fun openSettings() {
        startActivity(Intent(this, SettingsActivity::class.java))
    }

    private fun showMessageActions(anchor: View, message: ChatMessage) {
        val text = shareableMessageText(message)
        if (text.isBlank()) return
        showMessageActionPopup(anchor, message, text)
    }

    private fun showMessageActionPopup(anchor: View, message: ChatMessage, text: String) {
        val actions = listOf(
            TopAction("复制", R.drawable.ic_msg_copy) { shareActions().copyMessageText(text) },
            TopAction("转发", R.drawable.ic_msg_forward) { shareActions().forwardMessageText(text) },
            TopAction("收藏", R.drawable.ic_msg_favorite) { shareActions().toastMessageAction("已收藏") },
            TopAction("删除", R.drawable.ic_msg_delete) { deleteMessage(message) },
            TopAction("多选", R.drawable.ic_msg_multi) { shareActions().toastMessageAction("多选准备中") },
            TopAction("引用", R.drawable.ic_msg_quote) { quoteMessage(text) },
            TopAction("提醒", R.drawable.ic_msg_remind) { shareActions().toastMessageAction("提醒准备中") },
            TopAction("搜一搜", R.drawable.ic_msg_search) { shareActions().searchMessageText(text) },
            TopAction("从当前听", R.drawable.ic_msg_listen) { shareActions().toastMessageAction("从当前听准备中") }
        )
        actionPopup = popupRenderer().showMessageActionPopup(anchor, actionPopup, actions)
    }

    private fun deleteMessage(message: ChatMessage) {
        val index = activeConversation().messages.indexOf(message)
        if (index < 0) return
        activeConversation().messages.removeAt(index)
        chatAdapter.notifyItemRemoved(index)
        saveConversations()
        renderConversationList()
        Toast.makeText(this, "已删除", Toast.LENGTH_SHORT).show()
    }

    private fun quoteMessage(text: String) {
        showChat()
        binding.inputEdit.setText("> ${summarize(text, 40)}\n")
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
    }

    private fun showPromotionDialog() {
        shareActions().showPromotionDialog(apkDownloadUrl, apkDownloadPageUrl)
    }

    private fun shareActions(): MainShareActions {
        return MainShareActions(this, ::dp)
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
        foldedCliLogCount = 0
        foldedCliLogSamples.clear()
        foldedCliLogCategories.clear()
    }

    private fun recordEvidence(kind: String, detail: String) {
        if (!activeRequestIsDevelopment) return
        val clean = sanitizeEvidenceDetail(detail)
        if (clean.isBlank()) return

        currentEvidenceEntries.add(EvidenceEntry(kind, summarize(clean, 96)))
        while (currentEvidenceEntries.size > 40) {
            currentEvidenceEntries.removeAt(0)
        }
        attachEvidenceToLatestAi()
    }

    private fun attachEvidenceToLatestAi() {
        if (currentEvidenceEntries.isEmpty()) return
        val messages = activeConversation().messages
        val latestUserIndex = messages.indexOfLast { it.role == "user" }
        val index = messages.indices.lastOrNull { it > latestUserIndex && messages[it].role in assistantEvidenceRoles }
            ?: return

        applyEvidenceToMessage(messages[index], currentEvidenceEntries, working = true)
        chatAdapter.notifyMessageUpdated(index)
        saveConversations()
    }

    private fun finalizeEvidenceForLatestAssistant() {
        if (currentEvidenceEntries.isEmpty()) return
        val messages = activeConversation().messages
        val latestUserIndex = messages.indexOfLast { it.role == "user" }
        val index = messages.indices.lastOrNull { it > latestUserIndex && messages[it].role in assistantEvidenceRoles }
            ?: return

        applyEvidenceToMessage(messages[index], currentEvidenceEntries, working = false)
        currentEvidenceEntries.clear()
        chatAdapter.notifyMessageUpdated(index)
        saveConversations()
    }

    private fun aiMessageWithCurrentEvidence(
        content: String,
        attachments: List<ChatAttachment> = emptyList()
    ): ChatMessage {
        val message = ChatMessage("ai", content, attachments.takeIf { it.isNotEmpty() })
        if (currentEvidenceEntries.isNotEmpty()) {
            stopWorkingEvidenceForActiveConversation()
            applyEvidenceToMessage(message, currentEvidenceEntries, working = false)
            currentEvidenceEntries.clear()
        }
        return message
    }

    private fun applyEvidenceToMessage(message: ChatMessage, entries: List<EvidenceEntry>, working: Boolean) {
        message.evidenceTitle = evidenceTitle(entries)
        message.evidenceDetails = evidenceDetails(entries)
        message.evidenceWorking = working
    }

    private fun stopWorkingEvidenceForActiveConversation() {
        var changed = false
        activeConversation().messages.forEachIndexed { index, message ->
            if (message.evidenceWorking) {
                message.evidenceWorking = false
                chatAdapter.notifyMessageUpdated(index)
                changed = true
            }
        }
        if (changed) saveConversations()
    }

    private fun handleFoldedCliOutput(content: String) {
        foldedCliLogCount += 1
        val line = cleanCliOutputLine(content)
        val category = cliOutputCategory(line)
        foldedCliLogCategories[category] = (foldedCliLogCategories[category] ?: 0) + 1

        val hint = when {
            category == "编译打包" -> "正在编译或检查 APK。"
            category == "执行命令" -> "正在检查项目文件。"
            category == "模型回复" -> "开发助手正在整理下一步。"
            category == "环境提示" -> "服务器环境有提示，技术细节已收起。"
            else -> "后台正在处理项目。"
        }
        val stage = when (category) {
            "编译打包" -> "编译打包"
            "环境提示" -> currentStage
            else -> "开发实现"
        }
        updateStage(stage, hint)
        val surfaced = maybeAppendVisibleCliSignal(category, line)
        if (!surfaced && category != "模型回复") {
            recordEvidence(evidenceKindForCliCategory(category), line)
        }
    }

    private fun foldedCliLogSummary(): String {
        val mainWork = foldedCliLogCategories.entries.maxByOrNull { it.value }?.key
        val friendly = when (mainWork) {
            "编译打包" -> "正在编译 APK"
            "执行命令" -> "正在检查项目文件"
            "环境提示" -> "环境提示已归类"
            "模型回复" -> "正在整理下一步"
            else -> "后台正在处理项目"
        }
        return "后台开发日志已收起（${foldedCliLogCount} 条） · $friendly"
    }

    private fun removeLeakedAndRoutineWorkflowMessages(messages: MutableList<ChatMessage>) {
        messages.removeAll { message ->
            isLeakedPlatformPromptMessage(message.content) ||
                isTechnicalLeakMessage(message.content) ||
                (message.role in workflowHistoryStatusRoles && isRoutineWorkflowMessage(message.content))
        }
    }

    private fun compactWorkflowStatusMessages(messages: MutableList<ChatMessage>) {
        if (messages.none { it.role in workflowHistoryStatusRoles }) return

        val compacted = mutableListOf<ChatMessage>()
        var pendingStatus: ChatMessage? = null

        for (message in messages) {
            when {
                message.role in workflowHistoryStatusRoles -> {
                    pendingStatus = if (message.role == "ai-cli-log") {
                        ChatMessage("ai-cli-log", genericFoldedCliLogSummary())
                    } else {
                        message
                    }
                }
                message.role in workflowTerminalRoles -> {
                    pendingStatus = null
                    compacted.add(message)
                }
                else -> {
                    pendingStatus?.let(compacted::add)
                    pendingStatus = null
                    compacted.add(message)
                }
            }
        }

        pendingStatus?.let(compacted::add)
        messages.clear()
        messages.addAll(compacted)
    }

    private fun maybeAppendVisibleCliSignal(category: String, line: String): Boolean {
        if (!activeRequestIsDevelopment) return false
        val narrative = CodexProgressNarrative.fromCliOutput(category, line) ?: return false
        return appendProgressNarrative(narrative)
    }

    private fun maybeAppendWorkflowProgressNarrative(content: String): Boolean {
        if (!activeRequestIsDevelopment) return false
        val narrative = CodexProgressNarrative.fromWorkflowProgress(content) ?: return false
        return appendProgressNarrative(narrative)
    }

    private fun maybeAppendTaskEventNarrative(event: String, content: String): Boolean {
        if (!activeRequestIsDevelopment) return false
        val narrative = CodexProgressNarrative.fromTaskEvent(event, content) ?: return false
        return appendProgressNarrative(narrative)
    }

    private fun maybeAppendToolCallNarrative(tool: String): Boolean {
        if (!activeRequestIsDevelopment) return false
        val narrative = CodexProgressNarrative.fromToolCall(tool) ?: return false
        return appendProgressNarrative(narrative)
    }

    private fun appendProgressNarrative(narrative: CodexProgressNarrative.Narrative): Boolean {
        if (!emittedProgressSignals.add(narrative.key)) return false
        finalizeEvidenceForLatestAssistant()
        appendMessage(narrative.message)
        attachEvidenceToLatestAi()
        return true
    }

    private fun compactCliProjectEvents(events: MutableList<String>) {
        val cliCount = events.count { isCliProjectEvent(it) }
        if (cliCount == 0) return
        val compacted = events.filterNot { isCliProjectEvent(it) }.toMutableList()
        compacted.add(0, "${timeFormatter.format(Date())}  后台日志已归类：历史 ${cliCount} 条")
        while (compacted.size > 40) compacted.removeAt(compacted.size - 1)
        events.clear()
        events.addAll(compacted)
    }

    private fun closeStaleWorkflowMessages(messages: MutableList<ChatMessage>) {
        val lastRole = messages.lastOrNull()?.role ?: return
        if (lastRole !in staleWorkflowRoles) return
        messages.removeAt(messages.lastIndex)
    }

    private fun scheduleFirstServerResponseWatchdog(traceId: String, token: Int) {
        binding.root.postDelayed({
            if (taskResponseTokens[traceId] != token) return@postDelayed
            val task = runningTraceToConversation[traceId]?.let { runningConversationTasks[it] } ?: return@postDelayed
            task.pendingReconnect = true
            refreshActiveTaskState()
            if (task.isDevelopment && activeConversationTask()?.traceId == traceId) {
                updateStage(currentStage, "暂时没有收到服务器进度，正在自动恢复连接。")
                addProjectEvent("服务端暂未返回进度，自动恢复连接")
            }
            startTaskWorkService(TaskWorkService.ACTION_RESUME_PENDING, traceId = traceId)
        }, 20_000L)
    }

    private fun appendMessage(raw: String) {
        // 解析服务端推送的 JSON 消息
        try {
            val json = com.google.gson.JsonParser.parseString(raw).asJsonObject
            val type = jsonStringOrNull(json, "type") ?: return
            if (type == "app_update_available") {
                val remoteVersionCode = runCatching {
                    json.get("versionCode")?.asInt ?: 0
                }.getOrDefault(0)
                AppUpdateManager(this).realtimeCheck(remoteVersionCode)
                return
            }

            serverResponseToken += 1
            val msg = when (type) {
                "task_event" -> {
                    val event = jsonStringOrNull(json, "event").orEmpty()
                    val taskId = jsonStringOrNull(json, "task_id")
                    val content = jsonStringOrNull(json, "message").orEmpty()
                    handleTaskEvent(event, taskId, content)
                    if (maybeAppendTaskEventNarrative(event, content)) return
                    if (event == "accepted" && shouldShowProgressBubble(content)) {
                        ChatMessage("ai-progress", workflowProgressMessage(content))
                    } else {
                        return
                    }
                }
                "progress"    -> {
                    val content = jsonStringOrNull(json, "message") ?: ""
                    if (isCliOutputProgress(content)) {
                        handleFoldedCliOutput(content)
                        return
                    }
                    val surfaced = maybeAppendWorkflowProgressNarrative(content)
                    handleProgress(content, recordProgressEvidence = !surfaced)
                    if (surfaced) return
                    if (shouldShowProgressBubble(content)) {
                        ChatMessage("ai-progress", workflowProgressMessage(content))
                    } else {
                        return
                    }
                }
                "tool_call"   -> {
                    val tool = jsonStringOrNull(json, "tool") ?: "工具"
                    val args = json.get("args")?.takeIf { it.isJsonObject }?.asJsonObject
                    maybeAppendToolCallNarrative(tool)
                    handleToolCall(tool)
                    // 真实交互感：每次 tool_call 追加一条小灰字"动作卡片"气泡，
                    // 让用户看到 AI 正在读什么、改什么、执行什么命令。
                    val actionText = describeToolAction(tool, args)
                    val bubble = ChatMessage("ai-action", actionText)
                    appendMessage(bubble)
                    val msgs = activeConversation().messages
                    val idx = msgs.indices.lastOrNull { msgs[it].role == "ai-action" } ?: -1
                    if (idx >= 0) {
                        pendingToolActionBubbles.getOrPut(tool) { ArrayDeque() }.addLast(idx)
                    }
                    return
                }
                "tool_result" -> {
                    val tool = jsonStringOrNull(json, "tool") ?: "工具"
                    val result = jsonStringOrNull(json, "result").orEmpty()
                    val evidence = if (result.isBlank()) {
                        "完成：${toolLabel(tool)}"
                    } else {
                        "完成：${toolLabel(tool)}，${summarize(result, 80)}"
                    }
                    recordEvidence(toolEvidenceKind(tool), evidence)
                    // 把对应 ai-action 卡片标记为 ✓ 已完成
                    val queue = pendingToolActionBubbles[tool]
                    if (queue != null && queue.isNotEmpty()) {
                        val idx = queue.removeFirst()
                        val messages = activeConversation().messages
                        if (idx in messages.indices && messages[idx].role == "ai-action") {
                            val old = messages[idx]
                            val newContent = markToolActionDone(old.content)
                            if (newContent != old.content) {
                                messages[idx] = old.copy(content = newContent)
                                chatAdapter.notifyMessageUpdated(idx)
                                saveConversations()
                            }
                        }
                        if (queue.isEmpty()) pendingToolActionBubbles.remove(tool)
                    }
                    updateStage(currentStage, "${toolLabel(tool)} 已完成，正在判断下一步。")
                    addProjectEvent("工具完成：${toolLabel(tool)}")
                    return
                }
                "assistant_message" -> {
                    val text = jsonStringOrNull(json, "text").orEmpty().trim()
                    if (text.isBlank()) return
                    if (activeRequestIsDevelopment) {
                        addProjectEvent("AI 说明：${summarize(text, 36)}")
                    }
                    // AI 的中间发言：直接以 ai-intent 主白底气泡呈现，
                    // 不再被压缩成 200 字进度文案，给用户真正的"对话感"。
                    ChatMessage("ai-intent", text)
                }
                "done"        -> {
                    waitingForReply = false
                    setSendEnabled(true)
                    pendingRequestPayload = null
                    pendingReconnectForActiveWork = false
                    reconnectAttempts = 0
                    clearPersistedActiveWork()
                    val content = jsonStringOrNull(json, "message") ?: ""
                    val apkUrl  = jsonStringOrNull(json, "apk_url")
                    val imageUrl = jsonStringOrNull(json, "image_url")
                    val wasDevelopment = activeRequestIsDevelopment
                    if (wasDevelopment) {
                        updateStage("交付完成", if (apkUrl != null) "APK 已生成，可以下载安装测试。" else "任务已完成，可以继续提出修改。")
                        addProjectEvent(if (apkUrl != null) "生成 APK 下载链接" else "任务完成")
                        recordEvidence("result", if (apkUrl != null) "APK 已生成：$apkUrl" else "任务完成")
                    } else {
                        updateProjectViews("普通消息已回复，开发项目记录保持不变。")
                    }
                    activeRequestIsDevelopment = false
                    stopWorkingEvidenceForActiveConversation()
                    resetFoldedCliLog()
                    val visibleApkUrl = if (wasDevelopment) apkUrl else null
                    val finalMessage = aiMessageWithCurrentEvidence(
                        finalReplyMessage(content, visibleApkUrl, imageUrl, wasDevelopment),
                        chatAttachmentFromImageUrl(imageUrl)
                    )
                    finalMessage
                }
                "error" -> {
                    waitingForReply = false
                    setSendEnabled(true)
                    pendingRequestPayload = null
                    pendingReconnectForActiveWork = false
                    reconnectAttempts = 0
                    clearPersistedActiveWork()
                    val error = friendlyErrorMessage(jsonStringOrNull(json, "message") ?: "未知错误")
                    val wasDevelopment = activeRequestIsDevelopment
                    if (wasDevelopment) {
                        updateStage("需要处理", error)
                        addProjectEvent("发生错误：${summarize(error, 30)}")
                        recordEvidence("result", "发生错误：$error")
                    }
                    activeRequestIsDevelopment = false
                    stopWorkingEvidenceForActiveConversation()
                    currentEvidenceEntries.clear()
                    ChatMessage("error", error)
                }
                else -> return
            }
            appendMessage(msg)
        } catch (_: Exception) {
            waitingForReply = false
            setSendEnabled(true)
            pendingRequestPayload = null
            pendingReconnectForActiveWork = false
            reconnectAttempts = 0
            clearPersistedActiveWork()
            if (activeRequestIsDevelopment) {
                updateStage("需要处理", "服务端返回内容无法识别。")
                addProjectEvent("服务端返回异常")
            }
            appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("服务端返回内容无法识别。")))
            activeRequestIsDevelopment = false
            stopWorkingEvidenceForActiveConversation()
            currentEvidenceEntries.clear()
            appendMessage(ChatMessage("error", "服务端返回异常，无法解析。"))
        }
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
        val messages = activeConversation().messages
        val latestUserIndex = messages.indexOfLast { it.role == "user" }
        if (latestUserIndex < 0) return
        var removed = false
        for (index in messages.lastIndex downTo latestUserIndex + 1) {
            if (messages[index].role in staleWorkflowRoles) {
                messages.removeAt(index)
                removed = true
            }
        }
        if (removed) {
            chatAdapter.notifyDataSetChanged()
            saveConversations()
        }
    }

    private fun handleProgress(content: String, recordProgressEvidence: Boolean = true) {
        val lower = content.lowercase(Locale.CHINA)
        val facing = userFacingProgress(content)
        when {
            content.contains("进入队列") || content.contains("排队") ->
                updateStage("任务排队", facing)
            content.contains("通用项目工作流") ||
                content.contains("项目文档") ||
                content.contains("Git/权限") ||
                content.contains("项目自己的规则") ->
                updateStage("需求分析", facing)
            content.contains("未找到 APK") ||
                content.contains("未检测到 java") ||
                content.contains("未检测到 Android SDK") ->
                updateStage("需要处理", facing)
            content.contains("编译") ||
                content.contains("APK") ||
                content.contains("下载链接") ||
                lower.contains("gradle") ||
                lower.contains("assemble") ->
                updateStage("编译打包", facing)
            content.contains("CLI 输出") ||
                content.contains("写入") ||
                content.contains("读取") ||
                content.contains("修改") ||
                content.contains("工具") ->
                updateStage("开发实现", facing)
            content.contains("理解需求") ||
                content.contains("AI 代理") ||
                content.contains("CLI 工作区") ||
                content.contains("启动本地 CLI") ->
                updateStage("需求分析", facing)
            else ->
                updateStage("开发实现", facing)
        }
        if (recordProgressEvidence && !content.startsWith("CLI 仍在运行")) {
            recordEvidence("progress", userFacingProgress(content))
        }
        addProjectEvent("进度更新：${summarize(content, 30)}")
    }

    private fun handleTaskEvent(event: String, taskId: String?, content: String) {
        val suffix = taskId?.takeIf { it.isNotBlank() }?.let { "（任务 $it）" }.orEmpty()
        when (event) {
            "accepted" -> {
                updateStage("任务排队", if (content.isBlank()) "请求已进入任务队列。" else content)
                addProjectEvent("任务已受理$suffix")
            }
            "started" -> {
                updateStage("开发实现", if (content.isBlank()) "任务开始执行。" else content)
                addProjectEvent("任务开始执行$suffix")
            }
            "cancel_requested" -> {
                updateStage("需要处理", if (content.isBlank()) "已请求取消任务。" else content)
                addProjectEvent("任务取消请求已发送$suffix")
            }
            "canceled" -> {
                updateStage("需要处理", if (content.isBlank()) "任务已取消。" else content)
                addProjectEvent("任务已取消$suffix")
            }
            else -> {
                if (content.isNotBlank()) {
                    addProjectEvent("任务事件：${summarize(content, 30)}")
                }
            }
        }
    }

    private fun handleToolCall(tool: String) {
        recordEvidence(toolEvidenceKind(tool), "开始：${toolLabel(tool)}")
        when (tool) {
            "build_project" -> updateStage("编译打包", "正在编译项目并准备 APK。")
            "git_commit" -> updateStage("交付完成", "正在保存当前开发版本。")
            else -> updateStage("开发实现", "正在执行：${toolLabel(tool)}")
        }
        addProjectEvent("执行工具：${toolLabel(tool)}")
    }

    private fun updateStageHintShimmer() {
        if (isActiveConversationWorking() && binding.chatPage.visibility == View.VISIBLE) {
            startStageHintShimmer()
        } else {
            stopStageHintShimmer()
        }
    }

    private fun startStageHintShimmer() {
        stageHintShimmerToken += 1
        val token = stageHintShimmerToken
        stageHintAnimator?.cancel()
        stageHintAnimator = null

        val text = binding.stageHintText
        text.paint.shader = null
        text.alpha = 1f
        text.post {
            if (token != stageHintShimmerToken || !isActiveConversationWorking() || binding.chatPage.visibility != View.VISIBLE) {
                return@post
            }
            val width = text.width.coerceAtLeast(text.measuredWidth)
            if (width <= 0) return@post

            val shader = LinearGradient(
                0f,
                0f,
                width.toFloat(),
                0f,
                intArrayOf(
                    Color.parseColor("#9A9A9A"),
                    Color.parseColor("#CFCFCF"),
                    Color.parseColor("#F6F6F6"),
                    Color.parseColor("#D8D8D8"),
                    Color.parseColor("#9A9A9A")
                ),
                floatArrayOf(0f, 0.28f, 0.5f, 0.72f, 1f),
                Shader.TileMode.CLAMP
            )
            val matrix = Matrix()
            text.paint.shader = shader

            stageHintAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
                duration = 1350L
                repeatCount = ValueAnimator.INFINITE
                repeatMode = ValueAnimator.RESTART
                interpolator = LinearInterpolator()
                addUpdateListener { animator ->
                    val fraction = animator.animatedFraction
                    matrix.setTranslate(width * (fraction * 2f - 1f), 0f)
                    shader.setLocalMatrix(matrix)
                    text.alpha = 0.76f + 0.24f * sin(Math.PI * fraction).toFloat()
                    text.invalidate()
                }
                start()
            }
        }
    }

    private fun stopStageHintShimmer() {
        stageHintShimmerToken += 1
        stageHintAnimator?.cancel()
        stageHintAnimator = null
        if (::binding.isInitialized) {
            binding.stageHintText.paint.shader = null
            binding.stageHintText.alpha = 1f
            binding.stageHintText.setTextColor(Color.parseColor("#B8B8B8"))
            binding.stageHintText.invalidate()
        }
    }

    private fun updateStage(stage: String, hint: String) {
        currentStage = stage
        activeProject().subtitle = hint
        saveProjects()
        updateProjectViews(hint)
    }

    private fun updateProjectViews(hint: String) {
        binding.currentStageText.text = currentStage
        binding.projectStatusText.text = "一龙开发助手"
        binding.stageHintText.text = hint
        binding.progressTitleText.text = "开发进度：$currentStage"
        binding.conversationTimeText.text = timeFormatter.format(Date())
        binding.userInfoText.text = accountInfoText(this)

        val recent = projectEvents.take(5).joinToString("\n")
        binding.projectOverviewText.text = buildString {
            append("项目管理\n")
            append("项目：$currentProjectTitle\n")
            append("阶段：$currentStage")
            if (recent.isNotBlank()) {
                append("\n\n最近记录\n")
                append(recent)
            }
        }
        binding.projectHistoryText.text = if (projectEvents.isEmpty()) {
            "暂无进度记录"
        } else {
            projectEvents.joinToString("\n")
        }
        binding.projectWorkflowText.text = projectWorkflowCardText(currentStage)
        updateStageLines()
        renderConversationList()
        if (binding.projectPage.visibility == View.VISIBLE) {
            renderProjectList()
        }
        updateStageHintShimmer()
    }

    private fun updateStageLines() {
        val active = when (currentStage) {
            "任务排队" -> 1
            "需求分析" -> 1
            "开发实现" -> 2
            "编译打包" -> 3
            "交付完成" -> 4
            "需要处理" -> -1
            else -> 0
        }
        binding.stagePlanText.text = stageLine(1, active, "需求分析")
        binding.stageCodeText.text = stageLine(2, active, "开发实现")
        binding.stageBuildText.text = stageLine(3, active, "编译打包")
        binding.stageDeliverText.text = stageLine(4, active, "交付下载")
    }

    private fun compactProjectTitle(): String {
        return currentProjectTitle.trim().ifBlank { getString(R.string.app_name) }.take(6)
    }

    private companion object {
        const val PREF_PENDING_WORK_PAYLOAD = "pending_work_payload"
        const val PREF_PENDING_WORK_IS_DEVELOPMENT = "pending_work_is_development"
        const val PREF_PENDING_WORK_TIME = "pending_work_time"
        const val PENDING_WORK_TTL_MS = 24 * 60 * 60 * 1000L
        val assistantEvidenceRoles = setOf("ai", "ai-intent")
        val staleWorkflowRoles = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool")
        val workflowHistoryStatusRoles = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete")
        val workflowTerminalRoles = setOf("ai", "ai-intent", "error", "ai-stopped")
    }

    private fun addProjectEvent(text: String) {
        val line = "${timeFormatter.format(Date())}  $text"
        projectEvents.add(0, line)
        while (projectEvents.size > 40) projectEvents.removeAt(projectEvents.size - 1)
        activeProject().updatedAt = System.currentTimeMillis()
        saveProjects()
        updateProjectViews(binding.stageHintText.text.toString())
    }

    private fun saveProjectTitle() {
        saveProjects()
    }

    private fun updateProjectTitleFromRequest(text: String) {
        val project = activeProject()
        val shouldAutoName = project.title.startsWith("新项目") ||
            project.title == "一龙开发助手" ||
            project.title == "等待你的第一个开发需求"
        if (shouldAutoName) {
            currentProjectTitle = summarize(text, 24)
        }
        project.subtitle = summarize(text, 34)
    }

    private fun setSendEnabled(enabled: Boolean) {
        val conversationEnded = activeConversation().ended
        val canSend = enabled && !conversationEnded
        inputCanSend = canSend
        binding.inputEdit.isEnabled = !conversationEnded
        binding.inputEdit.hint = if (conversationEnded) "会话已结束，请新建会话继续" else "文本内容在此输入。"
        if (::inputModeButton.isInitialized) {
            inputModeButton.isEnabled = !conversationEnded
            inputModeButton.alpha = if (conversationEnded) 0.55f else 1f
        }
        if (::voiceHoldButton.isInitialized) {
            voiceHoldButton.isEnabled = !conversationEnded
            voiceHoldButton.alpha = if (conversationEnded) 0.55f else 1f
        }
        binding.modelButton.isEnabled = !conversationEnded
        if (::modelButtonShell.isInitialized) {
            modelButtonShell.isEnabled = !conversationEnded
            modelButtonShell.alpha = when {
                conversationEnded -> 0.55f
                ::inputComposerMotion.isInitialized && inputComposerMotion.isExpanded -> 1f
                else -> 0f
            }
        }
        updateSendButtonVisual()
        updateStageHintShimmer()
    }

    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        return false
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == R.id.action_settings) {
            openSettings()
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
        if (requestCode == speechPermissionRequest) {
            val granted = grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED
            Toast.makeText(
                this,
                if (granted) "已开启语音权限，请按住说话" else "需要麦克风权限才能语音转文字",
                Toast.LENGTH_SHORT
            ).show()
        } else if (requestCode == notificationPermissionRequest) {
            val granted = grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED
            if (!granted) {
                Toast.makeText(this, "需要通知权限才能显示任务完成和应用更新提醒", Toast.LENGTH_SHORT).show()
            }
        }
    }

    override fun onDestroy() {
        stopStageHintShimmer()
        conversationHomeRowAnimator?.cancel()
        conversationHomeRowAnimator = null
        speechRecognizer?.destroy()
        speechRecognizer = null
        if (taskWorkReceiverRegistered) {
            unregisterReceiver(taskWorkReceiver)
            taskWorkReceiverRegistered = false
        }
        super.onDestroy()
    }
}

