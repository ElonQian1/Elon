package com.elon.app

import android.Manifest
import android.annotation.SuppressLint
import android.animation.ValueAnimator
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
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
import android.os.Build
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
import android.widget.GridLayout
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
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.net.URLEncoder
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
    private var modelOptions: List<ModelOption> = emptyList()
    private var codexCliOnly = true
    private var selectedAgentName: String? = null
    private var currentModelLabel = "默认"
    private var stageHintAnimator: ValueAnimator? = null
    private var stageHintShimmerToken = 0
    private var conversationHomeRowAnimator: ValueAnimator? = null
    private var exitConfirmDialog: AlertDialog? = null
    private var actionPopup: PopupWindow? = null
    private lateinit var inputModeButton: ImageButton
    private lateinit var attachmentButton: ImageButton
    private lateinit var voiceHoldButton: TextView
    private lateinit var inputBarContainer: LinearLayout
    private lateinit var inputCenterContainer: FrameLayout
    private lateinit var attachmentPanel: LinearLayout
    private var attachmentPanelOpen = false
    private var attachmentIconAnimationToken = 0
    private var voiceMode = false
    private var inputCanSend = true
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
        setupTaskCompletionAlerts()

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
        clearCompletedTaskBadge()
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
        val text = binding.inputEdit.text.toString().trim()
        if (text.isEmpty()) return
        if (isActiveConversationWorking()) return
        if (activeConversation().ended) {
            appendMessage(ChatMessage("error", "这个会话已结束，请新建会话继续。"))
            return
        }
        val outgoingText = expandShortDevelopmentCommand(text)
        val target = currentSendTarget()
        if (pendingAttachments.isNotEmpty()) {
            uploadAttachmentsThenSend(text, outgoingText, target)
            return
        }
        startPreparedMessage(text, outgoingText, com.google.gson.JsonArray(), target)
    }

    private fun startPreparedMessage(
        visibleText: String,
        outgoingText: String,
        attachmentRefs: com.google.gson.JsonArray,
        target: SendTarget
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
            if (!codexCliOnly) {
                selectedAgentName?.let { addProperty("agent", it) }
            }
            if (attachmentRefs.size() > 0) add("attachments", attachmentRefs)
        }

        // 显示用户消息
        appendMessage(ChatMessage(role = "user", content = visibleText))
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

        inputBarContainer = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            minimumHeight = dp(62)
            gravity = Gravity.BOTTOM
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(4), dp(7), dp(4), dp(7))
        }

        // WeChat-scale circular controls: 44dp touch area with a 30dp visual icon.
        inputModeButton = ImageButton(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(44), dp(44))
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_voice_circle)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(7), dp(7), dp(7), dp(7))
            contentDescription = "切换语音输入"
            setOnClickListener { toggleVoiceMode() }
        }

        inputCenterContainer = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                dp(40),
                1f
            ).apply {
                marginStart = dp(4)
                marginEnd = dp(4)
            }
            setBackgroundResource(R.drawable.bg_input_pill)
            minimumHeight = dp(40)
        }

        inputEdit.apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ).apply {
                rightMargin = dp(64)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            hint = "描述你想开发的 App 功能"
            minLines = 1
            maxLines = 4
            setSingleLine(false)
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
            isVerticalScrollBarEnabled = false
            includeFontPadding = true
            setHorizontallyScrolling(false)
            setPadding(dp(14), dp(8), dp(8), dp(8))
            textSize = 14f
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

        modelButton.apply {
            layoutParams = FrameLayout.LayoutParams(dp(62), dp(32)).apply {
                gravity = Gravity.END or Gravity.CENTER_VERTICAL
                rightMargin = dp(4)
            }
            textSize = 12f
            setOnClickListener { showModelDialog() }
        }

        inputCenterContainer.addView(inputEdit)
        inputCenterContainer.addView(voiceHoldButton)
        inputCenterContainer.addView(modelButton)

        val rightControls = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(44), dp(44))
        }

        attachmentButton = ImageButton(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(44), dp(44), Gravity.CENTER)
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_add_circle_simple)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(7), dp(7), dp(7), dp(7))
            contentDescription = "展开更多输入功能"
            setOnClickListener { toggleAttachmentPanel() }
        }

        sendButton.apply {
            layoutParams = FrameLayout.LayoutParams(dp(58), dp(36), Gravity.CENTER)
            gravity = Gravity.CENTER
            includeFontPadding = false
            setOnClickListener { sendMessage() }
        }

        inputBarContainer.addView(inputModeButton)
        inputBarContainer.addView(inputCenterContainer)
        rightControls.addView(attachmentButton)
        rightControls.addView(sendButton)
        inputBarContainer.addView(rightControls)

        attachmentPanel = buildAttachmentPanel()
        root.addView(inputBarContainer)
        root.addView(attachmentPanel)

        inputEdit.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                updateSendButtonVisual()
                updateAdaptiveInputHeight()
            }
            override fun afterTextChanged(s: Editable?) = Unit
        })

        binding.chatList.setOnTouchListener { _, event ->
            if (event.action == MotionEvent.ACTION_DOWN && attachmentPanelOpen) {
                collapseAttachmentPanel()
            }
            false
        }
        binding.stageHintText.setOnClickListener { collapseAttachmentPanel() }
        applyVoiceMode()
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun updateAdaptiveInputHeight() {
        if (!::inputCenterContainer.isInitialized || !::inputBarContainer.isInitialized) return
        val inputEdit = binding.inputEdit
        inputEdit.post {
            if (!::inputCenterContainer.isInitialized || !::inputBarContainer.isInitialized) return@post
            val minHeight = dp(40)
            val maxHeight = dp(122)
            val rawLineCount = inputEdit.lineCount.coerceAtLeast(1)
            val desiredHeight = if (voiceMode) {
                minHeight
            } else {
                val multilineTopGuard = if (rawLineCount > 1) dp(8) else 0
                (rawLineCount.coerceAtMost(4) * inputEdit.lineHeight +
                    inputEdit.paddingTop +
                    inputEdit.paddingBottom +
                    multilineTopGuard).coerceIn(minHeight, maxHeight)
            }

            val centerParams = inputCenterContainer.layoutParams as LinearLayout.LayoutParams
            if (centerParams.height != desiredHeight) {
                centerParams.height = desiredHeight
                inputCenterContainer.layoutParams = centerParams
            }

            val multiline = !voiceMode && desiredHeight > minHeight
            inputEdit.gravity = (if (multiline) Gravity.TOP else Gravity.CENTER_VERTICAL) or Gravity.START
            inputEdit.isVerticalScrollBarEnabled = !voiceMode && rawLineCount > 4
        }
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
        appendAttachmentLabel(kind, attachment.displayName)
        Toast.makeText(this, "已添加${kind}：${attachment.displayName}", Toast.LENGTH_SHORT).show()
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
                attachments = attachments,
                target = target
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
                startPreparedMessage(visibleText, outgoingText, refs, target)
            }
        }.start()
    }

    private fun uploadAttachmentRefsOrNull(
        attachments: List<PendingAttachment>,
        target: SendTarget
    ): com.google.gson.JsonArray? {
        val array = com.google.gson.JsonArray()
        for (attachment in attachments) {
            if (!attachment.file.exists()) {
                runOnUiThread {
                    Toast.makeText(this, "附件已失效，请重新选择：${attachment.displayName}", Toast.LENGTH_SHORT).show()
                }
                return null
            }
            if (attachment.file.length() > MAX_ATTACHMENT_BYTES) {
                runOnUiThread {
                    Toast.makeText(this, "附件过大，请重新选择较小文件：${attachment.displayName}", Toast.LENGTH_SHORT).show()
                }
                return null
            }
            val url = buildString {
                append("$serverUrl/api/user/${urlPart(userId)}/projects/${urlPart(target.projectId)}/attachments")
                append("?title=${urlPart(target.projectTitle)}")
                append("&conversation_id=${urlPart(target.conversationId)}")
                append("&conversation_title=${urlPart(target.conversationTitle)}")
                append("&kind=${urlPart(attachment.kind)}")
                append("&display_name=${urlPart(attachment.displayName)}")
                append("&file_name=${urlPart(attachment.fileName)}")
                append("&mime_type=${urlPart(attachment.mimeType)}")
            }
            val mediaType = attachment.mimeType.toMediaTypeOrNull()
                ?: "application/octet-stream".toMediaType()
            val response = try {
                http.newCall(
                    Request.Builder()
                        .url(url)
                        .post(attachment.file.asRequestBody(mediaType))
                        .build()
                ).execute()
            } catch (e: Exception) {
                runOnUiThread {
                    Toast.makeText(this, "附件上传失败：${e.message}", Toast.LENGTH_LONG).show()
                }
                DebugTraceStore.record(
                    "ui_attachment_upload_failed",
                    mapOf("project_id" to target.projectId, "file" to attachment.displayName, "error" to e.message)
                )
                return null
            }
            response.use {
                val body = it.body?.string().orEmpty()
                if (!it.isSuccessful) {
                    runOnUiThread {
                        Toast.makeText(this, "附件上传失败：HTTP ${it.code}", Toast.LENGTH_LONG).show()
                    }
                    DebugTraceStore.record(
                        "ui_attachment_upload_failed",
                        mapOf("project_id" to target.projectId, "file" to attachment.displayName, "http_code" to it.code)
                    )
                    return null
                }
                val uploaded = runCatching { JSONObject(body).optJSONObject("attachment") }.getOrNull()
                if (uploaded == null) {
                    runOnUiThread {
                        Toast.makeText(this, "附件上传响应异常：${attachment.displayName}", Toast.LENGTH_LONG).show()
                    }
                    return null
                }
                array.add(com.google.gson.JsonObject().apply {
                    addProperty("kind", uploaded.optString("kind", attachment.kind))
                    addProperty("display_name", uploaded.optString("display_name", attachment.displayName))
                    addProperty("file_name", uploaded.optString("file_name", attachment.fileName))
                    addProperty("mime_type", uploaded.optString("mime_type", attachment.mimeType))
                    addProperty("path", uploaded.optString("path", ""))
                    uploaded.optString("url", "").takeIf { it.isNotBlank() }?.let {
                        addProperty("url", it)
                    }
                    addProperty("size_bytes", uploaded.optLong("size_bytes", attachment.file.length()))
                })
            }
        }
        return array
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
        hideKeyboard()
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
            binding.inputEdit.visibility = View.GONE
            binding.modelButton.visibility = View.GONE
            voiceHoldButton.visibility = View.VISIBLE
        } else {
            inputModeButton.setImageResource(R.drawable.ic_input_voice_circle)
            binding.inputEdit.visibility = View.VISIBLE
            binding.modelButton.visibility = View.VISIBLE
            voiceHoldButton.visibility = View.GONE
        }
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun updateSendButtonVisual() {
        if (!::binding.isInitialized) return
        val hasText = binding.inputEdit.text.toString().trim().isNotEmpty()
        val sendMode = hasText && !voiceMode
        val params = binding.sendButton.layoutParams as? FrameLayout.LayoutParams
        if (sendMode) {
            params?.width = dp(58)
            binding.sendButton.background = getDrawable(R.drawable.bg_send_button)
            binding.sendButton.text = "发送"
            binding.sendButton.setTextColor(Color.WHITE)
            binding.sendButton.textSize = 14f
            binding.sendButton.visibility = View.VISIBLE
            if (::attachmentButton.isInitialized) {
                attachmentButton.visibility = View.GONE
            }
        } else {
            binding.sendButton.visibility = View.GONE
            if (::attachmentButton.isInitialized) {
                attachmentButton.visibility = View.VISIBLE
            }
        }
            params?.height = dp(36)
        params?.gravity = Gravity.CENTER
        params?.let { binding.sendButton.layoutParams = it }
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
            tabs.forEach {
                it.setTextColor(Color.parseColor(if (it == tab) "#D0D0D0" else "#A5A5A5"))
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
        if (binding.chatPage.visibility == View.VISIBLE) {
            showConversationHome()
            return
        }
        showExitConfirmation()
    }

    private fun showConversationHome() {
        binding.tabChat.performClick()
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

    private fun showChat() {
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
        setSendEnabled(!isActiveConversationWorking())
        maybePrewarmCodexSession("show_chat")
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
            AppConversation(
                id = UUID.randomUUID().toString(),
                title = summarize(title, 24),
                subtitle = "点击进入开发会话",
                updatedAt = System.currentTimeMillis(),
                messages = mutableListOf(welcomeMessage())
            )
        )
        activeProject().updatedAt = System.currentTimeMillis()
        activeProject().subtitle = "${conversations.size} 个会话"
        saveConversations()
        renderConversationList()
    }

    private fun createProject(title: String) {
        projects.add(createProject(title, "新项目 · 点击进入会话"))
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
            conversations.add(createDefaultConversation())
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
        val label = cachedModelLabel() ?: return
        selectedAgentName = cachedModelAgentName()
        currentModelLabel = label
        updateModelButton()
    }

    private fun cacheModelSelection(agentName: String?, label: String) {
        prefs.edit().apply {
            if (agentName.isNullOrBlank()) remove(PREF_SELECTED_AGENT)
            else putString(PREF_SELECTED_AGENT, agentName)
            putString(PREF_SELECTED_MODEL_LABEL, label)
        }.apply()
    }

    private fun cachedModelAgentName(): String? {
        return prefs.getString(PREF_SELECTED_AGENT, null)
            ?.trim()
            ?.takeIf { it.isNotBlank() && it != "null" }
    }

    private fun cachedModelLabel(): String? {
        return prefs.getString(PREF_SELECTED_MODEL_LABEL, null)
            ?.trim()
            ?.takeIf { it.isNotBlank() && it != "null" }
    }

    private fun jsonStringOrNull(json: JSONObject, name: String): String? {
        if (!json.has(name) || json.isNull(name)) return null
        return json.optString(name, "")
            .trim()
            .takeIf { it.isNotBlank() && it != "null" }
    }

    private fun loadModelOptions(afterLoad: (() -> Unit)? = null) {
        Thread {
            try {
                val response = http.newCall(
                    Request.Builder()
                        .url("$serverUrl/api/user/$userId/agent")
                        .get()
                        .build()
                ).execute()
                val body = response.body?.string().orEmpty()
                if (!response.isSuccessful) error(body.ifBlank { "HTTP ${response.code}" })

                val json = JSONObject(body)
                val serverCodexCliOnly = json.optBoolean("codex_cli_only", false)
                if (serverCodexCliOnly) {
                    runOnUiThread {
                        codexCliOnly = true
                        modelOptions = listOf(ModelOption("Codex CLI", null))
                        selectedAgentName = null
                        currentModelLabel = "Codex CLI"
                        cacheModelSelection(null, currentModelLabel)
                        updateModelButton()
                        afterLoad?.invoke()
                    }
                    return@Thread
                }

                val config = json.optJSONObject("config") ?: JSONObject()
                val agents = json.optJSONArray("available_agents") ?: JSONArray()
                val options = mutableListOf(ModelOption("服务器默认", null))

                for (i in 0 until agents.length()) {
                    val item = agents.getJSONObject(i)
                    val name = item.optString("name", "")
                    val model = item.optString("model", "")
                    val provider = item.optString("provider", name)
                    val label = displayModelLabel(provider, model, item.optString("label", ""))
                    if (name.isNotBlank()) {
                        options.add(ModelOption(label, name))
                    }
                }

                val serverUseAgent = jsonStringOrNull(config, "use_agent")
                    ?.takeIf { configured -> options.any { it.agentName == configured } }
                val customModel = jsonStringOrNull(config, "model").orEmpty()
                val customBase = jsonStringOrNull(config, "api_base").orEmpty()
                val cachedAgent = cachedModelAgentName()
                val effectiveUseAgent = serverUseAgent ?: cachedAgent?.takeIf { cached ->
                    options.any { it.agentName == cached }
                }
                val hasCustomConfig = customModel.isNotBlank() || customBase.isNotBlank()
                val label = when {
                    hasCustomConfig -> "自定义模型"
                    effectiveUseAgent != null -> options.firstOrNull { it.agentName == effectiveUseAgent }?.label ?: effectiveUseAgent
                    else -> "服务器默认"
                }
                val shouldSyncCache = serverUseAgent != null ||
                    hasCustomConfig ||
                    cachedAgent == null ||
                    effectiveUseAgent == null

                runOnUiThread {
                    codexCliOnly = false
                    modelOptions = options
                    selectedAgentName = effectiveUseAgent
                    currentModelLabel = label
                    if (shouldSyncCache) {
                        cacheModelSelection(effectiveUseAgent, label)
                    }
                    updateModelButton()
                    afterLoad?.invoke()
                }
            } catch (e: Exception) {
                runOnUiThread {
                    Toast.makeText(this, "模型列表加载失败: ${e.message}", Toast.LENGTH_SHORT).show()
                    afterLoad?.invoke()
                }
            }
        }.start()
    }

    private fun showModelDialog() {
        if (codexCliOnly) {
            Toast.makeText(this, "当前已锁定使用 Codex CLI", Toast.LENGTH_SHORT).show()
            return
        }
        if (modelOptions.isEmpty()) {
            Toast.makeText(this, "正在加载模型列表...", Toast.LENGTH_SHORT).show()
            loadModelOptions { showModelDialog() }
            return
        }

        val labels = modelOptions.map { it.label }.toTypedArray()
        val checked = if (currentModelLabel.startsWith("自定义")) {
            -1
        } else {
            modelOptions.indexOfFirst { it.agentName == selectedAgentName }.coerceAtLeast(0)
        }
        AlertDialog.Builder(this)
            .setTitle("选择 AI 模型")
            .setSingleChoiceItems(labels, checked) { dialog, which ->
                saveModelSelection(modelOptions[which])
                dialog.dismiss()
            }
            .setNeutralButton("自定义") { _, _ -> openSettings() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun saveModelSelection(option: ModelOption) {
        if (codexCliOnly) {
            Toast.makeText(this, "当前已锁定使用 Codex CLI", Toast.LENGTH_SHORT).show()
            return
        }
        binding.modelButton.isEnabled = false
        Thread {
            try {
                val payload = JSONObject().apply {
                    put("use_agent", option.agentName ?: JSONObject.NULL)
                    put("api_base", JSONObject.NULL)
                    put("api_key", JSONObject.NULL)
                    put("model", JSONObject.NULL)
                }
                val body = payload.toString().toRequestBody("application/json".toMediaType())
                val response = http.newCall(
                    Request.Builder()
                        .url("$serverUrl/api/user/$userId/agent")
                        .put(body)
                        .build()
                ).execute()
                val responseBody = response.body?.string().orEmpty()
                if (!response.isSuccessful) error(responseBody.ifBlank { "HTTP ${response.code}" })

                runOnUiThread {
                    selectedAgentName = option.agentName
                    currentModelLabel = option.label
                    cacheModelSelection(option.agentName, option.label)
                    updateModelButton()
                    Toast.makeText(this, "已切换模型: ${option.label}", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                runOnUiThread {
                    Toast.makeText(this, "模型切换失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            } finally {
                runOnUiThread { binding.modelButton.isEnabled = true }
            }
        }.start()
    }

    private fun updateModelButton() {
        binding.modelButton.text = shortModelLabel(currentModelLabel)
        binding.modelButton.contentDescription = "选择模型：$currentModelLabel"
    }

    private fun openConversation(index: Int) {
        if (conversations.isEmpty()) conversations.add(createDefaultConversation())
        activeConversationIndex = index.coerceIn(0, conversations.lastIndex)
        chatAdapter = ChatAdapter(activeConversation().messages, ::pauseCurrentWork, ::showMessageActions)
        binding.chatList.adapter = chatAdapter
        showChat()
        if (chatAdapter.itemCount > 0) {
            binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
        }
    }

    private fun openProject(index: Int) {
        if (index !in projects.indices) return
        activeProjectIndex = index
        if (conversations.isEmpty()) conversations.add(createDefaultConversation())
        activeConversationIndex = activeConversationIndex.coerceIn(0, conversations.lastIndex)
        saveProjects()
        binding.tabChat.performClick()
    }

    private fun activeProject(): AppProject {
        if (projects.isEmpty()) {
            projects.add(createProject("一龙开发助手", "默认项目 · 点击进入会话"))
        }
        activeProjectIndex = activeProjectIndex.coerceIn(0, projects.lastIndex)
        val project = projects[activeProjectIndex]
        if (project.conversations.isEmpty()) project.conversations.add(createDefaultConversation())
        project.activeConversationIndex = project.activeConversationIndex.coerceIn(0, project.conversations.lastIndex)
        return project
    }

    private fun activeConversation(): AppConversation {
        if (conversations.isEmpty()) {
            conversations.add(createDefaultConversation())
        }
        activeConversationIndex = activeConversationIndex.coerceIn(0, conversations.lastIndex)
        return conversations[activeConversationIndex]
    }

    private fun createDefaultConversation(): AppConversation {
        return AppConversation(
            id = "default",
            title = "一龙开发助手",
            subtitle = "连接中...",
            updatedAt = System.currentTimeMillis(),
            messages = mutableListOf(welcomeMessage())
        )
    }

    private fun createProject(title: String, subtitle: String): AppProject {
        return AppProject(
            id = UUID.randomUUID().toString(),
            title = summarize(title, 24),
            subtitle = subtitle,
            updatedAt = System.currentTimeMillis(),
            conversations = mutableListOf(createDefaultConversation())
        )
    }

    private fun welcomeMessage(): ChatMessage {
        return ChatMessage(
            "ai",
            "你可以直接描述想开发的 App 功能；我会先说明我理解到的意图，再把需求分析、开发实现、编译打包和交付证据折叠同步给你。"
        )
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
            projects.add(loadLegacyProject())
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

    private fun setupTaskCompletionAlerts() {
        createTaskCompletionChannel()
        requestTaskNotificationPermissionIfNeeded()
    }

    private fun createTaskCompletionChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val channel = NotificationChannel(
            TASK_COMPLETE_CHANNEL_ID,
            "任务完成提醒",
            NotificationManager.IMPORTANCE_DEFAULT
        ).apply {
            description = "后台任务完成后显示桌面角标"
            setShowBadge(true)
        }
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun requestTaskNotificationPermissionIfNeeded() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        if (prefs.getBoolean(PREF_NOTIFICATION_PERMISSION_ASKED, false)) return
        prefs.edit().putBoolean(PREF_NOTIFICATION_PERMISSION_ASKED, true).apply()
        ActivityCompat.requestPermissions(
            this,
            arrayOf(Manifest.permission.POST_NOTIFICATIONS),
            notificationPermissionRequest
        )
    }

    private fun notifyBackgroundTaskCompleted(wasDevelopment: Boolean, apkUrl: String?) {
        val count = completedTaskBadgeCount() + 1
        prefs.edit().putInt(PREF_COMPLETED_TASK_BADGE_COUNT, count).apply()
        updateLauncherBadgeCount(count)
        showTaskCompletedNotification(count, wasDevelopment, apkUrl)
    }

    private fun completedTaskBadgeCount(): Int {
        return prefs.getInt(PREF_COMPLETED_TASK_BADGE_COUNT, 0).coerceAtLeast(0)
    }

    private fun clearCompletedTaskBadge() {
        prefs.edit().putInt(PREF_COMPLETED_TASK_BADGE_COUNT, 0).apply()
        NotificationManagerCompat.from(this).cancel(TASK_COMPLETE_NOTIFICATION_ID)
        updateLauncherBadgeCount(0)
    }

    private fun updateLauncherBadgeCount(count: Int) {
        updateHuaweiLauncherBadgeCount(count)
    }

    private fun shouldNotifyTaskCompletion(): Boolean {
        return !appInForeground || !hasWindowFocus()
    }

    private fun updateHuaweiLauncherBadgeCount(count: Int) {
        val badge = count.coerceAtLeast(0)
        val payload = Bundle().apply {
            putString("package", packageName)
            putString("class", MainActivity::class.java.name)
            putInt("badgenumber", badge)
        }
        listOf(
            "content://com.huawei.android.launcher.settings/badge/",
            "content://com.hihonor.android.launcher.settings/badge/"
        ).forEach { badgeUri ->
            runCatching {
                contentResolver.call(Uri.parse(badgeUri), "change_badge", null, payload)
            }
        }
    }

    private fun showTaskCompletedNotification(count: Int, wasDevelopment: Boolean, apkUrl: String?) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val title = if (wasDevelopment) "开发任务已完成" else "任务已完成"
        val text = if (apkUrl != null) {
            "已有 $count 个任务完成，APK 可以下载测试。"
        } else {
            "已有 $count 个任务完成，点击查看结果。"
        }
        val notification = NotificationCompat.Builder(this, TASK_COMPLETE_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle(title)
            .setContentText(text)
            .setNumber(count)
            .setBadgeIconType(NotificationCompat.BADGE_ICON_SMALL)
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()
        runCatching {
            NotificationManagerCompat.from(this).notify(TASK_COMPLETE_NOTIFICATION_ID, notification)
        }
    }

    private fun normalizeProject(project: AppProject) {
        if (project.conversations.isEmpty()) project.conversations.add(createDefaultConversation())
        project.conversations.forEach {
            if (it.messages.isEmpty()) it.messages.add(welcomeMessage())
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

    private fun loadLegacyProject(): AppProject {
        val saved = prefs.getString("conversations_json", null)
        val legacyConversations = runCatching {
            if (saved.isNullOrBlank()) null
            else gson.fromJson(saved, Array<AppConversation>::class.java)?.toMutableList()
        }.getOrNull().orEmpty().filter { it.title.isNotBlank() }.toMutableList()
        legacyConversations.forEach {
            if (it.messages.isEmpty()) it.messages.add(welcomeMessage())
        }
        if (legacyConversations.isEmpty()) legacyConversations.add(createDefaultConversation())

        val savedEvents = prefs.getString("project_events", "").orEmpty()
        val title = prefs.getString("project_title", null)?.takeIf { it.isNotBlank() } ?: "一龙开发助手"
        return AppProject(
            id = UUID.randomUUID().toString(),
            title = summarize(title, 24),
            subtitle = "默认项目 · ${legacyConversations.size} 个会话",
            updatedAt = legacyConversations.maxOfOrNull { it.updatedAt } ?: System.currentTimeMillis(),
            conversations = legacyConversations,
            events = savedEvents.lines().filter { it.isNotBlank() }.toMutableList()
        )
    }

    private fun updateFirstConversationStatus(text: String) {
        if (conversations.isEmpty()) conversations.add(createDefaultConversation())
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

    private fun blendColor(startColor: Int, endColor: Int, fraction: Float): Int {
        val clamped = fraction.coerceIn(0f, 1f)
        val alpha = (Color.alpha(startColor) + (Color.alpha(endColor) - Color.alpha(startColor)) * clamped).toInt()
        val red = (Color.red(startColor) + (Color.red(endColor) - Color.red(startColor)) * clamped).toInt()
        val green = (Color.green(startColor) + (Color.green(endColor) - Color.green(startColor)) * clamped).toInt()
        val blue = (Color.blue(startColor) + (Color.blue(endColor) - Color.blue(startColor)) * clamped).toInt()
        return Color.argb(alpha, red, green, blue)
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

    private fun avatarText(title: String): String {
        return if (title.startsWith("一龙")) "龙" else title.take(1).ifBlank { "新" }
    }

    private fun conversationSubtitleColor(text: String): Int {
        return when {
            text.startsWith("已连接") -> Color.parseColor("#07C160")
            text.startsWith("未连接") -> Color.parseColor("#D93025")
            text.startsWith("工作完成") -> Color.parseColor("#07C160")
            text.startsWith("工作停止") -> Color.parseColor("#D93025")
            text.startsWith("会话已结束") -> Color.parseColor("#6E6E6E")
            else -> Color.parseColor("#A9A9A9")
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
        val loggedIn = AuthManager.isLoggedIn(this)
        binding.profileLoginButton.visibility = if (loggedIn) View.GONE else View.VISIBLE
        binding.profileLogoutButton.visibility = if (loggedIn) View.VISIBLE else View.GONE
        binding.profileImportGuestButton.visibility =
            if (loggedIn && importableGuestProjects().isNotEmpty()) View.VISIBLE else View.GONE
        binding.userInfoText.text = buildAccountInfoText()
    }

    private fun buildAccountInfoText(): String {
        return if (AuthManager.isLoggedIn(this)) {
            val name = AuthManager.displayName(this)
            val account = AuthManager.account(this)
            val tail = if (account != null && account != name) " · $account" else ""
            "我的开发工作台\n登录账号：$name$tail\n云端工作区已就绪，可在网页版和其它手机间同步。"
        } else {
            "我的开发工作台\n游客模式 · 项目仅保存在本机\n登录后可在网页版和其它手机间继续同一个项目。"
        }
    }

    /** 返回游客 prefs 中"值得导入"的项目列表（已登录、且当前账号中不存在）。 */
    private fun importableGuestProjects(): List<AppProject> {
        if (!AuthManager.isLoggedIn(this)) return emptyList()
        val json = AuthManager.guestDataPrefs(this).getString("projects_json", null) ?: return emptyList()
        val all = runCatching {
            gson.fromJson(json, Array<AppProject>::class.java)?.toList()
        }.getOrNull() ?: return emptyList()
        val existingIds = projects.map { it.id }.toSet()
        return all.filter { p ->
            p.id != "elon-self" &&
            p.id !in existingIds &&
            p.conversations.any { c -> c.messages.any { m -> m.role == "user" } }
        }
    }

    /** 首次登录后自动弹窗询问是否导入游客记录（每个游客 ID 只弹一次）。 */
    private fun checkAndOfferGuestImport() {
        if (!AuthManager.isLoggedIn(this)) return
        val guestId = AuthManager.legacyAnonymousUserId(this)
        val offerKey = "guest_import_offered_$guestId"
        if (prefs.getBoolean(offerKey, false)) return
        val importable = importableGuestProjects()
        if (importable.isEmpty()) return
        prefs.edit().putBoolean(offerKey, true).apply()
        AlertDialog.Builder(this)
            .setTitle("发现游客记录")
            .setMessage("检测到本机游客状态下有 ${importable.size} 个项目，是否导入到当前账号？")
            .setPositiveButton("导入") { _, _ -> performGuestImport(importable) }
            .setNegativeButton("暂不导入", null)
            .show()
    }

    /** 手动从个人页触发的导入入口。 */
    private fun showGuestImportDialog() {
        val importable = importableGuestProjects()
        if (importable.isEmpty()) {
            android.widget.Toast.makeText(this, "没有可导入的游客记录", android.widget.Toast.LENGTH_SHORT).show()
            return
        }
        AlertDialog.Builder(this)
            .setTitle("导入游客记录")
            .setMessage("将导入 ${importable.size} 个游客项目到当前账号，是否继续？")
            .setPositiveButton("导入") { _, _ -> performGuestImport(importable) }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun performGuestImport(importable: List<AppProject>) {
        var count = 0
        for (p in importable) {
            if (projects.none { it.id == p.id }) {
                projects.add(p)
                count++
            }
        }
        if (count > 0) {
            saveProjects()
            renderProjectList()
            refreshAccountUi()
            android.widget.Toast.makeText(this, "已导入 $count 个游客项目", android.widget.Toast.LENGTH_SHORT).show()
        }
    }

    private fun confirmLogout() {
        AlertDialog.Builder(this)
            .setTitle("退出登录")
            .setMessage("退出后将切换为游客模式。已经登录的项目数据仍保留在云端，可重新登录恢复。")
            .setPositiveButton("继续退出") { _, _ -> confirmLogoutStep2() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun confirmLogoutStep2() {
        AlertDialog.Builder(this)
            .setTitle("再次确认")
            .setMessage("确认退出当前账号？")
            .setPositiveButton("确认退出") { _, _ -> performLogout() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun performLogout() {
        AuthManager.clear(this)
        val intent = Intent(this, LoginActivity::class.java)
        intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
        startActivity(intent)
        finish()
    }

    private fun localAppVersionLine(): String =
        "一龙 v${BuildConfig.VERSION_NAME}  (build ${BuildConfig.VERSION_CODE})"

    private fun refreshServerVersion() {
        Thread {
            val info = fetchServerVersionInfo()
            val serverLine = info?.let { formatServerVersionLine(it) } ?: "服务器版本暂不可用"
            runOnUiThread {
                if (::binding.isInitialized) {
                    binding.profileVersionText.text = "${localAppVersionLine()}\n$serverLine"
                }
            }
        }.start()
    }

    private fun fetchServerVersionInfo(): ServerVersionInfo? = try {
        val request = Request.Builder()
            .url(serverVersionUrl)
            .addHeader("Cache-Control", "no-cache")
            .build()
        http.newCall(request).execute().use { resp ->
            if (!resp.isSuccessful) return null
            val body = resp.body?.string() ?: return null
            val json = JSONObject(body)
            val versionName = json.optString("versionName", json.optString("version_name", ""))
            val gitSha = json.optString("gitSha", json.optString("git_sha", ""))
            if (versionName.isBlank()) return null
            ServerVersionInfo(versionName = versionName, gitSha = gitSha.takeIf { it.isNotBlank() })
        }
    } catch (_: Exception) {
        null
    }

    private fun formatServerVersionLine(info: ServerVersionInfo): String {
        val shortSha = info.gitSha
            ?.takeIf { it != "dev" }
            ?.take(8)
        return if (shortSha.isNullOrBlank()) {
            "服务器 v${info.versionName}"
        } else {
            "服务器 v${info.versionName} ($shortSha)"
        }
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
        actionPopup?.dismiss()

        val popupWidth = dp(174)
        val arrowHeight = dp(9)
        val root = FrameLayout(this).apply {
            layoutParams = ViewGroup.LayoutParams(popupWidth, ViewGroup.LayoutParams.WRAP_CONTENT)
        }

        val panel = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                cornerRadius = dp(4).toFloat()
                setColor(Color.parseColor("#3D3D3D"))
            }
        }
        root.addView(panel, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = arrowHeight
        })

        root.addView(createPopupArrowView(), FrameLayout.LayoutParams(dp(20), arrowHeight).apply {
            gravity = Gravity.TOP or Gravity.END
            rightMargin = dp(20)
        })

        actions.forEachIndexed { index, action ->
            panel.addView(createTopActionRow(action))
            if (index < actions.lastIndex) {
                panel.addView(View(this).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        1
                    ).apply {
                        marginStart = dp(64)
                    }
                    setBackgroundColor(Color.parseColor("#555555"))
                })
            }
        }

        actionPopup = PopupWindow(
            root,
            popupWidth,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            true
        ).apply {
            isOutsideTouchable = true
            elevation = dp(8).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAsDropDown(anchor, anchor.width - popupWidth + dp(2), -dp(2))
        }
    }

    private fun createTopActionRow(action: TopAction): View {
        return LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(52)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(22), 0, dp(12), 0)
            isClickable = true
            foreground = selectableForeground()

            addView(ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(28), dp(28))
                setImageResource(action.iconRes)
                setColorFilter(Color.WHITE)
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    marginStart = dp(18)
                }
                includeFontPadding = false
                text = action.title
                setTextColor(Color.WHITE)
                textSize = 18f
            })
            setOnClickListener {
                actionPopup?.dismiss()
                action.action()
            }
        }
    }

    private fun createPopupArrowView(pointsUp: Boolean = true): View {
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.parseColor("#3D3D3D")
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
        val actions = arrayOf("查看同步状态", "查看通用工作流", "配置 GitHub 仓库", "生成并复制 Deploy Key", "打开 GitHub Deploy Keys", "授权说明")
        AlertDialog.Builder(this)
            .setTitle("${currentProjectTitle} · Git 仓库")
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "查看同步状态" -> loadGitProjectStatus { status -> showGitStatusDialog(status) }
                    "查看通用工作流" -> loadGitProjectStatus { status -> showProjectWorkflowDialog(status) }
                    "配置 GitHub 仓库" -> showConfigureGitDialog()
                    "生成并复制 Deploy Key" -> generateDeployKey()
                    "打开 GitHub Deploy Keys" -> loadGitProjectStatus { status -> openUrl(status.deployKeysUrl) }
                    "授权说明" -> showGitAuthHelpDialog()
                }
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun showGitStatusDialog(status: GitProjectStatus) {
        val remoteLine = when (status.remoteOk) {
            true -> "远端权限：正常"
            false -> "远端权限：未通过\n${status.remoteMessage.orEmpty().ifBlank { "请检查 Deploy Key 是否已加到 GitHub，并勾选写权限。" }}"
            null -> "远端权限：尚未配置远端"
        }
        AlertDialog.Builder(this)
            .setTitle("${currentProjectTitle} · Git 状态")
            .setMessage(
                buildString {
                    append("Git 工作区：${if (status.hasGit) "已准备" else "未初始化"}\n")
                    append("远端：${status.origin ?: "未配置"}\n")
                    append("分支：${status.branch ?: "未设置"}\n")
                    append("Deploy Key：${if (status.deployKeyExists) "已生成" else "未生成"}\n")
                    append(remoteLine)
                }
            )
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun showProjectWorkflowDialog(status: GitProjectStatus) {
        AlertDialog.Builder(this)
            .setTitle(status.workflowTitle.ifBlank { "通用项目工作流" })
            .setMessage(projectWorkflowDialogText(status))
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun showConfigureGitDialog() {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(18), dp(6), dp(18), 0)
        }
        val repoInput = EditText(this).apply {
            hint = "git@github.com:owner/repo.git"
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            setSingleLine(true)
        }
        val branchInput = EditText(this).apply {
            hint = "main"
            setText("main")
            inputType = InputType.TYPE_CLASS_TEXT
            setSingleLine(true)
        }
        root.addView(TextView(this).apply {
            text = "仓库地址"
            setTextColor(Color.parseColor("#444444"))
            textSize = 13f
        })
        root.addView(repoInput)
        root.addView(TextView(this).apply {
            text = "分支"
            setTextColor(Color.parseColor("#444444"))
            textSize = 13f
        })
        root.addView(branchInput)

        val dialog = AlertDialog.Builder(this)
            .setTitle("配置 GitHub 仓库")
            .setView(root)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val repo = repoInput.text.toString().trim()
                val branch = branchInput.text.toString().trim().ifBlank { "main" }
                if (repo.isBlank()) {
                    repoInput.error = "请输入 GitHub 仓库地址"
                    return@setOnClickListener
                }
                saveGitConfig(repo, branch)
                dialog.dismiss()
            }
        }
        dialog.show()
    }

    private fun showGitAuthHelpDialog() {
        AlertDialog.Builder(this)
            .setTitle("GitHub 授权说明")
            .setMessage(
                "当前版本使用每项目 Deploy Key：先生成公钥，在 GitHub 仓库 Settings → Deploy keys 添加，并勾选写权限。\n\n" +
                    "正式多用户版会接入 GitHub App，用户只需要在 GitHub 授权指定仓库，服务器再用短期 token 读写代码。"
            )
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun loadGitProjectStatus(onLoaded: (GitProjectStatus) -> Unit) {
        Thread {
            try {
                val response = http.newCall(
                    Request.Builder()
                        .url(projectGitUrl("status"))
                        .get()
                        .build()
                ).execute()
                val body = response.body?.string().orEmpty()
                if (!response.isSuccessful) error(body.ifBlank { "HTTP ${response.code}" })
                val status = parseGitProjectStatus(JSONObject(body))
                runOnUiThread { onLoaded(status) }
            } catch (e: Exception) {
                runOnUiThread {
                    Toast.makeText(this, "Git 状态读取失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun generateDeployKey() {
        Thread {
            try {
                val emptyBody = "{}".toRequestBody("application/json".toMediaType())
                val response = http.newCall(
                    Request.Builder()
                        .url(projectGitUrl("deploy-key"))
                        .post(emptyBody)
                        .build()
                ).execute()
                val body = response.body?.string().orEmpty()
                if (!response.isSuccessful) error(body.ifBlank { "HTTP ${response.code}" })
                val json = JSONObject(body)
                val publicKey = json.optString("public_key", "")
                val status = parseGitProjectStatus(json.optJSONObject("status") ?: JSONObject())
                runOnUiThread {
                    copyText("GitHub Deploy Key", publicKey)
                    AlertDialog.Builder(this)
                        .setTitle("Deploy Key 已复制")
                        .setMessage(
                            "已复制公钥。请到 GitHub 仓库 Settings → Deploy keys 添加它，并勾选写权限。\n\n$publicKey"
                        )
                        .setPositiveButton("打开 GitHub") { _, _ -> openUrl(status.deployKeysUrl) }
                        .setNegativeButton("知道了", null)
                        .show()
                }
            } catch (e: Exception) {
                runOnUiThread {
                    Toast.makeText(this, "Deploy Key 生成失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun saveGitConfig(repoUrl: String, branch: String) {
        Thread {
            try {
                val payload = JSONObject().apply {
                    put("repo_url", repoUrl)
                    put("branch", branch)
                    put("auth_type", "deploy_key")
                }
                val body = payload.toString().toRequestBody("application/json".toMediaType())
                val response = http.newCall(
                    Request.Builder()
                        .url(projectGitUrl("config"))
                        .post(body)
                        .build()
                ).execute()
                val responseBody = response.body?.string().orEmpty()
                if (!response.isSuccessful) error(responseBody.ifBlank { "HTTP ${response.code}" })
                val status = parseGitProjectStatus(JSONObject(responseBody))
                runOnUiThread {
                    activeProject().subtitle = if (status.remoteOk == true) {
                        "GitHub 仓库已连接"
                    } else {
                        "GitHub 仓库待授权"
                    }
                    addProjectEvent("Git 仓库配置：${summarize(repoUrl, 30)}")
                    showGitStatusDialog(status)
                }
            } catch (e: Exception) {
                runOnUiThread {
                    Toast.makeText(this, "Git 配置失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun projectGitUrl(action: String): String {
        return "$serverUrl/api/user/${urlPart(userId)}/projects/${urlPart(activeProject().id)}/git/$action?title=${urlPart(activeProject().title)}"
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

        val selectedAgent = if (codexCliOnly) null else selectedAgentName
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

    private fun urlPart(value: String): String {
        return URLEncoder.encode(value, "UTF-8").replace("+", "%20")
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
        actionPopup?.dismiss()

        val actions = listOf(
            TopAction("复制", R.drawable.ic_msg_copy) { copyMessageText(text) },
            TopAction("转发", R.drawable.ic_msg_forward) { forwardMessageText(text) },
            TopAction("收藏", R.drawable.ic_msg_favorite) { toastMessageAction("已收藏") },
            TopAction("删除", R.drawable.ic_msg_delete) { deleteMessage(message) },
            TopAction("多选", R.drawable.ic_msg_multi) { toastMessageAction("多选准备中") },
            TopAction("引用", R.drawable.ic_msg_quote) { quoteMessage(text) },
            TopAction("提醒", R.drawable.ic_msg_remind) { toastMessageAction("提醒准备中") },
            TopAction("搜一搜", R.drawable.ic_msg_search) { searchMessageText(text) },
            TopAction("从当前听", R.drawable.ic_msg_listen) { toastMessageAction("从当前听准备中") }
        )

        val popupWidth = minOf(resources.displayMetrics.widthPixels - dp(24), dp(282))
        val arrowHeight = dp(8)
        val panelHeight = dp(132)
        val totalHeight = panelHeight + arrowHeight
        val root = FrameLayout(this).apply {
            layoutParams = ViewGroup.LayoutParams(popupWidth, totalHeight)
            alpha = 0f
            scaleX = 0.96f
            scaleY = 0.96f
        }

        val panel = GridLayout(this).apply {
            columnCount = 5
            rowCount = 2
            background = GradientDrawable().apply {
                cornerRadius = dp(4).toFloat()
                setColor(Color.parseColor("#3D3D3D"))
            }
            setPadding(dp(10), dp(8), dp(10), dp(8))
        }
        root.addView(panel, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            panelHeight
        ))

        actions.forEach { action ->
            panel.addView(createMessageActionCell(action), GridLayout.LayoutParams().apply {
                width = (popupWidth - dp(20)) / 5
                height = dp(58)
            })
        }

        val anchorLocation = IntArray(2)
        anchor.getLocationOnScreen(anchorLocation)
        val anchorCenterX = anchorLocation[0] + anchor.width / 2
        val aboveY = anchorLocation[1] - totalHeight - dp(8)
        val showAbove = aboveY > dp(76)
        val popupX = (anchorCenterX - popupWidth / 2)
            .coerceIn(dp(12), resources.displayMetrics.widthPixels - popupWidth - dp(12))
        val popupY = if (showAbove) aboveY else anchorLocation[1] + anchor.height + dp(8)
        val arrowX = (anchorCenterX - popupX - dp(9)).coerceIn(dp(18), popupWidth - dp(36))

        root.addView(createPopupArrowView(pointsUp = !showAbove), FrameLayout.LayoutParams(dp(18), arrowHeight).apply {
            gravity = if (showAbove) Gravity.BOTTOM or Gravity.START else Gravity.TOP or Gravity.START
            leftMargin = arrowX
        })
        if (!showAbove) {
            (panel.layoutParams as FrameLayout.LayoutParams).topMargin = arrowHeight
        }

        actionPopup = PopupWindow(
            root,
            popupWidth,
            totalHeight,
            true
        ).apply {
            isOutsideTouchable = true
            elevation = dp(8).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAtLocation(binding.root, Gravity.NO_GRAVITY, popupX, popupY)
        }
        root.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .setDuration(120L)
            .start()
    }

    private fun createMessageActionCell(action: TopAction): View {
        return LinearLayout(this).apply {
            gravity = Gravity.CENTER
            orientation = LinearLayout.VERTICAL
            isClickable = true
            foreground = selectableForeground()

            addView(ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(24), dp(24))
                setImageResource(action.iconRes)
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(4)
                }
                includeFontPadding = false
                text = action.title
                setTextColor(Color.parseColor("#EAEAEA"))
                textSize = 13f
            })
            setOnClickListener {
                actionPopup?.dismiss()
                action.action()
            }
        }
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

    private fun searchMessageText(text: String) {
        Toast.makeText(this, "搜一搜：${summarize(text, 12)}", Toast.LENGTH_SHORT).show()
    }

    private fun toastMessageAction(text: String) {
        Toast.makeText(this, text, Toast.LENGTH_SHORT).show()
    }

    private fun copyMessageText(text: String) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("一龙聊天内容", text))
        Toast.makeText(this, "已复制", Toast.LENGTH_SHORT).show()
    }

    private fun showPromotionDialog() {
        val text = promotionText(apkDownloadUrl)
        val content = TextView(this).apply {
            setText(text)
            setTextIsSelectable(true)
            setPadding(dp(22), dp(8), dp(22), dp(2))
            setTextColor(Color.parseColor("#333333"))
            textSize = 14f
            setLineSpacing(dp(3).toFloat(), 1.0f)
        }

        AlertDialog.Builder(this)
            .setTitle("分享推广")
            .setView(content)
            .setPositiveButton("复制推广语") { _, _ -> copyPromotionText(text) }
            .setNeutralButton("系统分享") { _, _ -> sharePromotionText(text) }
            .setNegativeButton("打开下载页") { _, _ -> openApkDownloadPage() }
            .show()
    }

    private fun copyPromotionText(text: String) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("一龙推广语", text))
        Toast.makeText(this, "推广语已复制", Toast.LENGTH_SHORT).show()
    }

    private fun sharePromotionText(text: String) {
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        startActivity(Intent.createChooser(intent, "分享一龙 APK"))
    }

    private fun openApkDownloadPage() {
        startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(apkDownloadPageUrl)))
    }

    private fun forwardMessageText(text: String) {
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        startActivity(Intent.createChooser(intent, "转发聊天内容"))
    }

    private fun jsonStringOrNull(json: com.google.gson.JsonObject, name: String): String? {
        val element = json.get(name) ?: return null
        if (element.isJsonNull) return null
        return runCatching { element.asString }.getOrNull()
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

    private fun aiMessageWithCurrentEvidence(content: String): ChatMessage {
        val message = ChatMessage("ai", content)
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
        maybeAppendVisibleCliSignal(category, line)
        if (category != "模型回复") {
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

    private fun maybeAppendVisibleCliSignal(category: String, line: String) {
        if (!activeRequestIsDevelopment) return
        val narrative = CodexProgressNarrative.fromCliOutput(category, line) ?: return
        appendProgressNarrative(narrative)
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
                    handleProgress(content)
                    if (surfaced) return
                    if (shouldShowProgressBubble(content)) {
                        ChatMessage("ai-progress", workflowProgressMessage(content))
                    } else {
                        return
                    }
                }
                "tool_call"   -> {
                    val tool = jsonStringOrNull(json, "tool") ?: "工具"
                    maybeAppendToolCallNarrative(tool)
                    handleToolCall(tool)
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
                    updateStage(currentStage, "${toolLabel(tool)} 已完成，正在判断下一步。")
                    addProjectEvent("工具完成：${toolLabel(tool)}")
                    return
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
                    val finalMessage = aiMessageWithCurrentEvidence(finalReplyMessage(content, visibleApkUrl, imageUrl, wasDevelopment))
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

    private fun handleProgress(content: String) {
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
        if (!content.startsWith("CLI 仍在运行")) {
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

    private fun shouldShowProgressBubble(content: String): Boolean {
        val progress = userFacingProgress(content)
        return !isRoutineWorkflowMessage(workflowProgressMessage(content)) &&
            !isRoutineWorkflowMessage(progress) &&
            (content.startsWith("环境提醒") ||
                content.contains("已识别为开发任务") ||
                content.contains("正在确认这是否需要进入开发流程") ||
                content.startsWith("正在准备项目工作区") ||
                content.contains("AI 助手正在处理") ||
                content.contains("通用项目工作流") ||
                content.contains("已轮到本会话任务") ||
                content.contains("已获得本会话执行权") ||
                content.contains("已获得项目执行权") ||
                content.contains("进入队列") ||
                content.contains("排队") ||
                content.startsWith("未找到 APK") ||
                content.contains("失败") ||
                content.contains("错误") ||
                content.contains("不可用"))
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

    private fun nextWorkflowHint(stage: String): String {
        return when (stage) {
            "需求分析" -> "定位相关文件。"
            "开发实现" -> "继续修改并检查结果。"
            "编译打包" -> "等待编译结果。"
            "交付完成" -> "整理最终结果。"
            "需要处理" -> "根据错误判断是否可修复。"
            else -> "等待下一步结果。"
        }
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
        binding.userInfoText.text = buildAccountInfoText()

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

    private fun stageLine(index: Int, active: Int, label: String): String {
        val state = when {
            active == -1 -> if (index == 1) "需处理" else "等待"
            active > index -> "已完成"
            active == index -> "进行中"
            else -> "等待"
        }
        return "$index. $label：$state"
    }

    private fun compactProjectTitle(): String {
        return currentProjectTitle.trim().ifBlank { getString(R.string.app_name) }.take(6)
    }

    private companion object {
        const val PREF_SELECTED_AGENT = "selected_agent_name"
        const val PREF_SELECTED_MODEL_LABEL = "selected_model_label"
        const val PREF_PENDING_WORK_PAYLOAD = "pending_work_payload"
        const val PREF_PENDING_WORK_IS_DEVELOPMENT = "pending_work_is_development"
        const val PREF_PENDING_WORK_TIME = "pending_work_time"
        const val PREF_COMPLETED_TASK_BADGE_COUNT = "completed_task_badge_count"
        const val PREF_NOTIFICATION_PERMISSION_ASKED = "notification_permission_asked"
        const val TASK_COMPLETE_CHANNEL_ID = "task_complete_alerts"
        const val TASK_COMPLETE_NOTIFICATION_ID = 2401
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
        binding.inputEdit.hint = if (conversationEnded) "会话已结束，请新建会话继续" else "描述你想开发的 App 功能"
        if (::inputModeButton.isInitialized) {
            inputModeButton.isEnabled = !conversationEnded
            inputModeButton.alpha = if (conversationEnded) 0.55f else 1f
        }
        if (::voiceHoldButton.isInitialized) {
            voiceHoldButton.isEnabled = !conversationEnded
            voiceHoldButton.alpha = if (conversationEnded) 0.55f else 1f
        }
        binding.modelButton.isEnabled = !conversationEnded
        updateSendButtonVisual()
        updateStageHintShimmer()
    }

    private fun quickLocalChatReply(text: String): String? {
        if (pendingAttachments.isNotEmpty()) return null
        if (looksLikeDevelopmentRequest(text) || looksLikeDirectImageRequest(text)) return null
        return when (text.trim().lowercase(Locale.CHINA)) {
            "你好", "你好呀", "在吗", "你在吗", "在不在", "hi", "hello" ->
                "你好，我在。你可以直接告诉我想改代码、查问题、构建 APK，或者先聊聊想法。"
            "谢谢", "谢谢你", "辛苦了" ->
                "不客气，我在这边。你继续说下一步想怎么改就行。"
            else -> null
        }
    }

    private fun expandShortDevelopmentCommand(text: String): String {
        val normalized = text.trim().lowercase(Locale.CHINA)
        return when {
            looksLikeResumeCommand(normalized) -> buildResumeDevelopmentCommand(text)
            normalized in setOf("打包", "编译", "生成apk", "生成 apk", "打包apk", "打包 apk") ->
                "请编译当前项目并生成可以下载安装到手机的 APK 下载链接。"
            else -> text
        }
    }

    private fun buildResumeDevelopmentCommand(originalText: String): String {
        val lastRequest = lastActionableUserRequest()
        val latestFailure = latestFailureMessage()
        return buildString {
            append("请继续完成上一次未完成的开发任务，不要只返回之前已经生成过的 APK。")
            if (!lastRequest.isNullOrBlank()) {
                append("\n\n上一条未完成的用户需求：")
                append(lastRequest)
            }
            if (!latestFailure.isNullOrBlank()) {
                append("\n\n最近一次中断或错误：")
                append(latestFailure)
            }
            append("\n\n当前用户补充：")
            append(originalText.trim())
            append("\n\n请结合当前项目文件、上一条用户需求和最近错误继续完成开发；只有确认该需求已经实现，并重新检查或构建后，才返回新的 APK 下载链接和当前进度。")
        }
    }

    private fun lastActionableUserRequest(): String? {
        return activeConversation().messages
            .asReversed()
            .firstOrNull { message ->
                message.role == "user" &&
                    message.content.isNotBlank() &&
                    !looksLikeResumeCommand(message.content.trim().lowercase(Locale.CHINA)) &&
                    !looksLikeApkDeliveryRequest(message.content)
            }
            ?.content
            ?.trim()
    }

    private fun latestFailureMessage(): String? {
        return activeConversation().messages
            .asReversed()
            .firstOrNull { it.role == "error" || it.role == "ai-stopped" }
            ?.content
            ?.trim()
            ?.takeIf { it.isNotBlank() }
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

