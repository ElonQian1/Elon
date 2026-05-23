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
import android.provider.OpenableColumns
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.text.Editable
import android.text.InputType
import android.text.TextUtils
import android.text.TextWatcher
import android.util.Base64
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
    private val emittedProgressSignals = linkedSetOf<String>()
    private var visibleAssistantUpdateCount = 0
    private var serverResponseToken = 0
    private var appInForeground = false
    private var pendingRequestPayload: String? = null
    private var pendingReconnectForActiveWork = false
    private var reconnectAttempts = 0
    private var backendConnected = false
    private var taskWorkReceiverRegistered = false
    private val projects = mutableListOf<AppProject>()
    private val gson = com.google.gson.Gson()
    private val http = OkHttpClient()
    private val timeFormatter = SimpleDateFormat("HH:mm", Locale.CHINA)
    private val prefs by lazy { getSharedPreferences("elon", MODE_PRIVATE) }
    private val serverUrl = "http://43.139.149.158:8080"
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
    private lateinit var photoAttachmentLauncher: ActivityResultLauncher<String>
    private lateinit var documentAttachmentLauncher: ActivityResultLauncher<Array<String>>
    private var pendingCameraUri: Uri? = null
    private var pendingCameraName: String? = null
    private val pendingAttachments = mutableListOf<PendingAttachment>()
    private val taskWorkReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            handleTaskWorkEvent(intent)
        }
    }

    private data class AppConversation(
        val id: String,
        var title: String,
        var subtitle: String,
        var updatedAt: Long,
        var ended: Boolean = false,
        val messages: MutableList<ChatMessage>
    )

    private data class AppProject(
        val id: String,
        var title: String,
        var subtitle: String,
        var updatedAt: Long,
        var stage: String = "待提交需求",
        var activeConversationIndex: Int = 0,
        val events: MutableList<String> = mutableListOf(),
        val conversations: MutableList<AppConversation> = mutableListOf()
    )

    private data class ModelOption(
        val label: String,
        val agentName: String?
    )

    private data class PendingAttachment(
        val kind: String,
        val displayName: String,
        val fileName: String,
        val mimeType: String,
        val file: File
    )

    private data class EvidenceEntry(
        val kind: String,
        val text: String
    )

    private class TopAction(
        val title: String,
        val iconRes: Int,
        val action: () -> Unit
    )

    /** 每次安装 APP 生成的唯一用户 ID，存入 SharedPreferences 持久化 */
    private val userId: String by lazy {
        prefs.getString("user_id", null) ?: UUID.randomUUID().toString().replace("-", "").also {
            prefs.edit().putString("user_id", it).apply()
        }
    }

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
        startTaskWorkService(
            if (waitingForReply) TaskWorkService.ACTION_RESUME_PENDING else TaskWorkService.ACTION_CONNECT
        )

        // 重连按钮
        binding.statusText.setOnClickListener {
            if (backendConnected) openConversation(0)
            else startTaskWorkService(TaskWorkService.ACTION_CONNECT)
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
    }

    override fun onResume() {
        super.onResume()
        appInForeground = true
        setTaskAppForeground(true)
        drainQueuedTaskEvents()
        clearCompletedTaskBadge()
        if (::binding.isInitialized) {
            loadModelOptions()
            if (!backendConnected) {
                if (waitingForReply && !pendingReconnectForActiveWork) {
                    pendingReconnectForActiveWork = true
                    updateStage(currentStage, "正在恢复连接，回来后会自动继续本轮任务。")
                    recordEvidence("connection", "连接恢复中，正在继续上次任务")
                    appendMessage(ChatMessage("ai-cli-log", "连接恢复中 · 正在继续上次任务"))
                }
                startTaskWorkService(
                    if (waitingForReply) TaskWorkService.ACTION_RESUME_PENDING else TaskWorkService.ACTION_CONNECT
                )
            } else if (!waitingForReply) {
                setSendEnabled(true)
            }
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
        if (activeConversation().ended) {
            appendMessage(ChatMessage("error", "这个会话已结束，请新建会话继续。"))
            return
        }
        val outgoingText = expandShortDevelopmentCommand(text)
        quickLocalChatReply(outgoingText)?.let { reply ->
            appendMessage(ChatMessage(role = "user", content = text))
            binding.inputEdit.text.clear()
            updateProjectViews("普通消息已回复，开发项目记录保持不变。")
            appendMessage(ChatMessage(role = "ai", content = reply))
            return
        }
        val attachmentPayload = attachmentPayloadJsonOrNull() ?: return

        val payload = com.google.gson.JsonObject().apply {
            addProperty("user_id", userId)
            addProperty("project_id", activeProject().id)
            addProperty("message", outgoingText)
            selectedAgentName?.let { addProperty("agent", it) }
            if (attachmentPayload.size() > 0) add("attachments", attachmentPayload)
        }

        // 显示用户消息
        appendMessage(ChatMessage(role = "user", content = text))
        binding.inputEdit.text.clear()
        setSendEnabled(false)
        waitingForReply = true
        activeRequestIsDevelopment = looksLikeDevelopmentRequest(outgoingText) && !looksLikeDirectImageRequest(outgoingText)
        pendingRequestPayload = payload.toString()
        pendingReconnectForActiveWork = false
        reconnectAttempts = 0
        persistActiveWork()
        workflowStepIndex = 0
        resetFoldedCliLog()
        currentEvidenceEntries.clear()
        emittedProgressSignals.clear()
        visibleAssistantUpdateCount = 0
        if (activeRequestIsDevelopment) {
            updateProjectTitleFromRequest(text)
            saveProjectTitle()
            addProjectEvent("提交需求：${summarize(text, 36)}")
            updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
        } else {
            updateProjectViews("普通消息已发送，开发项目记录保持不变。")
        }
        appendMessage(ChatMessage("ai-working", initialWorkflowMessage(activeRequestIsDevelopment)))

        // 通过前台任务服务发送 JSON（包含 user_id，服务端据此隔离工作区）
        val responseToken = ++serverResponseToken
        if (!startTaskWorkService(TaskWorkService.ACTION_START_WORK, payload.toString(), activeRequestIsDevelopment)) {
            waitingForReply = false
            setSendEnabled(true)
            pendingRequestPayload = null
            pendingReconnectForActiveWork = false
            clearPersistedActiveWork()
            val wasDevelopment = activeRequestIsDevelopment
            if (wasDevelopment) {
                updateStage("需要处理", "消息发送失败，请检查网络后重试。")
            }
            appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("消息发送失败，请检查网络后重试。", wasDevelopment)))
            activeRequestIsDevelopment = false
        } else {
            clearPendingAttachments()
            scheduleFirstServerResponseWatchdog(responseToken)
        }
    }

    private fun pauseCurrentWork() {
        if (!waitingForReply) return
        val wasDevelopment = activeRequestIsDevelopment
        waitingForReply = false
        activeRequestIsDevelopment = false
        pendingRequestPayload = null
        pendingReconnectForActiveWork = false
        reconnectAttempts = 0
        clearPersistedActiveWork()
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
        startTaskWorkService(TaskWorkService.ACTION_PAUSE)
    }

    private fun handleActiveWorkDisconnected() {
        pendingReconnectForActiveWork = true
        persistActiveWork()
        setSendEnabled(false)
        updateFirstConversationStatus("连接恢复中 · 回来后继续")
        if (activeRequestIsDevelopment) {
            updateStage(currentStage, "连接暂时断开，正在保留本轮任务并准备自动恢复。")
            recordEvidence("connection", "连接暂时断开，正在自动恢复任务")
        }
        appendMessage(ChatMessage("ai-cli-log", "连接暂时断开 · 正在自动恢复任务"))

        scheduleReconnectForActiveWork()
    }

    private fun scheduleReconnectForActiveWork() {
        if (!waitingForReply || !pendingReconnectForActiveWork) return
        reconnectAttempts += 1
        val delay = (800L * reconnectAttempts).coerceAtMost(5_000L)
        binding.root.postDelayed({
            if (!waitingForReply || !pendingReconnectForActiveWork || backendConnected) return@postDelayed
            startTaskWorkService(TaskWorkService.ACTION_RESUME_PENDING)
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
        appendMessage(ChatMessage("ai-cli-log", "连接已恢复 · 已自动继续上次任务"))
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
            scheduleFirstServerResponseWatchdog(responseToken)
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
        photoAttachmentLauncher = registerForActivityResult(ActivityResultContracts.GetContent()) { uri ->
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
            photoAttachmentLauncher.launch("image/*")
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
        val name = fallbackName ?: displayNameForUri(uri) ?: uri.lastPathSegment ?: kind
        val attachment = runCatching {
            copyAttachmentToCache(kind, uri, name)
        }.onFailure {
            Toast.makeText(this, "附件读取失败，请重新选择", Toast.LENGTH_SHORT).show()
        }.getOrNull() ?: return

        pendingAttachments.add(attachment)
        appendAttachmentLabel(kind, attachment.displayName)
        Toast.makeText(this, "已添加${kind}：${attachment.displayName}", Toast.LENGTH_SHORT).show()
    }

    private fun displayNameForUri(uri: Uri): String? {
        return runCatching {
            contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
                val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (index >= 0 && cursor.moveToFirst()) cursor.getString(index) else null
            }
        }.getOrNull()?.takeIf { it.isNotBlank() }
    }

    private fun copyAttachmentToCache(kind: String, uri: Uri, displayName: String): PendingAttachment {
        val mimeType = contentResolver.getType(uri) ?: guessMimeType(displayName)
        val extension = extensionForAttachment(displayName, mimeType)
        val fileName = "attachment_${System.currentTimeMillis()}_${pendingAttachments.size + 1}.$extension"
        val attachmentDir = File(cacheDir, "pending_attachments").apply { mkdirs() }
        val target = File(attachmentDir, fileName)
        var total = 0L
        contentResolver.openInputStream(uri).use { input ->
            requireNotNull(input) { "Cannot open selected file" }
            target.outputStream().use { output ->
                val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                while (true) {
                    val read = input.read(buffer)
                    if (read <= 0) break
                    total += read
                    if (total > MAX_ATTACHMENT_BYTES) {
                        target.delete()
                        throw IllegalArgumentException("Attachment too large")
                    }
                    output.write(buffer, 0, read)
                }
            }
        }
        return PendingAttachment(
            kind = kind,
            displayName = displayName,
            fileName = fileName,
            mimeType = mimeType,
            file = target
        )
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

    private fun attachmentPayloadJsonOrNull(): com.google.gson.JsonArray? {
        val array = com.google.gson.JsonArray()
        for (attachment in pendingAttachments) {
            val bytes = runCatching { attachment.file.readBytes() }.getOrElse {
                Toast.makeText(this, "附件已失效，请重新选择：${attachment.displayName}", Toast.LENGTH_SHORT).show()
                return null
            }
            if (bytes.size > MAX_ATTACHMENT_BYTES) {
                Toast.makeText(this, "附件过大，请重新选择较小文件：${attachment.displayName}", Toast.LENGTH_SHORT).show()
                return null
            }
            array.add(com.google.gson.JsonObject().apply {
                addProperty("kind", attachment.kind)
                addProperty("display_name", attachment.displayName)
                addProperty("file_name", attachment.fileName)
                addProperty("mime_type", attachment.mimeType)
                addProperty("data_base64", Base64.encodeToString(bytes, Base64.NO_WRAP))
            })
        }
        return array
    }

    private fun clearPendingAttachments() {
        pendingAttachments.forEach { attachment ->
            runCatching { attachment.file.delete() }
        }
        pendingAttachments.clear()
    }

    private fun guessMimeType(name: String): String {
        return when (name.substringAfterLast('.', "").lowercase(Locale.CHINA)) {
            "jpg", "jpeg" -> "image/jpeg"
            "png" -> "image/png"
            "webp" -> "image/webp"
            "gif" -> "image/gif"
            "pdf" -> "application/pdf"
            "txt" -> "text/plain"
            else -> "application/octet-stream"
        }
    }

    private fun extensionForAttachment(name: String, mimeType: String): String {
        val fromName = name.substringAfterLast('.', "").lowercase(Locale.CHINA)
            .filter { it.isLetterOrDigit() }
            .take(8)
        if (fromName.isNotBlank()) return fromName
        return when (mimeType) {
            "image/jpeg" -> "jpg"
            "image/png" -> "png"
            "image/webp" -> "webp"
            "image/gif" -> "gif"
            "application/pdf" -> "pdf"
            "text/plain" -> "txt"
            else -> "bin"
        }
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
        setSendEnabled(backendConnected && !waitingForReply)
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
                val config = json.optJSONObject("config") ?: JSONObject()
                val agents = json.optJSONArray("available_agents") ?: JSONArray()
                val options = mutableListOf(ModelOption("服务器默认", null))

                for (i in 0 until agents.length()) {
                    val item = agents.getJSONObject(i)
                    val name = item.optString("name", "")
                    val model = item.optString("model", "")
                    val provider = item.optString("provider", name)
                    val label = item.optString("label", modelLabel(provider, model))
                    if (name.isNotBlank()) {
                        options.add(ModelOption(label, name))
                    }
                }

                val serverUseAgent = jsonStringOrNull(config, "use_agent")
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

    private fun modelLabel(name: String, model: String): String {
        return if (model.isBlank()) name else "$name [$model]"
    }

    private fun shortModelLabel(label: String): String {
        return when {
            label.startsWith("服务器默认") -> "默认"
            label.startsWith("自定义") -> "自定"
            label.contains("/") -> label.substringBefore("/").trim().take(4)
            label.contains("[") -> label.substringBefore("[").trim().take(4)
            else -> label.take(4)
        }
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
            "你可以直接描述想开发的 App 功能；我会把需求分析、开发实现、编译打包和交付记录同步到进度页。"
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
            .apply()
    }

    private fun persistActiveWork() {
        val payload = pendingRequestPayload
        if (!waitingForReply || payload.isNullOrBlank()) {
            clearPersistedActiveWork()
            return
        }
        prefs.edit()
            .putString(PREF_PENDING_WORK_PAYLOAD, payload)
            .putBoolean(PREF_PENDING_WORK_IS_DEVELOPMENT, activeRequestIsDevelopment)
            .putLong(PREF_PENDING_WORK_TIME, System.currentTimeMillis())
            .apply()
    }

    private fun clearPersistedActiveWork() {
        prefs.edit()
            .remove(PREF_PENDING_WORK_PAYLOAD)
            .remove(PREF_PENDING_WORK_IS_DEVELOPMENT)
            .remove(PREF_PENDING_WORK_TIME)
            .apply()
    }

    private fun restorePendingActiveWork() {
        val payload = prefs.getString(PREF_PENDING_WORK_PAYLOAD, null)?.takeIf { it.isNotBlank() }
            ?: return
        val savedAt = prefs.getLong(PREF_PENDING_WORK_TIME, 0L)
        val tooOld = savedAt <= 0L || System.currentTimeMillis() - savedAt > PENDING_WORK_TTL_MS
        if (tooOld) {
            clearPersistedActiveWork()
            return
        }

        waitingForReply = true
        activeRequestIsDevelopment = prefs.getBoolean(PREF_PENDING_WORK_IS_DEVELOPMENT, true)
        pendingRequestPayload = payload
        pendingReconnectForActiveWork = true
        reconnectAttempts = 0
        setSendEnabled(false)
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
                when (intent.getStringExtra(TaskWorkService.EXTRA_KIND)) {
                    "connected" -> {
                        reconnectAttempts = 0
                        updateFirstConversationStatus("已连接 · 点击进入开发会话")
                        if (waitingForReply && pendingReconnectForActiveWork) {
                            pendingReconnectForActiveWork = false
                            recordEvidence("connection", "连接已恢复，后台任务继续运行")
                            appendMessage(ChatMessage("ai-cli-log", "连接已恢复 · 后台任务继续运行"))
                        }
                        if (!waitingForReply) setSendEnabled(true)
                    }
                    "disconnected" -> {
                        backendConnected = false
                        if (waitingForReply) {
                            if (!pendingReconnectForActiveWork) handleActiveWorkDisconnected()
                        } else {
                            updateFirstConversationStatus("未连接 · 点击重试")
                            setSendEnabled(true)
                        }
                    }
                    "message" -> {
                        intent.getStringExtra(TaskWorkService.EXTRA_RAW_MESSAGE)?.let { raw ->
                            appendMessage(raw)
                        }
                    }
                    "paused" -> {
                        backendConnected = false
                        setSendEnabled(true)
                    }
                }
            }
            TaskWorkService.ACTION_STATE -> {
                backendConnected = intent.getBooleanExtra(TaskWorkService.EXTRA_CONNECTED, backendConnected)
                val serviceWaiting = intent.getBooleanExtra(TaskWorkService.EXTRA_WAITING, waitingForReply)
                if (!serviceWaiting && !waitingForReply) {
                    setSendEnabled(backendConnected)
                }
            }
        }
    }

    private fun startTaskWorkService(
        action: String,
        payload: String? = null,
        isDevelopment: Boolean = activeRequestIsDevelopment
    ): Boolean {
        val intent = Intent(this, TaskWorkService::class.java).apply {
            this.action = action
            payload?.let { putExtra(TaskWorkService.EXTRA_PAYLOAD, it) }
            putExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, isDevelopment)
        }
        return runCatching {
            if (action == TaskWorkService.ACTION_START_WORK || action == TaskWorkService.ACTION_RESUME_PENDING) {
                ContextCompat.startForegroundService(this, intent)
            } else {
                startService(intent)
            }
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
                array.optString(index).takeIf { it.isNotBlank() }?.let { appendMessage(it) }
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
            arrayOf("编辑项目名称")
        } else {
            arrayOf("编辑项目名称", "删除项目")
        }

        AlertDialog.Builder(this)
            .setTitle(project.title)
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "编辑项目名称" -> showRenameProjectDialog(index)
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
        return waitingForReply &&
            index == activeConversationIndex &&
            index in conversations.indices &&
            !conversations[index].ended
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
        binding.projectSettingsButton.setOnClickListener { openSettings() }
        binding.profileSettingsButton.setOnClickListener { openSettings() }
        binding.profileCheckUpdateButton.setOnClickListener {
            AppUpdateManager(this).manualCheck()
        }
        binding.profileVersionText.text =
            "一龙 v${BuildConfig.VERSION_NAME}  (build ${BuildConfig.VERSION_CODE})"
    }

    private fun showMoreActions() {
        showChatActionPopup(binding.moreButton)
    }

    private fun showHomeActionPopup(anchor: View, tab: TextView) {
        val actions = if (tab == binding.tabProject) {
            listOf(
                TopAction("新建项目", R.drawable.ic_popup_project) { showCreateProjectDialog() },
                TopAction("项目记录", R.drawable.ic_popup_history) { showProjectRecordDialog() },
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

    private fun forwardMessageText(text: String) {
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        startActivity(Intent.createChooser(intent, "转发聊天内容"))
    }

    private fun shareableMessageText(message: ChatMessage): String {
        return buildString {
            append(message.content.trim())
            val details = message.evidenceDetails?.trim().orEmpty()
            if (details.isNotBlank()) {
                append("\n\n")
                message.evidenceTitle?.trim()?.takeIf { it.isNotBlank() }?.let {
                    append(it)
                    append('\n')
                }
                append(details)
            }
        }.trim()
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

    private fun progressStepLabel(content: String): String {
        return when {
            content.startsWith("CLI 工作区") -> "准备项目"
            content.startsWith("项目环境已准备好") -> "准备项目"
            content.startsWith("环境提醒") -> "环境检查"
            content.startsWith("正在启动本地 CLI") -> "启动助手"
            content.startsWith("开发助手已启动") -> "启动助手"
            content.startsWith("开发助手仍在运行") -> "处理中"
            content.startsWith("CLI 输出") -> "后台处理"
            content.startsWith("后台正在处理") -> "后台处理"
            content.startsWith("CLI 已结束") -> "检查结果"
            content.startsWith("开发处理已结束") -> "检查结果"
            content.startsWith("未找到 APK") -> "产物缺失"
            content.contains("下载链接") -> "生成下载"
            content.contains("APK") || content.contains("编译") -> "编译打包"
            else -> "进度更新"
        }
    }

    private fun initialWorkflowMessage(isDevelopment: Boolean): String {
        return if (isDevelopment) "正在思考" else "正在整理回复。"
    }

    private fun workflowProgressMessage(content: String): String {
        val progress = userFacingProgress(content.ifBlank { "正在推进当前任务。" })
        if (progress == "正在思考") return progress
        return "${progressStepLabel(progress)}：$progress"
    }

    private fun userFacingProgress(content: String): String {
        return when {
            content.startsWith("CLI 工作区") ->
                "项目环境已准备好，正在进入开发流程。"
            content.startsWith("正在启动本地 CLI") ->
                "开发助手已启动，正在处理你的需求。"
            content.startsWith("CLI 仍在运行") ->
                "正在思考"
            content.startsWith("CLI 已结束") ->
                "开发处理已结束，正在检查 APK 文件。"
            content.startsWith("未找到 APK") ->
                "暂时没有找到 APK 文件，正在判断是否需要继续处理。"
            content.startsWith("环境提醒") && content.contains("Codex CLI") ->
                "服务器开发助手配置需要检查，可能会影响本次开发。"
            content.startsWith("环境提醒") && content.contains("Android SDK") ->
                "服务器 Android 构建环境需要检查，可能会影响打包 APK。"
            content.startsWith("环境提醒") && content.contains("java", ignoreCase = true) ->
                "服务器 Java 环境需要检查，可能会影响打包 APK。"
            content.startsWith("CLI 输出") ->
                "后台正在处理项目，技术日志已收起。"
            else -> content
        }
    }

    private fun finalReplyMessage(content: String, apkUrl: String?, imageUrl: String?, wasDevelopment: Boolean): String {
        val cleanAsDevelopment = shouldCleanFinalAsDevelopment(content, wasDevelopment, apkUrl)
        val main = cleanFinalReplyForUser(content, cleanAsDevelopment, apkUrl).ifBlank {
            if (cleanAsDevelopment) "本轮开发任务已完成。" else "回复已完成。"
        }
        return buildString {
            append(main)
            if (wasDevelopment) apkUrl?.let { append("\n\n下载新 APK：$it") }
            imageUrl?.takeIf { !main.contains(it) }?.let { append("\n\n图片链接：$it") }
        }
    }

    private fun shouldCleanFinalAsDevelopment(content: String, wasDevelopment: Boolean, apkUrl: String?): Boolean {
        if (wasDevelopment || apkUrl != null) return true
        val lower = content.lowercase(Locale.CHINA)
        val strongSignals = listOf(
            "/root/workspaces/",
            "build/android/",
            "src/main/",
            "androidmanifest",
            "mainactivity.",
            ".java:",
            ".kt:",
            ".xml:",
            "gradle",
            "assemble",
            "apksigner",
            "aapt dump",
            "已处理：",
            "改动：",
            "验证情况：",
            "apk 已生成"
        )
        return strongSignals.any { lower.contains(it) }
    }

    private fun cleanFinalReplyForUser(content: String, wasDevelopment: Boolean, apkUrl: String?): String {
        if (!wasDevelopment) return content.trim()

        val cleanedLines = content
            .replace(Regex("\\[([^\\]]+)]\\s*\\(/root/workspaces/[^)]*\\)"), "$1")
            .replace(Regex("\\s*\\(/root/workspaces/[^)]*\\)"), "")
            .replace(Regex("/root/workspaces/\\S+"), "项目文件")
            .replace(Regex("\\[[^\\]]+\\.apk]\\([^)]*\\)"), "APK 已生成")
            .lineSequence()
            .map { sanitizeFinalReplyLine(it.trimEnd()) }
            .filterNot { line ->
                val lower = line.lowercase(Locale.CHINA)
                    line.contains("/root/workspaces/") ||
                    line.contains("build/android/") ||
                    isLeakedPlatformPromptLine(line) ||
                    line.startsWith("用户可见：") ||
                    line.startsWith("用户可见:") ||
                    lower.contains("apksigner") ||
                    lower.contains("aapt dump") ||
                    lower.contains("sha256") ||
                    lower.startsWith("下载链接：") ||
                    lower.startsWith("验证结果：")
            }
            .joinToString("\n")
            .replace(Regex("\n{3,}"), "\n\n")
            .trim()

        val result = if (apkUrl != null && cleanedLines.length > 520) {
            val usefulLines = cleanedLines
                .lineSequence()
                .filter { line ->
                    val trimmed = line.trim()
                    trimmed.isNotBlank() &&
                        !trimmed.startsWith("- `") &&
                        !trimmed.startsWith("已检查：")
                }
                .take(6)
                .joinToString("\n")
                .trim()
            usefulLines.ifBlank { "已完成并生成 APK。你可以先下载安装测试。" }
        } else {
            cleanedLines
        }

        return result
    }

    private fun sanitizeFinalReplyLine(line: String): String {
        return line
            .replace("已处理：", "已完成：")
            .replace(Regex("在\\s+[^\\s，。；：]+\\.(kt|java|xml)(:\\d+)?\\s*的"), "")
            .replace(Regex("`([^`]+)`"), "$1")
            .trimEnd()
    }

    private fun workflowStoppedMessage(reason: String, wasDevelopment: Boolean = activeRequestIsDevelopment): String {
        val stage = if (wasDevelopment) "需要处理" else "回复中断"
        return "工作停止：$stage。原因：$reason"
    }

    private fun friendlyErrorMessage(raw: String): String {
        val nestedMessage = nestedApiErrorMessage(raw)
        val source = listOf(raw, nestedMessage).joinToString(" ").lowercase(Locale.CHINA)
        return when {
            source.contains("free_quota_exhausted") ||
                source.contains("payment required") ||
                source.contains("endpoint is inactive") ->
                "当前选择的 AI 模型额度已用尽或接口不可用。请点右下角模型按钮切换可用模型，或联系管理员补充额度后重试。"
            source.contains("unauthorized") ||
                source.contains("invalid api key") ||
                source.contains("api key") && source.contains("invalid") ->
                "当前 AI 模型密钥无效或权限不足。请在 AI 设置里检查密钥，或切换到可用模型。"
            source.contains("rate limit") ||
                source.contains("too many requests") ||
                source.contains("429") ->
                "当前 AI 模型请求过于频繁。请稍后重试，或切换到其他可用模型。"
            source.contains("timeout") || source.contains("超时") ->
                "AI 请求超时了。请检查网络或稍后重试。"
            source.contains("connection") || source.contains("network") ->
                "连接 AI 服务失败。请检查网络、代理地址或稍后重试。"
            nestedMessage.isNotBlank() ->
                summarize(nestedMessage, 90)
            raw.isBlank() ->
                "AI 服务暂时不可用，请稍后重试。"
            else ->
                summarize(raw.replace(Regex("\\{.*"), "").trim().ifBlank { raw }, 90)
        }
    }

    private fun nestedApiErrorMessage(raw: String): String {
        val jsonStart = raw.indexOf('{')
        if (jsonStart < 0) return ""
        return runCatching {
            val root = JSONObject(raw.substring(jsonStart))
            val error = root.optJSONObject("error")
            error?.optString("message").orEmpty().ifBlank {
                root.optString("message").orEmpty()
            }
        }.getOrDefault("")
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

    private fun evidenceTitle(entries: List<EvidenceEntry>): String {
        val counts = entries.groupingBy { it.kind }.eachCount()
        val parts = mutableListOf<String>()
        counts["command"]?.let { parts.add("已运行 ${it} 条命令") }
        counts["file"]?.let { parts.add("已查看 ${it} 个文件") }
        counts["edit"]?.let { parts.add("已编辑 ${it} 次") }
        counts["build"]?.let { parts.add("构建记录 ${it} 条") }
        counts["cli"]?.let { parts.add("CLI 输出 ${it} 条") }
        counts["env"]?.let { parts.add("环境提示 ${it} 条") }
        counts["connection"]?.let { parts.add("连接事件 ${it} 条") }
        counts["result"]?.let { parts.add("结果 ${it} 条") }
        counts["progress"]?.let { parts.add("进度 ${it} 条") }
        return parts.take(3).joinToString(" · ").ifBlank { "已收起 ${entries.size} 条后台记录" }
    }

    private fun evidenceDetails(entries: List<EvidenceEntry>): String {
        return entries.takeLast(24).joinToString("\n") {
            "· ${evidenceKindLabel(it.kind)}：${it.text}"
        }
    }

    private fun evidenceKindLabel(kind: String): String {
        return when (kind) {
            "command" -> "命令"
            "file" -> "文件"
            "edit" -> "编辑"
            "build" -> "构建"
            "cli" -> "CLI"
            "env" -> "环境"
            "connection" -> "连接"
            "result" -> "结果"
            else -> "进度"
        }
    }

    private fun sanitizeEvidenceDetail(detail: String): String {
        val cleaned = detail
            .replace(Regex("\\[([^\\]]+)]\\s*\\(/root/workspaces/[^)]*\\)"), "$1")
            .replace(Regex("\\s*\\(/root/workspaces/[^)]*\\)"), "")
            .replace(Regex("/root/workspaces/\\S+"), "项目文件")
            .replace("用户可见：", "")
            .replace("用户可见:", "")
            .replace(Regex("\\s+"), " ")
            .trim()

        if (cleaned.isBlank()) return ""
        if (isLeakedPlatformPromptMessage(cleaned) || isTechnicalLeakMessage(cleaned)) return ""
        val lower = cleaned.lowercase(Locale.CHINA)
        val noisy = listOf(
            "tokens used",
            "feedback_tags",
            "codex_analytics",
            "original token count",
            "reading additional input",
            "openai codex v",
            "session id:",
            "auth_header"
        )
        if (noisy.any { lower.contains(it) }) return ""
        return cleaned
    }

    private fun evidenceKindForCliCategory(category: String): String {
        return when (category) {
            "编译打包" -> "build"
            "执行命令" -> "command"
            "环境提示" -> "env"
            "模型回复" -> "cli"
            else -> "cli"
        }
    }

    private fun toolEvidenceKind(tool: String): String {
        return when (tool) {
            "read_file", "list_dir" -> "file"
            "write_file", "init_project" -> "edit"
            "build_project" -> "build"
            "run_shell", "git_commit" -> "command"
            else -> "progress"
        }
    }

    private fun isCliOutputProgress(content: String): Boolean {
        return content.contains("CLI 输出(") ||
            content.contains("CLI 输出(stdout)") ||
            content.contains("CLI 输出(stderr)")
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

    private fun cleanCliOutputLine(content: String): String {
        val rawLine = content
            .lineSequence()
            .firstOrNull { it.contains("CLI 输出(") }
            ?: content
        return rawLine
            .substringAfter("):", rawLine)
            .replace("（输出过长，已截断）", "")
            .trim()
    }

    private fun cliOutputCategory(line: String): String {
        val lower = line.lowercase(Locale.CHINA)
        return when {
            lower.contains("gradle") || lower.contains("assemble") || line.contains("APK") -> "编译打包"
            lower.contains("/bin/bash") || lower == "exec" || lower.contains("succeeded in") ||
                lower.contains("process exited") || lower.contains("wall time") -> "执行命令"
            lower.contains("warn") || lower.contains("error") || lower.contains("failed") ||
                line.contains("未检测") || line.contains("失败") -> "环境提示"
            looksLikeAssistantCliLine(line) -> "模型回复"
            else -> "后台输出"
        }
    }

    private fun looksLikeAssistantCliLine(line: String): Boolean {
        if (line.length !in 8..220) return false
        if (line.any { it in '\u4e00'..'\u9fff' }) {
            val lower = line.lowercase(Locale.CHINA)
            return !lower.contains("mcp_server") &&
                !lower.contains("event.timestamp") &&
                !lower.contains("feedback_tags")
        }
        return false
    }

    private fun shouldKeepCliSample(line: String): Boolean {
        if (line.isBlank()) return false
        val lower = line.lowercase(Locale.CHINA)
        if (line in setOf("codex", "exec", "user", "tokens used", "Output:")) return false
        val noisy = listOf(
            "feedback_tags",
            "model_client.",
            "responses_websocket",
            "mcp_server=",
            "event.timestamp=",
            "original token count",
            "reading additional input",
            "openai codex v",
            "session id:",
            "auth_header",
            "codex_analytics",
            "plugins/featured",
            "plugins/installed",
            "</html>"
        )
        return noisy.none { lower.contains(it) }
    }

    private fun compactCliTranscriptMessages(messages: MutableList<ChatMessage>) {
        if (messages.none { it.role == "ai-progress" && isCliOutputProgress(it.content) }) return

        val compacted = mutableListOf<ChatMessage>()
        var count = 0
        val categories = linkedMapOf<String, Int>()

        fun flushCliLog() {
            if (count == 0) return
            compacted.add(ChatMessage("ai-cli-log", genericFoldedCliLogSummary(categories)))
            count = 0
            categories.clear()
        }

        for (message in messages) {
            if (message.role == "ai-progress" && isCliOutputProgress(message.content)) {
                count += 1
                val line = cleanCliOutputLine(message.content)
                val category = cliOutputCategory(line)
                categories[category] = (categories[category] ?: 0) + 1
            } else {
                flushCliLog()
                compacted.add(message)
            }
        }
        flushCliLog()

        messages.clear()
        messages.addAll(compacted)
    }

    private fun sanitizeExistingCliLogMessages(messages: MutableList<ChatMessage>) {
        for (index in messages.indices) {
            val message = messages[index]
            if (message.role == "ai-cli-log") {
                messages[index] = ChatMessage("ai-cli-log", genericFoldedCliLogSummary())
            } else if (message.role == "ai-progress") {
                messages[index] = ChatMessage("ai-progress", sanitizeStoredProgressMessage(message.content))
            }
        }
    }

    private fun removeLeakedAndRoutineWorkflowMessages(messages: MutableList<ChatMessage>) {
        messages.removeAll { message ->
            isLeakedPlatformPromptMessage(message.content) ||
                isTechnicalLeakMessage(message.content) ||
                (message.role in workflowHistoryStatusRoles && isRoutineWorkflowMessage(message.content))
        }
    }

    private fun sanitizeExistingUserVisibleMessages(messages: MutableList<ChatMessage>) {
        val roles = setOf("ai", "ai-intent")
        messages.indices.forEach { index ->
            val message = messages[index]
            if (message.role !in roles) return@forEach
            if (!shouldCleanFinalAsDevelopment(message.content, wasDevelopment = false, apkUrl = null)) return@forEach
            val cleaned = cleanFinalReplyForUser(message.content, wasDevelopment = true, apkUrl = null)
            messages[index] = if (cleaned.isBlank()) {
                message.copy(content = "本轮开发任务已完成。")
            } else {
                message.copy(content = cleaned)
            }
        }
    }

    private fun isRoutineWorkflowMessage(content: String): Boolean {
        val trimmed = content.trim()
        return trimmed == "正在思考" ||
            trimmed.startsWith("启动助手：") ||
            trimmed.startsWith("准备项目：") ||
            trimmed.startsWith("处理中：") ||
            trimmed.startsWith("检查结果：开发处理已结束") ||
            trimmed.contains("开发助手已启动，正在处理你的需求") ||
            trimmed.contains("项目环境已准备好，正在进入开发流程") ||
            trimmed.contains("开发助手仍在运行")
    }

    private fun isLeakedPlatformPromptMessage(content: String): Boolean {
        return content
            .lineSequence()
            .map { it.trim() }
            .any { isLeakedPlatformPromptLine(it) }
    }

    private fun isTechnicalLeakMessage(content: String): Boolean {
        val lower = content.lowercase(Locale.CHINA)
        return lower.contains("rmcp::") ||
            lower.contains("worker quit with fatal") ||
            lower.contains("http request failed") ||
            lower.contains("client error:") ||
            lower.contains("event.timestamp=") ||
            lower.contains("mcp_server=")
    }

    private fun isLeakedPlatformPromptLine(line: String): Boolean {
        return line.contains("你是「一龙」平台服务器上的本地 AI CLI 编程助手") ||
            line.startsWith("当前 CLI") ||
            line.startsWith("当前工作目录") ||
            line.contains("用户隔离工作区") ||
            line.contains("不要使用固定模板") ||
            line.contains("不要提“CLI/后台/工作区”") ||
            line.startsWith("请直接处理用户请求") ||
            line.startsWith("用户请求：")
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

    private fun sanitizeStoredProgressMessage(content: String): String {
        val cleaned = content
            .replace("启动 CLI", "启动助手")
            .replace("准备工作区", "准备项目")
        return when {
            content.contains("CLI 工作区已准备") ->
                cleaned.replaceAfter("\n", "项目环境已准备好，正在进入开发流程。")
            content.contains("正在启动本地 CLI") ->
                cleaned.replaceAfter("\n", "开发助手已启动，正在处理你的需求。")
            content.contains("CLI 输出") ->
                genericFoldedCliLogSummary()
            else -> cleaned
        }
    }

    private fun genericFoldedCliLogSummary(categories: Map<String, Int> = emptyMap()): String {
        val mainWork = categories.entries.maxByOrNull { it.value }?.key
        val friendly = when (mainWork) {
            "编译打包" -> "正在编译 APK"
            "执行命令" -> "正在检查项目文件"
            "环境提示" -> "环境提示已归类"
            "模型回复" -> "正在整理下一步"
            else -> "后台正在处理项目"
        }
        val count = categories.values.sum()
        val suffix = if (count > 0) "（${count} 条）" else ""
        return "后台开发日志已收起$suffix · $friendly"
    }

    private fun maybeAppendVisibleCliSignal(category: String, line: String) {
        if (!activeRequestIsDevelopment) return
        if (category != "模型回复") return
        val signal = visibleCliSignal(category, line) ?: return
        val (key, message) = signal
        if (!emittedProgressSignals.add(key)) return
        finalizeEvidenceForLatestAssistant()
        visibleAssistantUpdateCount += 1
        appendMessage(ChatMessage("ai-intent", message))
        attachEvidenceToLatestAi()
    }

    private fun visibleCliSignal(category: String, line: String): Pair<String, String>? {
        val lower = line.lowercase(Locale.CHINA)
        val clean = userSafeCliLine(line)
        return when (category) {
            "模型回复" -> {
                val userVisible = extractUserVisibleCliMessage(clean) ?: return null
                if (!shouldExposeAssistantCliLine(userVisible)) return null
                "assistant:${userVisible.take(64)}" to userVisible
            }
            "编译打包" -> when {
                lower.contains("build successful") ->
                    "build_success" to "编译结果：APK 构建通过，正在查找可安装文件。"
                lower.contains("build failed") ->
                    "build_failed" to "编译结果：打包失败。我会先根据失败原因继续修复。"
                lower.contains("exception") || lower.contains("error") || lower.contains("failed") ->
                    "build_issue:${clean.take(48)}" to "编译遇到问题：$clean\n我会优先判断是代码、依赖还是构建配置导致。"
                lower.contains("assemble") || lower.contains("gradle") || line.contains("APK") ->
                    "build_started" to "正在进入 APK 编译检查。接下来会看构建是否能通过，以及失败点是否需要继续修。"
                else -> null
            }
            "环境提示" -> when {
                lower.contains("java") ->
                    "env_java" to "环境检查：服务器 Java 环境可能影响打包，我会先确认它是不是本次失败原因。"
                lower.contains("android sdk") || line.contains("Android SDK") ->
                    "env_android_sdk" to "环境检查：Android SDK 配置可能影响 APK 构建，我会优先确认构建环境。"
                lower.contains("gradle") ->
                    "env_gradle" to "环境检查：Gradle 构建环境有提示，我会判断它是否需要修复。"
                else -> null
            }
            else -> null
        }
    }

    private fun shouldExposeAssistantCliLine(line: String): Boolean {
        if (visibleAssistantUpdateCount >= 3) return false
        if (!shouldKeepCliSample(line)) return false
        if (line.length !in 10..260) return false
        val lower = line.lowercase(Locale.CHINA)
        val finalLike = listOf(
            "已完成",
            "完成了",
            "apk 已生成",
            "下载链接",
            "本轮",
            "验证结果",
            "改动："
        )
        if (finalLike.any { lower.contains(it) }) return false
        val technical = listOf(
            "```",
            "/root/",
            "/home/",
            "http://",
            "https://",
            "build.gradle",
            "androidmanifest",
            ".kt",
            ".xml",
            "gradle",
            "assemble",
            "tokens used",
            "不要使用固定模板",
            "不要提",
            "cli/后台/工作区",
            "工作区"
        )
        return technical.none { lower.contains(it) }
    }

    private fun extractUserVisibleCliMessage(line: String): String? {
        val trimmed = line.trim()
        val marker = when {
            trimmed.startsWith("用户可见：") -> "用户可见："
            trimmed.startsWith("用户可见:") -> "用户可见:"
            else -> return null
        }
        return trimmed.substringAfter(marker).trim().takeIf { it.isNotBlank() }
    }

    private fun userSafeCliLine(line: String): String {
        return summarize(
            line
                .replace(Regex("\\s+"), " ")
                .trim(),
            120
        )
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

    private fun isCliProjectEvent(line: String): Boolean {
        return line.contains("CLI 输出(") ||
            line.contains("CLI 工作区") ||
            line.contains("正在启动本地 CLI") ||
            line.contains("CLI 日志已折叠") ||
            line.contains("CLI 日志已归类") ||
            line.contains("CLI 运行日志已折叠")
    }

    private fun closeStaleWorkflowMessages(messages: MutableList<ChatMessage>) {
        val lastRole = messages.lastOrNull()?.role ?: return
        if (lastRole !in staleWorkflowRoles) return
        messages.add(
            ChatMessage(
                "ai-stopped",
                "工作停止：没有收到服务器进度\n原因：上次请求只停在本地发送或后台日志阶段，未收到最终回包。\n下一步：请重新发送需求，我会重新连接服务器执行。"
            )
        )
    }

    private fun scheduleFirstServerResponseWatchdog(token: Int) {
        binding.root.postDelayed({
            if (!waitingForReply || serverResponseToken != token) return@postDelayed
            pendingReconnectForActiveWork = true
            if (activeRequestIsDevelopment) {
                updateStage(currentStage, "暂时没有收到服务器进度，正在自动恢复连接。")
                addProjectEvent("服务端暂未返回进度，自动恢复连接")
                recordEvidence("connection", "暂时没有收到服务器进度，正在自动恢复连接")
            }
            appendMessage(ChatMessage("ai-cli-log", "暂时没有收到服务器进度 · 正在自动恢复"))
            startTaskWorkService(TaskWorkService.ACTION_RESUME_PENDING)
        }, 20_000L)
    }

    private fun appendMessage(raw: String) {
        serverResponseToken += 1
        // 解析服务端推送的 JSON 消息
        try {
            val json = com.google.gson.JsonParser.parseString(raw).asJsonObject
            val type = jsonStringOrNull(json, "type") ?: return
            val msg = when (type) {
                "progress"    -> {
                    val content = jsonStringOrNull(json, "message") ?: ""
                    if (isCliOutputProgress(content)) {
                        handleFoldedCliOutput(content)
                        return
                    }
                    handleProgress(content)
                    if (shouldShowProgressBubble(content)) {
                        ChatMessage("ai-progress", workflowProgressMessage(content))
                    } else {
                        return
                    }
                }
                "tool_call"   -> {
                    val tool = jsonStringOrNull(json, "tool") ?: "工具"
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

    private fun shouldShowProgressBubble(content: String): Boolean {
        val progress = userFacingProgress(content)
        return !isRoutineWorkflowMessage(workflowProgressMessage(content)) &&
            !isRoutineWorkflowMessage(progress) &&
            (content.startsWith("环境提醒") ||
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
        if (waitingForReply && binding.chatPage.visibility == View.VISIBLE) {
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
            if (token != stageHintShimmerToken || !waitingForReply || binding.chatPage.visibility != View.VISIBLE) {
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
        binding.userInfoText.text = "我的开发工作台\n用户 ID：${summarize(userId, 18)}\n服务端模型由管理员统一配置，当前项目记录保存在本机。"

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
        updateStageLines()
        renderConversationList()
        if (binding.projectPage.visibility == View.VISIBLE) {
            renderProjectList()
        }
        updateStageHintShimmer()
    }

    private fun updateStageLines() {
        val active = when (currentStage) {
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
        const val PENDING_WORK_TTL_MS = 6 * 60 * 60 * 1000L
        const val MAX_ATTACHMENT_BYTES = 8 * 1024 * 1024
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

    private fun summarize(text: String, maxLength: Int): String {
        val normalized = text.replace('\n', ' ').trim()
        if (normalized.length <= maxLength) return normalized
        return normalized.take(maxLength - 1) + "…"
    }

    private fun toolLabel(tool: String): String = when (tool) {
        "init_project" -> "初始化项目"
        "read_file" -> "读取文件"
        "write_file" -> "写入代码"
        "list_dir" -> "查看目录"
        "run_shell" -> "执行命令"
        "git_commit" -> "保存版本"
        "build_project" -> "编译项目"
        else -> tool
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

    private fun looksLikeDevelopmentRequest(text: String): Boolean {
        val lower = text.lowercase(Locale.CHINA)
        val directWords = listOf(
            "app", "apk", "android", "应用", "功能", "页面", "界面", "按钮", "代码", "开发",
            "修改", "添加", "新增", "生成", "做一个", "做个", "编译", "打包", "安装", "发布",
            "登录", "注册", "首页", "设置", "接口", "后端", "服务端", "数据库", "继续", "项目"
        )
        if (directWords.any { lower.contains(it) }) return true

        val actionWords = listOf(
            "改", "改成", "修改", "调整", "优化", "美化", "添加", "新增", "增加", "加上",
            "删掉", "删除", "去掉", "替换", "做成", "变成", "接入", "修复", "处理"
        )
        val uiWords = listOf(
            "点击", "屏幕", "中间", "文字", "字体", "动画", "闪烁", "按钮", "菜单",
            "页面", "界面", "弹窗", "提示", "显示", "隐藏", "颜色", "图标", "布局",
            "输入框", "底部", "顶部", "气泡", "回复", "折叠"
        )
        return actionWords.any { lower.contains(it) } && uiWords.any { lower.contains(it) }
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

    private fun looksLikeResumeCommand(normalized: String): Boolean {
        if (normalized in setOf(
                "继续",
                "继续吧",
                "继续开发",
                "继续做",
                "继续完成",
                "重试",
                "再试一次",
                "重新开始",
                "再来一次"
            )
        ) {
            return true
        }
        return (normalized.contains("继续") || normalized.contains("重试") || normalized.contains("再试")) &&
            (normalized.contains("上一次") ||
                normalized.contains("未完成") ||
                normalized.contains("当前项目的开发") ||
                normalized.contains("当前进度"))
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

    private fun looksLikeApkDeliveryRequest(text: String): Boolean {
        val lower = text.lowercase(Locale.CHINA)
        val asksForApk = lower.contains("apk") || lower.contains("安装包") || lower.contains("下载包")
        val asksForDelivery = listOf("地址", "链接", "下载", "发给我", "给我", "做好", "做完", "完成")
            .any { lower.contains(it) }
        return asksForApk && asksForDelivery
    }

    private fun looksLikeDirectImageRequest(text: String): Boolean {
        val lower = text.lowercase(Locale.CHINA)
        val appWords = listOf(
            "app", "apk", "android", "应用", "功能", "页面", "界面", "按钮", "代码", "开发",
            "修改", "添加", "新增", "编译", "打包", "安装", "发布", "登录", "注册", "首页",
            "设置", "接口", "后端", "服务端", "数据库", "项目"
        )
        if (appWords.any { lower.contains(it) }) return false

        val imageWords = listOf("文生图", "生图", "生成图", "图像", "图片", "壁纸", "照片", "头像", "插画", "海报", "卡通", "山水画")
        val intentWords = listOf("文生图", "生图", "生成", "画", "绘制", "做一张", "来一张", "出一张", "创作")
        return imageWords.any { lower.contains(it) } && intentWords.any { lower.contains(it) }
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
                Toast.makeText(this, "需要通知权限才能显示桌面任务角标", Toast.LENGTH_SHORT).show()
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
