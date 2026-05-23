package com.elon.app

import android.Manifest
import android.annotation.SuppressLint
import android.content.Intent
import android.animation.ValueAnimator
import android.content.Context
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
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID
import kotlin.math.sin

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var wsClient: ElonWsClient
    private lateinit var chatAdapter: ChatAdapter
    private var waitingForReply = false
    private var activeRequestIsDevelopment = false
    private var workflowStepIndex = 0
    private var foldedCliLogCount = 0
    private val foldedCliLogSamples = ArrayDeque<String>()
    private val foldedCliLogCategories = linkedMapOf<String, Int>()
    private val currentEvidenceEntries = mutableListOf<EvidenceEntry>()
    private val emittedNarrationMilestones = linkedSetOf<String>()
    private var serverResponseToken = 0
    private var appInForeground = false
    private var pendingRequestPayload: String? = null
    private var pendingReconnectForActiveWork = false
    private var reconnectAttempts = 0
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
    private var exitConfirmDialog: AlertDialog? = null
    private var actionPopup: PopupWindow? = null
    private lateinit var inputModeButton: ImageButton
    private lateinit var attachmentButton: ImageButton
    private lateinit var voiceHoldButton: TextView
    private lateinit var attachmentPanel: LinearLayout
    private var attachmentPanelOpen = false
    private var voiceMode = false
    private var inputCanSend = true
    private var speechRecognizer: SpeechRecognizer? = null
    private var isListeningForSpeech = false
    private val speechPermissionRequest = 4301

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

        loadProjects()

        chatAdapter = ChatAdapter(activeConversation().messages, ::pauseCurrentWork)
        binding.chatList.adapter = chatAdapter
        setupNavigation()
        setupQuickActions()
        setupBackHandling()
        setupInputComposer()
        updateProjectViews("像聊天一样发需求，我会同步整理开发进度和项目记录。")

        // 连接 WebSocket
        wsClient = ElonWsClient(
            serverUrl = "ws://43.139.149.158:8080/ws",
            onMessage = { msg -> runOnUiThread { appendMessage(msg) } },
            onConnected = {
                runOnUiThread {
                    reconnectAttempts = 0
                    updateFirstConversationStatus("已连接 · 点击进入开发会话")
                    if (waitingForReply && pendingReconnectForActiveWork) {
                        resumePendingWorkAfterReconnect()
                        return@runOnUiThread
                    }
                    if (!waitingForReply) setSendEnabled(true)
                }
            },
            onDisconnected = {
                runOnUiThread {
                    updateFirstConversationStatus("未连接 · 点击重试")
                    if (waitingForReply) {
                        handleActiveWorkDisconnected()
                        return@runOnUiThread
                    }
                    setSendEnabled(true)
                }
            }
        )
        wsClient.connect()

        // 重连按钮
        binding.statusText.setOnClickListener {
            if (wsClient.isConnected()) openConversation(0)
            else wsClient.connect()
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
    }

    override fun onResume() {
        super.onResume()
        appInForeground = true
        if (::binding.isInitialized) {
            loadModelOptions()
            if (::wsClient.isInitialized && !wsClient.isConnected()) {
                if (waitingForReply) {
                    pendingReconnectForActiveWork = true
                    updateStage(currentStage, "正在恢复连接，回来后会自动继续本轮任务。")
                    recordEvidence("connection", "连接恢复中，正在继续上次任务")
                    appendMessage(ChatMessage("ai-cli-log", "连接恢复中 · 正在继续上次任务"))
                }
                wsClient.connect()
            }
        }
    }

    override fun onStop() {
        appInForeground = false
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

        if (!wsClient.isConnected()) {
            appendMessage(ChatMessage("error", "还没有连接到服务器，请点击上方状态栏重试。"))
            updateFirstConversationStatus("未连接 · 点击重试")
            wsClient.connect()
            return
        }

        val payload = com.google.gson.JsonObject().apply {
            addProperty("user_id", userId)
            addProperty("project_id", activeProject().id)
            addProperty("message", outgoingText)
            selectedAgentName?.let { addProperty("agent", it) }
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
        workflowStepIndex = 0
        resetFoldedCliLog()
        currentEvidenceEntries.clear()
        emittedNarrationMilestones.clear()
        if (activeRequestIsDevelopment) {
            updateProjectTitleFromRequest(text)
            saveProjectTitle()
            addProjectEvent("提交需求：${summarize(text, 36)}")
            updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
            appendMessage(ChatMessage("ai", initialNarrationMessage(outgoingText)))
        } else {
            updateProjectViews("普通消息已发送，开发项目记录保持不变。")
        }
        appendMessage(ChatMessage("ai-working", initialWorkflowMessage(activeRequestIsDevelopment, outgoingText)))

        // 通过 WebSocket 发送 JSON（包含 user_id，服务端据此隔离工作区）
        val responseToken = ++serverResponseToken
        if (!wsClient.send(payload.toString())) {
            waitingForReply = false
            setSendEnabled(true)
            pendingRequestPayload = null
            pendingReconnectForActiveWork = false
            val wasDevelopment = activeRequestIsDevelopment
            if (wasDevelopment) {
                updateStage("需要处理", "消息发送失败，请检查网络后重试。")
            }
            appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("消息发送失败，请检查网络后重试。", wasDevelopment)))
            activeRequestIsDevelopment = false
        } else {
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
        currentEvidenceEntries.clear()
        setSendEnabled(true)
        if (wasDevelopment) {
            updateStage("工作暂停", "你已暂停当前任务，可以调整需求后继续发送。")
            addProjectEvent("暂停当前工作")
        } else {
            updateProjectViews("当前回复已暂停，你可以继续输入新的消息。")
        }
        appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("你已暂停当前工作。", wasDevelopment)))
        if (::wsClient.isInitialized) {
            wsClient.disconnect()
            binding.root.postDelayed({ wsClient.connect() }, 500)
        }
    }

    private fun handleActiveWorkDisconnected() {
        pendingReconnectForActiveWork = true
        setSendEnabled(false)
        updateFirstConversationStatus("连接恢复中 · 回来后继续")
        if (activeRequestIsDevelopment) {
            updateStage(currentStage, "连接暂时断开，正在保留本轮任务并准备自动恢复。")
            recordEvidence("connection", "连接暂时断开，正在自动恢复任务")
        }
        appendMessage(ChatMessage("ai-cli-log", "连接暂时断开 · 正在自动恢复任务"))

        if (appInForeground) {
            scheduleReconnectForActiveWork()
        }
    }

    private fun scheduleReconnectForActiveWork() {
        if (!waitingForReply || !pendingReconnectForActiveWork) return
        reconnectAttempts += 1
        val delay = (800L * reconnectAttempts).coerceAtMost(5_000L)
        binding.root.postDelayed({
            if (!waitingForReply || !pendingReconnectForActiveWork || wsClient.isConnected()) return@postDelayed
            wsClient.connect()
        }, delay)
    }

    private fun resumePendingWorkAfterReconnect() {
        val payload = pendingRequestPayload
        if (payload.isNullOrBlank()) {
            pendingReconnectForActiveWork = false
            waitingForReply = false
            activeRequestIsDevelopment = false
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
        if (!wsClient.send(payload)) {
            pendingReconnectForActiveWork = true
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

        val inputBar = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(62)
            )
            gravity = Gravity.CENTER_VERTICAL
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

        val center = FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                dp(40),
                1f
            ).apply {
                marginStart = dp(4)
                marginEnd = dp(4)
            }
            setBackgroundResource(R.drawable.bg_input_pill)
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
            maxLines = 2
            setPadding(dp(14), dp(6), dp(8), dp(6))
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

        center.addView(inputEdit)
        center.addView(voiceHoldButton)
        center.addView(modelButton)

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

        inputBar.addView(inputModeButton)
        inputBar.addView(center)
        rightControls.addView(attachmentButton)
        rightControls.addView(sendButton)
        inputBar.addView(rightControls)

        attachmentPanel = buildAttachmentPanel()
        root.addView(inputBar)
        root.addView(attachmentPanel)

        inputEdit.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                updateSendButtonVisual()
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
    }

    private fun buildAttachmentPanel(): LinearLayout {
        return LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(104)
            )
            setBackgroundColor(Color.parseColor("#222222"))
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(10), dp(8), dp(10), dp(8))
            visibility = View.GONE

            addView(createAttachmentAction("相机", R.drawable.ic_attach_camera) {
                Toast.makeText(this@MainActivity, "相机入口准备中", Toast.LENGTH_SHORT).show()
            })
            addView(createAttachmentAction("相册", R.drawable.ic_attach_photos) {
                Toast.makeText(this@MainActivity, "相册入口准备中", Toast.LENGTH_SHORT).show()
            })
            addView(createAttachmentAction("文档", R.drawable.ic_attach_files) {
                Toast.makeText(this@MainActivity, "文档入口准备中", Toast.LENGTH_SHORT).show()
            })
            addView(createAttachmentAction("功能", R.drawable.ic_attach_function, false) {
                collapseAttachmentPanel()
                showChatActionPopup(binding.moreButton)
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
                if (addEndMargin) marginEnd = dp(6)
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
                layoutParams = LinearLayout.LayoutParams(dp(26), dp(26))
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
        attachmentPanelOpen = true
        attachmentPanel.visibility = View.VISIBLE
    }

    private fun collapseAttachmentPanel() {
        if (!::attachmentPanel.isInitialized) return
        attachmentPanelOpen = false
        attachmentPanel.visibility = View.GONE
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
        binding.topTitleText.text = activeConversation().title
        binding.topTitleText.setOnLongClickListener {
            showConversationActions(activeConversationIndex)
            true
        }
        setSendEnabled(wsClient.isConnected() && !waitingForReply)
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

                val useAgent = config.optString("use_agent", "").ifBlank { null }
                val customModel = config.optString("model", "")
                val customBase = config.optString("api_base", "")
                val label = when {
                    customModel.isNotBlank() || customBase.isNotBlank() -> "自定义模型"
                    useAgent != null -> options.firstOrNull { it.agentName == useAgent }?.label ?: useAgent
                    else -> "服务器默认"
                }

                runOnUiThread {
                    modelOptions = options
                    selectedAgentName = useAgent
                    currentModelLabel = label
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
        chatAdapter = ChatAdapter(activeConversation().messages, ::pauseCurrentWork)
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

    private fun normalizeProject(project: AppProject) {
        if (project.conversations.isEmpty()) project.conversations.add(createDefaultConversation())
        project.conversations.forEach {
            if (it.messages.isEmpty()) it.messages.add(welcomeMessage())
            compactCliTranscriptMessages(it.messages)
            sanitizeExistingCliLogMessages(it.messages)
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
            "ai", "ai-working", "ai-progress", "ai-tool", "ai-complete", "ai-stopped", "error" -> {
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
        if (binding.conversationPage.visibility == View.VISIBLE && binding.chatPage.visibility != View.VISIBLE) {
            binding.topTitleText.text = compactProjectTitle()
        }

        val first = conversations[0]
        binding.projectStatusText.text = first.title
        binding.statusText.text = first.subtitle
        binding.statusText.setTextColor(conversationSubtitleColor(first.subtitle))
        binding.conversationTimeText.text = timeFormatter.format(Date(first.updatedAt))

        while (binding.conversationPage.childCount > 1) {
            binding.conversationPage.removeViewAt(1)
        }
        for (index in 1 until conversations.size) {
            binding.conversationPage.addView(createConversationDivider())
            binding.conversationPage.addView(createConversationRow(index, conversations[index]))
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

    private fun createConversationRow(index: Int, conversation: AppConversation): View {
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
        return row
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

    private fun createPopupArrowView(): View {
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.parseColor("#3D3D3D")
            style = Paint.Style.FILL
        }
        return object : View(this) {
            override fun onDraw(canvas: Canvas) {
                super.onDraw(canvas)
                val path = Path().apply {
                    moveTo(width / 2f, 0f)
                    lineTo(width.toFloat(), height.toFloat())
                    lineTo(0f, height.toFloat())
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

    private fun initialWorkflowMessage(isDevelopment: Boolean, requestText: String = ""): String {
        return if (isDevelopment) {
            if (looksLikeApkDeliveryRequest(requestText)) {
                "检查 APK 产物中。"
            } else {
                "确认项目状态中。"
            }
        } else {
            "正在整理回复。"
        }
    }

    private fun initialNarrationMessage(requestText: String): String {
        if (looksLikeApkDeliveryRequest(requestText)) {
            return "我先检查当前项目有没有已经生成的 APK。\n\n如果有，我直接把下载地址给你；如果没有，我会继续打包，并告诉你是缺文件、编译失败，还是还在处理中。"
        }

        val lower = requestText.lowercase(Locale.CHINA)
        val goal = when {
            lower.contains("hello") || requestText.contains("欢迎") ->
                "目标很清楚：做一个打开后显示指定文字的最小 Android APK。"
            requestText.contains("图片") || requestText.contains("相册") ->
                "我先把它当作图片类 Android App 来处理，重点看图片选择、保存和隐私保护这些核心路径。"
            requestText.contains("简单") ->
                "我会先按最小可运行 APK 来做，避免加太多不必要的功能。"
            else ->
                "我先把你的描述整理成一个 Android 开发任务，再看当前项目应该从哪里继续。"
        }

        return "$goal\n\n接下来我会按 Codex 这种节奏推进：先确认项目结构，再实现关键功能，最后编译 APK 并给出可下载结果。后台命令和 CLI 日志会折叠成灰色提示，不占用聊天正文。"
    }

    private fun workflowProgressMessage(content: String): String {
        val progress = userFacingProgress(content.ifBlank { "正在推进当前任务。" })
        return "${progressStepLabel(progress)}：$progress"
    }

    private fun userFacingProgress(content: String): String {
        return when {
            content.startsWith("CLI 工作区") ->
                "项目环境已准备好，正在进入开发流程。"
            content.startsWith("正在启动本地 CLI") ->
                "开发助手已启动，正在处理你的需求。"
            content.startsWith("CLI 仍在运行") ->
                "开发助手仍在运行，正在等待模型或编译结果。"
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

    private fun toolCallWorkflowMessage(tool: String): String {
        return "后台动作：${toolLabel(tool)}。${toolWorkflowDoing(tool)}"
    }

    private fun toolResultWorkflowMessage(tool: String): String {
        return "已完成：${toolLabel(tool)}。正在判断下一步。"
    }

    private fun finalReplyMessage(content: String, apkUrl: String?, imageUrl: String?, wasDevelopment: Boolean): String {
        val main = cleanFinalReplyForUser(content, wasDevelopment, apkUrl).ifBlank {
            if (wasDevelopment) "本轮开发任务已完成。" else "回复已完成。"
        }
        return buildString {
            append(main)
            apkUrl?.let { append("\n\n下载新 APK：$it") }
            imageUrl?.takeIf { !main.contains(it) }?.let { append("\n\n图片链接：$it") }
        }
    }

    private fun cleanFinalReplyForUser(content: String, wasDevelopment: Boolean, apkUrl: String?): String {
        if (!wasDevelopment) return content.trim()

        val cleanedLines = content
            .replace(Regex("\\[[^\\]]+\\.apk]\\([^)]*\\)"), "APK 已生成")
            .lineSequence()
            .map { it.trimEnd() }
            .filterNot { line ->
                val lower = line.lowercase(Locale.CHINA)
                line.contains("/root/workspaces/") ||
                    line.contains("build/android/") ||
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
        val clean = detail
            .replace(Regex("\\s+"), " ")
            .trim()
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
        val index = messages.indexOfLast { it.role == "ai" }
        if (index < 0) return

        applyEvidenceToMessage(messages[index], currentEvidenceEntries)
        chatAdapter.notifyMessageUpdated(index)
        saveConversations()
    }

    private fun aiMessageWithCurrentEvidence(content: String): ChatMessage {
        val message = ChatMessage("ai", content)
        if (currentEvidenceEntries.isNotEmpty()) {
            applyEvidenceToMessage(message, currentEvidenceEntries)
            currentEvidenceEntries.clear()
        }
        return message
    }

    private fun applyEvidenceToMessage(message: ChatMessage, entries: List<EvidenceEntry>) {
        message.evidenceTitle = evidenceTitle(entries)
        message.evidenceDetails = evidenceDetails(entries)
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
        maybeAppendNarrationForCliCategory(category)
        recordEvidence(evidenceKindForCliCategory(category), line)
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

    private fun appendNarrationOnce(key: String, message: String) {
        if (!activeRequestIsDevelopment) return
        if (!emittedNarrationMilestones.add(key)) return
        currentEvidenceEntries.clear()
        appendMessage(ChatMessage("ai", message))
    }

    private fun maybeAppendNarrationForCliCategory(category: String) {
        when (category) {
            "执行命令", "模型回复" -> appendNarrationOnce(
                "implementation",
                "我已经进入实现阶段了。现在会根据项目里的真实文件继续处理，尽量只改和这次需求相关的部分，避免把无关代码弄乱。"
            )
            "编译打包" -> appendNarrationOnce(
                "build",
                "现在开始进入编译和 APK 检查阶段。这里看的不是聊天回复，而是服务器真实的打包结果；如果编译失败，我会先根据错误继续修。"
            )
            "环境提示" -> appendNarrationOnce(
                "environment",
                "后台发现了环境提示。我会先判断它是不是影响本次开发的真实问题；如果只是技术日志，会继续折叠起来。"
            )
        }
    }

    private fun maybeAppendNarrationForProgress(content: String, stage: String) {
        when {
            content.contains("CLI 工作区") || content.contains("项目环境已准备好") -> appendNarrationOnce(
                "workspace",
                "我已经连到服务器工作区。下一步会看项目里现有文件，而不是凭空猜；这样能避免重复建错项目或覆盖已有内容。"
            )
            stage == "开发实现" -> appendNarrationOnce(
                "implementation",
                "我已经进入实现阶段了。现在会根据项目里的真实文件继续处理，尽量只改和这次需求相关的部分，避免把无关代码弄乱。"
            )
            stage == "编译打包" -> appendNarrationOnce(
                "build",
                "现在开始进入编译和 APK 检查阶段。这里看的不是聊天回复，而是服务器真实的打包结果；如果编译失败，我会先根据错误继续修。"
            )
        }
    }

    private fun maybeAppendNarrationForTool(tool: String) {
        when (tool) {
            "init_project" -> appendNarrationOnce(
                "workspace",
                "当前项目需要先准备 Android 工程。我会先建立最小可运行结构，再把你的功能放进去。"
            )
            "read_file", "list_dir" -> appendNarrationOnce(
                "workspace",
                "我正在查看项目结构和关键文件。先读清楚再改，比直接写代码更稳。"
            )
            "write_file" -> appendNarrationOnce(
                "implementation",
                "我已经开始写入代码了。接下来会检查这些改动能不能真的编译运行。"
            )
            "build_project" -> appendNarrationOnce(
                "build",
                "现在开始打包 APK。打包结果会决定这轮能不能交付下载链接。"
            )
        }
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
            if (!appInForeground) return@postDelayed
            wsClient.disconnect()
            binding.root.postDelayed({ wsClient.connect() }, 500)
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
                    ChatMessage("ai-progress", workflowProgressMessage(content))
                }
                "tool_call"   -> {
                    val tool = jsonStringOrNull(json, "tool") ?: "工具"
                    handleToolCall(tool)
                    appendMessage(ChatMessage("ai-tool", toolCallWorkflowMessage(tool)))
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
                    appendMessage(ChatMessage("ai-progress", toolResultWorkflowMessage(tool)))
                    return
                }
                "done"        -> {
                    waitingForReply = false
                    setSendEnabled(true)
                    pendingRequestPayload = null
                    pendingReconnectForActiveWork = false
                    reconnectAttempts = 0
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
                    resetFoldedCliLog()
                    aiMessageWithCurrentEvidence(finalReplyMessage(content, apkUrl, imageUrl, wasDevelopment))
                }
                "error" -> {
                    waitingForReply = false
                    setSendEnabled(true)
                    pendingRequestPayload = null
                    pendingReconnectForActiveWork = false
                    reconnectAttempts = 0
                    val error = friendlyErrorMessage(jsonStringOrNull(json, "message") ?: "未知错误")
                    val wasDevelopment = activeRequestIsDevelopment
                    if (wasDevelopment) {
                        updateStage("需要处理", error)
                        addProjectEvent("发生错误：${summarize(error, 30)}")
                        recordEvidence("result", "发生错误：$error")
                    }
                    activeRequestIsDevelopment = false
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
            if (activeRequestIsDevelopment) {
                updateStage("需要处理", "服务端返回内容无法识别。")
                addProjectEvent("服务端返回异常")
            }
            appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("服务端返回内容无法识别。")))
            activeRequestIsDevelopment = false
            appendMessage(ChatMessage("error", "服务端返回异常，无法解析。"))
        }
    }

    private fun appendMessage(msg: ChatMessage) {
        chatAdapter.addMessage(msg)
        updateActiveConversationPreview(msg)
        binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
    }

    private fun handleProgress(content: String) {
        val lower = content.lowercase(Locale.CHINA)
        when {
            content.contains("未找到 APK") ||
                content.contains("未检测到 java") ||
                content.contains("未检测到 Android SDK") ->
                updateStage("需要处理", content)
            content.contains("编译") ||
                content.contains("APK") ||
                content.contains("下载链接") ||
                lower.contains("gradle") ||
                lower.contains("assemble") ->
                updateStage("编译打包", content)
            content.contains("CLI 输出") ||
                content.contains("写入") ||
                content.contains("读取") ||
                content.contains("修改") ||
                content.contains("工具") ->
                updateStage("开发实现", content)
            content.contains("理解需求") ||
                content.contains("AI 代理") ||
                content.contains("CLI 工作区") ||
                content.contains("启动本地 CLI") ->
                updateStage("需求分析", content)
            else ->
                updateStage("开发实现", content)
        }
        maybeAppendNarrationForProgress(content, currentStage)
        if (!content.startsWith("CLI 仍在运行")) {
            recordEvidence("progress", userFacingProgress(content))
        }
        addProjectEvent("进度更新：${summarize(content, 30)}")
    }

    private fun handleToolCall(tool: String) {
        maybeAppendNarrationForTool(tool)
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

    private fun toolWorkflowDoing(tool: String): String {
        return when (tool) {
            "init_project" -> "准备项目结构。"
            "read_file" -> "读取相关代码，避免凭空修改。"
            "write_file" -> "写入已确认的改动。"
            "list_dir" -> "查看目录，确认文件位置。"
            "run_shell" -> "运行命令获取真实结果。"
            "git_commit" -> "保存当前版本。"
            "build_project" -> "编译项目并准备 APK。"
            else -> "推进当前任务。"
        }
    }

    private fun toolWorkflowHint(tool: String): String {
        return when (tool) {
            "init_project" -> "开始写入代码。"
            "read_file", "list_dir" -> "判断修改位置。"
            "write_file" -> "检查是否需要补充。"
            "run_shell" -> "读取输出并决定下一步。"
            "git_commit" -> "准备交付结果。"
            "build_project" -> "生成下载链接。"
            else -> "继续推进。"
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
        val staleWorkflowRoles = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool")
        val workflowHistoryStatusRoles = setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete")
        val workflowTerminalRoles = setOf("ai", "error", "ai-stopped")
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

    private fun looksLikeDevelopmentRequest(text: String): Boolean {
        val lower = text.lowercase(Locale.CHINA)
        val words = listOf(
            "app", "apk", "android", "应用", "功能", "页面", "界面", "按钮", "代码", "开发",
            "修改", "添加", "新增", "生成", "做一个", "做个", "编译", "打包", "安装", "发布",
            "登录", "注册", "首页", "设置", "接口", "后端", "服务端", "数据库", "继续", "项目"
        )
        return words.any { lower.contains(it) }
    }

    private fun expandShortDevelopmentCommand(text: String): String {
        val normalized = text.trim().lowercase(Locale.CHINA)
        return when (normalized) {
            "继续", "继续吧", "继续开发", "继续做", "继续完成", "重试", "再试一次", "重新开始", "再来一次" ->
                "请继续完成上一次未完成的开发任务。先检查当前项目状态和是否已经生成 APK；如果已生成，请直接给出下载链接；如果未生成，请继续开发、编译并说明结果。"
            "打包", "编译", "生成apk", "生成 apk", "打包apk", "打包 apk" ->
                "请编译当前项目并生成可以下载安装到手机的 APK 下载链接。"
            else -> text
        }
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
        }
    }

    override fun onDestroy() {
        stopStageHintShimmer()
        speechRecognizer?.destroy()
        speechRecognizer = null
        if (::wsClient.isInitialized && (!waitingForReply || isFinishing)) {
            wsClient.disconnect()
        }
        super.onDestroy()
    }
}
