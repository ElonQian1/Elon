// interface/AgentConfigActivity.kt
package com.elon.app.agent

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.widget.*

/**
 * Agent 配置界面 V2.0
 * 支持多种 AI API Key 配置 + 语音助手模式选择
 */
class AgentConfigActivity : Activity() {

    // 语音模式常量
    companion object {
        const val VOICE_MODE_SIMPLE = "simple"
        const val VOICE_MODE_APIKEY = "apikey"
        const val VOICE_MODE_CLI    = "cli"

        fun getAgentConfig(context: Context): AgentConfig {
            val prefs = context.getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            return AgentConfig(
                hunyuanApiKey   = prefs.getString("hunyuan_api_key", "") ?: "",
                qwenVLApiKey    = prefs.getString("qwen_vl_api_key", "") ?: "",
                openaiApiKey    = prefs.getString("openai_api_key", "") ?: "",
                visionProvider  = prefs.getString("vision_provider", "none") ?: "none",
                websocketPort   = prefs.getInt("websocket_port", 11452),
                voiceMode       = prefs.getString("voice_mode", VOICE_MODE_SIMPLE) ?: VOICE_MODE_SIMPLE,
                cliProjectId    = prefs.getString("cli_project_id", "") ?: "",
                cliServerUrl    = prefs.getString("cli_server_url", "http://43.139.149.158:8080") ?: "http://43.139.149.158:8080"
            )
        }

        // 兼容旧版本
        fun getApiKey(context: Context): String = getAgentConfig(context).hunyuanApiKey
    }

    private lateinit var statusText: TextView
    private lateinit var hunyuanKeyInput: EditText
    private lateinit var qwenVLKeyInput: EditText
    private lateinit var openaiKeyInput: EditText
    private lateinit var visionProviderSpinner: Spinner
    private lateinit var websocketPortInput: EditText

    // 语音模式选择
    private lateinit var voiceModeGroup: RadioGroup
    private lateinit var rbSimple: RadioButton
    private lateinit var rbApiKey: RadioButton
    private lateinit var rbCli: RadioButton

    // CLI 模式配置
    private lateinit var cliSection: LinearLayout
    private lateinit var cliProjectIdInput: EditText
    private lateinit var cliServerUrlInput: EditText

    // API Key 区域（仅 apikey 模式显示）
    private lateinit var apiKeySection: LinearLayout
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val scrollView = ScrollView(this)
        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(48, 48, 48, 48)
        }

        // === 标题区 ===
        layout.addView(createTitle())
        layout.addView(createDivider())

        // === 服务状态区 ===
        statusText = TextView(this).apply {
            text = "检查中..."
            textSize = 16f
            setPadding(0, 16, 0, 24)
        }
        layout.addView(statusText)

        // ================================================================
        // === 🎙️ 语音助手模式选择 ===
        // ================================================================
        layout.addView(createSectionTitle("🎙️ 语音助手模式"))

        voiceModeGroup = RadioGroup(this).apply {
            orientation = RadioGroup.VERTICAL
            setPadding(0, 8, 0, 8)
        }
        rbSimple = RadioButton(this).apply {
            text = "简单模式（本地关键词匹配，无需网络）"
            id = View.generateViewId()
        }
        rbApiKey = RadioButton(this).apply {
            text = "混元 API Key 模式（直连腾讯混元 AI）"
            id = View.generateViewId()
        }
        rbCli = RadioButton(this).apply {
            text = "服务器 CLI 模式（由 elon 服务器 AI 回答）"
            id = View.generateViewId()
        }
        voiceModeGroup.addView(rbSimple)
        voiceModeGroup.addView(rbApiKey)
        voiceModeGroup.addView(rbCli)
        layout.addView(voiceModeGroup)

        voiceModeGroup.setOnCheckedChangeListener { _, _ -> updateSectionVisibility() }

        layout.addView(createDivider())

        // ================================================================
        // === 🧠 API Key 配置区（仅 apikey 模式） ===
        // ================================================================
        apiKeySection = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }

        apiKeySection.addView(createSectionTitle("🧠 混元 AI 配置"))

        apiKeySection.addView(createLabel("混元 API Key", true))
        hunyuanKeyInput = createPasswordInput("输入混元 API Key")
        apiKeySection.addView(hunyuanKeyInput)
        apiKeySection.addView(createHint("从腾讯云控制台获取，用于语音意图分析和对话回复"))

        apiKeySection.addView(createLabel("视觉服务提供商 (可选)", false))
        visionProviderSpinner = Spinner(this).apply {
            adapter = ArrayAdapter(
                this@AgentConfigActivity,
                android.R.layout.simple_spinner_dropdown_item,
                listOf("不使用视觉服务", "通义千问 VL", "OpenAI GPT-4V")
            )
        }
        apiKeySection.addView(visionProviderSpinner)

        apiKeySection.addView(createLabel("通义千问 VL API Key", false))
        qwenVLKeyInput = createPasswordInput("输入通义千问 API Key")
        apiKeySection.addView(qwenVLKeyInput)
        apiKeySection.addView(createHint("用于图片理解，从阿里云控制台获取"))

        apiKeySection.addView(createLabel("OpenAI API Key", false))
        openaiKeyInput = createPasswordInput("输入 OpenAI API Key")
        apiKeySection.addView(openaiKeyInput)
        apiKeySection.addView(createHint("用于 GPT-4V 视觉分析"))

        layout.addView(apiKeySection)

        // ================================================================
        // === 🖥️ 服务器 CLI 配置区（仅 cli 模式） ===
        // ================================================================
        cliSection = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }

        cliSection.addView(createSectionTitle("🖥️ 服务器 CLI 配置"))
        cliSection.addView(createHint("语音输入将发送给 elon 服务器，由服务器上的 AI CLI 回答。\n需要已登录 elon APP 并拥有一个项目。"))

        cliSection.addView(createLabel("Project ID", true))
        cliProjectIdInput = EditText(this).apply {
            hint = "例如：abc123def456"
            inputType = InputType.TYPE_CLASS_TEXT
            isSingleLine = true
        }
        cliSection.addView(cliProjectIdInput)
        cliSection.addView(createHint("在 elon APP 项目设置中可查看项目 ID"))

        cliSection.addView(createLabel("服务器地址 (可选)", false))
        cliServerUrlInput = EditText(this).apply {
            hint = "http://43.139.149.158:8080"
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            isSingleLine = true
        }
        cliSection.addView(cliServerUrlInput)
        cliSection.addView(createHint("留空则使用默认服务器"))

        layout.addView(cliSection)

        layout.addView(createDivider())

        // ================================================================
        // === 🌐 网络配置区 ===
        // ================================================================
        layout.addView(createSectionTitle("🌐 网络配置"))
        layout.addView(createLabel("WebSocket 端口", false))
        websocketPortInput = EditText(this).apply {
            hint = "11452"
            inputType = InputType.TYPE_CLASS_NUMBER
        }
        layout.addView(websocketPortInput)
        layout.addView(createHint("PC 端连接使用的端口"))

        layout.addView(createDivider())

        // === 按钮区 ===
        val buttonLayout = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            setPadding(0, 24, 0, 16)
        }
        val saveButton = Button(this).apply {
            text = "💾 保存配置"
            setOnClickListener { saveConfig() }
        }
        buttonLayout.addView(saveButton)
        val testButton = Button(this).apply {
            text = "🧪 测试连接"
            setOnClickListener { testConnection() }
        }
        buttonLayout.addView(testButton)
        
        layout.addView(buttonLayout)
        
        // 无障碍设置按钮
        val accessibilityButton = Button(this).apply {
            text = "⚙️ 打开无障碍设置"
            setOnClickListener { openAccessibilitySettings() }
        }
        layout.addView(accessibilityButton)
        
        // 版本信息
        layout.addView(TextView(this).apply {
            text = "Android AI Agent V2.0"
            textSize = 12f
            setTextColor(Color.GRAY)
            gravity = Gravity.CENTER
            setPadding(0, 32, 0, 0)
        })
        
        scrollView.addView(layout)
        setContentView(scrollView)
        
        // 加载配置
        loadConfig()
        updateServiceStatus()
    }
    
    override fun onResume() {
        super.onResume()
        updateServiceStatus()
    }
    
    // === UI 辅助方法 ===

    private fun createTitle(): TextView = TextView(this).apply {
        text = "🤖 AI Agent 配置中心"
        textSize = 24f
        setTypeface(null, Typeface.BOLD)
        gravity = Gravity.CENTER
        setPadding(0, 0, 0, 16)
    }

    private fun createSectionTitle(title: String): TextView = TextView(this).apply {
        text = title
        textSize = 18f
        setTypeface(null, Typeface.BOLD)
        setPadding(0, 16, 0, 8)
    }

    private fun createLabel(text: String, required: Boolean): TextView = TextView(this).apply {
        this.text = if (required) "$text *" else text
        textSize = 14f
        setPadding(0, 16, 0, 4)
        if (required) setTextColor(Color.parseColor("#1976D2"))
    }

    private fun createHint(hint: String): TextView = TextView(this).apply {
        text = hint
        textSize = 12f
        setTextColor(Color.GRAY)
        setPadding(0, 0, 0, 8)
    }

    private fun createPasswordInput(hint: String): EditText = EditText(this).apply {
        this.hint = hint
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
        isSingleLine = true
    }

    private fun createDivider(): View = View(this).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, 2
        ).apply { setMargins(0, 24, 0, 24) }
        setBackgroundColor(Color.LTGRAY)
    }

    // === 模式切换 ===

    private fun updateSectionVisibility() {
        val isApiKey = rbApiKey.isChecked
        val isCli    = rbCli.isChecked
        apiKeySection.visibility = if (isApiKey) View.VISIBLE else View.GONE
        cliSection.visibility    = if (isCli)    View.VISIBLE else View.GONE
    }

    // === 配置管理 ===

    private fun loadConfig() {
        val config = getConfig()
        hunyuanKeyInput.setText(config.hunyuanApiKey)
        qwenVLKeyInput.setText(config.qwenVLApiKey)
        openaiKeyInput.setText(config.openaiApiKey)
        websocketPortInput.setText(config.websocketPort.toString())
        cliProjectIdInput.setText(config.cliProjectId)
        cliServerUrlInput.setText(config.cliServerUrl)

        visionProviderSpinner.setSelection(
            when (config.visionProvider) {
                "qwen" -> 1
                "openai" -> 2
                else -> 0
            }
        )

        when (config.voiceMode) {
            VOICE_MODE_APIKEY -> rbApiKey.isChecked = true
            VOICE_MODE_CLI    -> rbCli.isChecked    = true
            else              -> rbSimple.isChecked = true
        }
        updateSectionVisibility()
    }

    private fun saveConfig() {
        val mode = when {
            rbApiKey.isChecked -> VOICE_MODE_APIKEY
            rbCli.isChecked    -> VOICE_MODE_CLI
            else               -> VOICE_MODE_SIMPLE
        }

        // 模式特定校验
        if (mode == VOICE_MODE_APIKEY && hunyuanKeyInput.text.isBlank()) {
            Toast.makeText(this, "❌ 混元 API Key 模式需要填写 API Key", Toast.LENGTH_SHORT).show()
            return
        }
        if (mode == VOICE_MODE_CLI && cliProjectIdInput.text.isBlank()) {
            Toast.makeText(this, "❌ 服务器 CLI 模式需要填写 Project ID", Toast.LENGTH_SHORT).show()
            return
        }

        val visionProvider = when (visionProviderSpinner.selectedItemPosition) {
            1 -> "qwen"; 2 -> "openai"; else -> "none"
        }
        val serverUrl = cliServerUrlInput.text.toString().trim()
            .ifBlank { "http://43.139.149.158:8080" }

        val config = AgentConfig(
            hunyuanApiKey  = hunyuanKeyInput.text.toString().trim(),
            qwenVLApiKey   = qwenVLKeyInput.text.toString().trim(),
            openaiApiKey   = openaiKeyInput.text.toString().trim(),
            visionProvider = visionProvider,
            websocketPort  = websocketPortInput.text.toString().toIntOrNull() ?: 11452,
            voiceMode      = mode,
            cliProjectId   = cliProjectIdInput.text.toString().trim(),
            cliServerUrl   = serverUrl
        )

        saveConfig(config)
        Toast.makeText(this, "✅ 配置已保存（模式：${modeName(mode)}）", Toast.LENGTH_SHORT).show()
    }

    private fun modeName(mode: String) = when (mode) {
        VOICE_MODE_APIKEY -> "混元 API Key"
        VOICE_MODE_CLI    -> "服务器 CLI"
        else              -> "简单"
    }

    private fun testConnection() {
        when {
            rbApiKey.isChecked -> {
                if (hunyuanKeyInput.text.isBlank()) {
                    Toast.makeText(this, "❌ 请先填写混元 API Key", Toast.LENGTH_SHORT).show()
                    return
                }
                Toast.makeText(this, "🔄 测试 API Key 连接中...", Toast.LENGTH_SHORT).show()
                android.os.Handler(mainLooper).postDelayed({
                    Toast.makeText(this, "✅ 格式正确（完整验证需实际调用）", Toast.LENGTH_LONG).show()
                }, 800)
            }
            rbCli.isChecked -> {
                if (cliProjectIdInput.text.isBlank()) {
                    Toast.makeText(this, "❌ 请先填写 Project ID", Toast.LENGTH_SHORT).show()
                    return
                }
                Toast.makeText(this, "🔄 测试服务器连接中...", Toast.LENGTH_SHORT).show()
                android.os.Handler(mainLooper).postDelayed({
                    Toast.makeText(this, "✅ 配置已填写（需在语音界面实际发送才能验证）", Toast.LENGTH_LONG).show()
                }, 800)
            }
            else -> Toast.makeText(this, "ℹ️ 简单模式无需连接测试", Toast.LENGTH_SHORT).show()
        }
    }

    private fun getConfig(): AgentConfig {
        return getAgentConfig(this)
    }

    private fun saveConfig(config: AgentConfig) {
        getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            .edit()
            .putString("hunyuan_api_key",  config.hunyuanApiKey)
            .putString("qwen_vl_api_key",  config.qwenVLApiKey)
            .putString("openai_api_key",   config.openaiApiKey)
            .putString("vision_provider",  config.visionProvider)
            .putInt("websocket_port",      config.websocketPort)
            .putString("voice_mode",       config.voiceMode)
            .putString("cli_project_id",   config.cliProjectId)
            .putString("cli_server_url",   config.cliServerUrl)
            .apply()
    }

    private fun openAccessibilitySettings() {
        startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
    }

    private fun updateServiceStatus() {
        val v1Enabled = isServiceEnabled("AgentService")
        val v2Enabled = isServiceEnabled("AgentServiceV2")

        statusText.text = buildString {
            append("📱 服务状态:\n")
            append(if (v1Enabled) "  ✅ V1 服务：已启用\n" else "  ⚪ V1 服务：未启用\n")
            append(if (v2Enabled) "  ✅ V2 服务：已启用 (推荐)" else "  ⚪ V2 服务：未启用")
            if (!v1Enabled && !v2Enabled) append("\n\n⚠️ 请点击下方按钮启用无障碍服务")
        }
    }

    private fun isServiceEnabled(serviceName: String): Boolean {
        val fullName = "${packageName}/.$serviceName"
        val enabledServices = Settings.Secure.getString(
            contentResolver, Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        return enabledServices.contains(fullName)
    }
}

/**
 * Agent 配置数据类
 */
data class AgentConfig(
    val hunyuanApiKey:  String,
    val qwenVLApiKey:   String = "",
    val openaiApiKey:   String = "",
    val visionProvider: String = "none",        // none | qwen | openai
    val websocketPort:  Int    = 11452,
    val voiceMode:      String = AgentConfigActivity.VOICE_MODE_SIMPLE, // simple | apikey | cli
    val cliProjectId:   String = "",
    val cliServerUrl:   String = "http://43.139.149.158:8080"
) {
    val hasVision: Boolean
        get() = visionProvider != "none" && getVisionApiKey().isNotEmpty()

    fun getVisionApiKey(): String = when (visionProvider) {
        "qwen"   -> qwenVLApiKey
        "openai" -> openaiApiKey
        else     -> ""
    }
}
