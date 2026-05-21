package com.elon.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.text.InputType
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
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

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var wsClient: ElonWsClient
    private lateinit var chatAdapter: ChatAdapter
    private var waitingForReply = false
    private var activeRequestIsDevelopment = false
    private val projectEvents = mutableListOf<String>()
    private val conversations = mutableListOf<AppConversation>()
    private val gson = com.google.gson.Gson()
    private val http = OkHttpClient()
    private val timeFormatter = SimpleDateFormat("HH:mm", Locale.CHINA)
    private val prefs by lazy { getSharedPreferences("elon", MODE_PRIVATE) }
    private val serverUrl = "http://43.139.149.158:8080"
    private var currentProjectTitle = "等待你的第一个开发需求"
    private var currentStage = "待提交需求"
    private var activeConversationIndex = 0
    private var modelOptions: List<ModelOption> = emptyList()
    private var selectedAgentName: String? = null
    private var currentModelLabel = "默认"

    private data class AppConversation(
        val id: String,
        var title: String,
        var subtitle: String,
        var updatedAt: Long,
        var ended: Boolean = false,
        val messages: MutableList<ChatMessage>
    )

    private data class ModelOption(
        val label: String,
        val agentName: String?
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

        loadProjectState()
        loadConversations()

        chatAdapter = ChatAdapter(activeConversation().messages)
        binding.chatList.adapter = chatAdapter
        setupNavigation()
        setupQuickActions()
        updateProjectViews("像聊天一样发需求，我会同步整理开发进度和项目记录。")

        // 连接 WebSocket
        wsClient = ElonWsClient(
            serverUrl = "ws://43.139.149.158:8080/ws",
            onMessage = { msg -> runOnUiThread { appendMessage(msg) } },
            onConnected = {
                runOnUiThread {
                    updateFirstConversationStatus("已连接 · 点击进入开发会话")
                    if (!waitingForReply) setSendEnabled(true)
                }
            },
            onDisconnected = {
                runOnUiThread {
                    updateFirstConversationStatus("未连接 · 点击重试")
                    if (waitingForReply) {
                        waitingForReply = false
                        appendMessage(ChatMessage("error", "连接已断开，请重试。"))
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

        // 发送按钮
        binding.sendButton.setOnClickListener { sendMessage() }
        binding.modelButton.setOnClickListener { showModelDialog() }
        loadModelOptions()

        // 键盘回车发送
        binding.inputEdit.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_SEND) {
                sendMessage()
                true
            } else false
        }
    }

    override fun onResume() {
        super.onResume()
        if (::binding.isInitialized) {
            loadModelOptions()
        }
    }

    private fun sendMessage() {
        val text = binding.inputEdit.text.toString().trim()
        if (text.isEmpty()) return
        if (activeConversation().ended) {
            appendMessage(ChatMessage("error", "这个会话已结束，请新建会话继续。"))
            return
        }

        if (!wsClient.isConnected()) {
            appendMessage(ChatMessage("error", "还没有连接到服务器，请点击上方状态栏重试。"))
            updateFirstConversationStatus("未连接 · 点击重试")
            wsClient.connect()
            return
        }

        val payload = com.google.gson.JsonObject().apply {
            addProperty("user_id", userId)
            addProperty("message", text)
            selectedAgentName?.let { addProperty("agent", it) }
        }

        // 显示用户消息
        appendMessage(ChatMessage(role = "user", content = text))
        binding.inputEdit.text.clear()
        setSendEnabled(false)
        waitingForReply = true
        activeRequestIsDevelopment = looksLikeDevelopmentRequest(text)
        if (activeRequestIsDevelopment) {
            currentProjectTitle = summarize(text, 24)
            saveProjectTitle()
            addProjectEvent("提交需求：${summarize(text, 36)}")
            updateStage("需求分析", "已收到需求，正在拆解功能和实现路径。")
        } else {
            updateProjectViews("普通消息已发送，开发项目记录保持不变。")
        }

        // 通过 WebSocket 发送 JSON（包含 user_id，服务端据此隔离工作区）
        if (!wsClient.send(payload.toString())) {
            waitingForReply = false
            setSendEnabled(true)
            appendMessage(ChatMessage("error", "消息发送失败，请检查网络后重试。"))
        }
    }

    private fun setupNavigation() {
        val tabs = listOf(binding.tabChat, binding.tabProject, binding.tabProfile)

        fun select(tab: TextView) {
            tabs.forEach {
                it.setTextColor(Color.parseColor(if (it == tab) "#D0D0D0" else "#A5A5A5"))
                it.textSize = if (it == tab) 13f else 12f
            }
            binding.conversationPage.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
            binding.chatPage.visibility = View.GONE
            binding.projectPage.visibility = if (tab == binding.tabProject) View.VISIBLE else View.GONE
            binding.profilePage.visibility = if (tab == binding.tabProfile) View.VISIBLE else View.GONE
            binding.inputLayout.visibility = View.GONE
            binding.pageTabs.visibility = View.VISIBLE
            binding.backButton.visibility = View.GONE
            binding.searchButton.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
            binding.addButton.visibility = if (tab == binding.tabChat) View.VISIBLE else View.GONE
            binding.moreButton.visibility = View.GONE
            binding.topTitleText.text = when (tab) {
                binding.tabProject -> "项目"
                binding.tabProfile -> "我的"
                else -> "会话区"
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
        binding.addButton.setOnClickListener { showCreateConversationDialog() }
        binding.searchButton.setOnClickListener { updateFirstConversationStatus("搜索功能准备中 · 点击进入开发会话") }
        binding.moreButton.setOnClickListener { showMoreActions() }
        binding.backButton.setOnClickListener { select(binding.tabChat) }
        select(binding.tabChat)
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
        saveConversations()
        renderConversationList()
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
                    if (name.isNotBlank()) {
                        options.add(ModelOption(modelLabel(name, model), name))
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
            .setTitle("选择模型")
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
            label.contains("[") -> label.substringBefore("[").trim().take(4)
            else -> label.take(4)
        }
    }

    private fun openConversation(index: Int) {
        if (conversations.isEmpty()) loadConversations()
        activeConversationIndex = index.coerceIn(0, conversations.lastIndex)
        chatAdapter = ChatAdapter(activeConversation().messages)
        binding.chatList.adapter = chatAdapter
        showChat()
        if (chatAdapter.itemCount > 0) {
            binding.chatList.scrollToPosition(chatAdapter.itemCount - 1)
        }
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

    private fun welcomeMessage(): ChatMessage {
        return ChatMessage(
            "ai",
            "你可以直接描述想开发的 App 功能；我会把需求分析、开发实现、编译打包和交付记录同步到进度页。"
        )
    }

    private fun loadConversations() {
        conversations.clear()
        val saved = prefs.getString("conversations_json", null)
        val loaded = runCatching {
            if (saved.isNullOrBlank()) null
            else gson.fromJson(saved, Array<AppConversation>::class.java)?.toMutableList()
        }.getOrNull()

        loaded.orEmpty()
            .filter { it.title.isNotBlank() }
            .forEach {
                if (it.messages.isEmpty()) it.messages.add(welcomeMessage())
                conversations.add(it)
            }

        if (conversations.isEmpty()) {
            conversations.add(createDefaultConversation())
        }
        activeConversationIndex = activeConversationIndex.coerceIn(0, conversations.lastIndex)
    }

    private fun saveConversations() {
        prefs.edit().putString("conversations_json", gson.toJson(conversations)).apply()
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
        conversation.updatedAt = System.currentTimeMillis()
        when (message.role) {
            "user" -> {
                conversation.subtitle = summarize(message.content, 30)
                if (conversation.title.startsWith("新会话")) {
                    conversation.title = summarize(message.content, 12)
                    binding.topTitleText.text = conversation.title
                }
            }
            "ai", "ai-progress", "ai-tool", "error" -> {
                if (!conversation.ended) {
                    conversation.subtitle = summarize(message.content, 30)
                }
            }
        }
        saveConversations()
        renderConversationList()
    }

    private fun renderConversationList() {
        if (conversations.isEmpty()) return

        val first = conversations[0]
        binding.projectStatusText.text = first.title
        binding.statusText.text = first.subtitle
        binding.statusText.setTextColor(conversationSubtitleColor(first.subtitle))
        binding.conversationTimeText.text = timeFormatter.format(Date(first.updatedAt))

        while (binding.conversationPage.childCount > 1) {
            binding.conversationPage.removeViewAt(1)
        }
        for (index in 1 until conversations.size) {
            binding.conversationPage.addView(createConversationRow(index, conversations[index]))
        }
    }

    private fun createConversationRow(index: Int, conversation: AppConversation): View {
        val row = LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(72)
            )
            setBackgroundColor(Color.parseColor("#242424"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), 0, dp(16), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openConversation(index) }
            setOnLongClickListener {
                showConversationActions(index)
                true
            }
        }

        row.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            setBackgroundResource(R.drawable.bg_mock_avatar)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = avatarText(conversation.title)
            setTextColor(Color.parseColor("#333333"))
            textSize = 19f
            setTypeface(typeface, android.graphics.Typeface.BOLD)
        })

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
            text = conversation.title
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 17f
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
            text = conversation.subtitle
            setTextColor(conversationSubtitleColor(conversation.subtitle))
            textSize = 14f
        })
        row.addView(middle)

        row.addView(TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP
                marginStart = dp(8)
                topMargin = dp(18)
            }
            includeFontPadding = false
            text = timeFormatter.format(Date(conversation.updatedAt))
            setTextColor(Color.parseColor("#C4C4C4"))
            textSize = 13f
        })
        return row
    }

    private fun avatarText(title: String): String {
        return if (title.startsWith("一龙")) "龙" else title.take(1).ifBlank { "新" }
    }

    private fun conversationSubtitleColor(text: String): Int {
        return when {
            text.startsWith("已连接") -> Color.parseColor("#07C160")
            text.startsWith("未连接") -> Color.parseColor("#D93025")
            text.startsWith("会话已结束") -> Color.parseColor("#6E6E6E")
            else -> Color.parseColor("#A9A9A9")
        }
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
        binding.quickPlanButton.setOnClickListener {
            fillPlanPrompt()
        }
        binding.quickContinueButton.setOnClickListener {
            sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。")
        }
        binding.quickBuildButton.setOnClickListener {
            sendQuickCommand("请编译当前项目并生成 APK 下载链接。")
        }
        binding.quickHistoryButton.setOnClickListener { binding.tabProject.performClick() }
        binding.quickSettingsButton.setOnClickListener { openSettings() }

        binding.projectContinueButton.setOnClickListener {
            sendQuickCommand("请继续当前项目的开发，并先说明下一步要做什么。")
        }
        binding.projectBuildButton.setOnClickListener {
            sendQuickCommand("请打包当前项目，生成可以下载安装到手机的 APK。")
        }
        binding.projectRecordButton.setOnClickListener { binding.tabProject.performClick() }
        binding.projectSettingsButton.setOnClickListener { openSettings() }
        binding.profileSettingsButton.setOnClickListener { openSettings() }
    }

    private fun showMoreActions() {
        val actions = arrayOf("需求规划", "继续开发", "打包 APK", "项目记录", "AI 设置")
        AlertDialog.Builder(this)
            .setTitle("更多功能")
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "需求规划" -> fillPlanPrompt()
                    "继续开发" -> sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。")
                    "打包 APK" -> sendQuickCommand("请编译当前项目并生成 APK 下载链接。")
                    "项目记录" -> binding.tabProject.performClick()
                    "AI 设置" -> openSettings()
                }
            }
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

    private fun appendMessage(raw: String) {
        // 解析服务端推送的 JSON 消息
        try {
            val json = com.google.gson.JsonParser.parseString(raw).asJsonObject
            val type = jsonStringOrNull(json, "type") ?: return
            val msg = when (type) {
                "progress"    -> {
                    val content = jsonStringOrNull(json, "message") ?: ""
                    handleProgress(content)
                    ChatMessage("ai-progress", "进度：$content")
                }
                "tool_call"   -> {
                    handleToolCall(jsonStringOrNull(json, "tool") ?: "工具")
                    return
                }
                "tool_result" -> {
                    val tool = jsonStringOrNull(json, "tool") ?: "工具"
                    addProjectEvent("工具完成：${toolLabel(tool)}")
                    return
                }
                "done"        -> {
                    waitingForReply = false
                    setSendEnabled(true)
                    val content = jsonStringOrNull(json, "message") ?: ""
                    val apkUrl  = jsonStringOrNull(json, "apk_url")
                    if (activeRequestIsDevelopment) {
                        updateStage("交付完成", if (apkUrl != null) "APK 已生成，可以下载安装测试。" else "任务已完成，可以继续提出修改。")
                        addProjectEvent(if (apkUrl != null) "生成 APK 下载链接" else "任务完成")
                    } else {
                        updateProjectViews("普通消息已回复，开发项目记录保持不变。")
                    }
                    activeRequestIsDevelopment = false
                    ChatMessage("ai", content + (apkUrl?.let { "\n\n下载新 APK：$it" } ?: ""))
                }
                "error" -> {
                    waitingForReply = false
                    setSendEnabled(true)
                    val error = jsonStringOrNull(json, "message") ?: "未知错误"
                    if (activeRequestIsDevelopment) {
                        updateStage("需要处理", "开发过程中遇到问题，请根据提示重试或调整需求。")
                        addProjectEvent("发生错误：${summarize(error, 30)}")
                    }
                    activeRequestIsDevelopment = false
                    ChatMessage("error", "错误：$error")
                }
                else -> return
            }
            appendMessage(msg)
        } catch (_: Exception) {
            waitingForReply = false
            setSendEnabled(true)
            if (activeRequestIsDevelopment) {
                updateStage("需要处理", "服务端返回内容无法识别。")
                addProjectEvent("服务端返回异常")
            }
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
        when {
            content.contains("理解需求") || content.contains("AI 代理") ->
                updateStage("需求分析", content)
            content.contains("编译") || content.contains("APK") || content.contains("下载链接") ->
                updateStage("编译打包", content)
            else ->
                updateStage("开发实现", content)
        }
        addProjectEvent("进度更新：${summarize(content, 30)}")
    }

    private fun handleToolCall(tool: String) {
        when (tool) {
            "build_project" -> updateStage("编译打包", "正在编译项目并准备 APK。")
            "git_commit" -> updateStage("交付完成", "正在保存当前开发版本。")
            else -> updateStage("开发实现", "正在执行：${toolLabel(tool)}")
        }
        addProjectEvent("执行工具：${toolLabel(tool)}")
    }

    private fun updateStage(stage: String, hint: String) {
        currentStage = stage
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

    private fun addProjectEvent(text: String) {
        val line = "${timeFormatter.format(Date())}  $text"
        projectEvents.add(0, line)
        while (projectEvents.size > 40) projectEvents.removeAt(projectEvents.size - 1)
        prefs.edit().putString("project_events", projectEvents.joinToString("\n")).apply()
        updateProjectViews(binding.stageHintText.text.toString())
    }

    private fun loadProjectState() {
        currentProjectTitle = prefs.getString("project_title", currentProjectTitle) ?: currentProjectTitle
        if (!looksLikeDevelopmentRequest(currentProjectTitle)) {
            currentProjectTitle = "等待你的第一个开发需求"
        }
        val saved = prefs.getString("project_events", "").orEmpty()
        projectEvents.clear()
        saved.lines().filter { it.isNotBlank() }.forEach { projectEvents.add(it) }
    }

    private fun saveProjectTitle() {
        prefs.edit().putString("project_title", currentProjectTitle).apply()
    }

    private fun setSendEnabled(enabled: Boolean) {
        val conversationEnded = activeConversation().ended
        val canSend = enabled && !conversationEnded
        binding.inputEdit.isEnabled = !conversationEnded
        binding.inputEdit.hint = if (conversationEnded) "会话已结束，请新建会话继续" else "描述你想开发的 App 功能"
        binding.sendButton.isEnabled = canSend
        binding.sendButton.alpha = if (canSend) 1f else 0.55f
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

    override fun onDestroy() {
        super.onDestroy()
        wsClient.disconnect()
    }
}
