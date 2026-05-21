package com.elon.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
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
    private val timeFormatter = SimpleDateFormat("HH:mm", Locale.CHINA)
    private val prefs by lazy { getSharedPreferences("elon", MODE_PRIVATE) }
    private var currentProjectTitle = "等待你的第一个开发需求"
    private var currentStage = "待提交需求"

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

        chatAdapter = ChatAdapter(mutableListOf())
        binding.chatList.adapter = chatAdapter
        loadProjectState()
        setupNavigation()
        setupQuickActions()
        updateProjectViews("像聊天一样发需求，我会同步整理开发进度和项目记录。")
        appendMessage(ChatMessage("ai", "你可以直接描述想开发的 App 功能；我会把需求分析、开发实现、编译打包和交付记录同步到进度页。"))

        // 连接 WebSocket
        wsClient = ElonWsClient(
            serverUrl = "ws://43.139.149.158:8080/ws",
            onMessage = { msg -> runOnUiThread { appendMessage(msg) } },
            onConnected = {
                runOnUiThread {
                    binding.statusText.text = "已连接 · 点击进入开发会话"
                    binding.statusText.setTextColor(Color.parseColor("#07C160"))
                    if (!waitingForReply) setSendEnabled(true)
                }
            },
            onDisconnected = {
                runOnUiThread {
                    binding.statusText.text = "未连接 · 点击重试"
                    binding.statusText.setTextColor(Color.parseColor("#D93025"))
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
            if (wsClient.isConnected()) showChat()
            else wsClient.connect()
        }

        // 发送按钮
        binding.sendButton.setOnClickListener { sendMessage() }

        // 键盘回车发送
        binding.inputEdit.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_SEND) {
                sendMessage()
                true
            } else false
        }
    }

    private fun sendMessage() {
        val text = binding.inputEdit.text.toString().trim()
        if (text.isEmpty()) return

        if (!wsClient.isConnected()) {
            appendMessage(ChatMessage("error", "还没有连接到服务器，请点击上方状态栏重试。"))
            binding.statusText.text = "未连接 · 点击重试"
            binding.statusText.setTextColor(Color.parseColor("#D93025"))
            wsClient.connect()
            return
        }

        val payload = com.google.gson.JsonObject().apply {
            addProperty("user_id", userId)
            addProperty("message", text)
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
                it.textSize = if (it == tab) 18f else 17f
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
            binding.topTitleText.text = when (tab) {
                binding.tabProject -> "项目"
                binding.tabProfile -> "我的"
                else -> "会话区"
            }
        }

        binding.tabChat.setOnClickListener { select(binding.tabChat) }
        binding.tabProject.setOnClickListener { select(binding.tabProject) }
        binding.tabProfile.setOnClickListener { select(binding.tabProfile) }
        binding.conversationItem.setOnClickListener { showChat() }
        binding.addButton.setOnClickListener { showChat() }
        binding.searchButton.setOnClickListener { binding.statusText.text = "搜索功能准备中 · 点击进入开发会话" }
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
        binding.topTitleText.text = "一龙开发助手"
    }

    private fun setupQuickActions() {
        binding.quickPlanButton.setOnClickListener {
            binding.inputEdit.setText("我想开发一个 App，请先帮我拆解功能、页面和开发计划：")
            binding.inputEdit.setSelection(binding.inputEdit.text.length)
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

    private fun sendQuickCommand(text: String) {
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
        binding.sendButton.isEnabled = enabled
        binding.sendButton.alpha = if (enabled) 1f else 0.55f
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
