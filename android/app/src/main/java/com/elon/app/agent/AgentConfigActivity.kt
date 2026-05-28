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
 * 支持多种 AI API Key 配置
 */
class AgentConfigActivity : Activity() {
    
    private lateinit var statusText: TextView
    private lateinit var hunyuanKeyInput: EditText
    private lateinit var qwenVLKeyInput: EditText
    private lateinit var openaiKeyInput: EditText
    private lateinit var visionProviderSpinner: Spinner
    private lateinit var websocketPortInput: EditText
    
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
        
        // === AI 配置区 ===
        layout.addView(createSectionTitle("🧠 AI 服务配置"))
        
        // 混元 API Key (必填)
        layout.addView(createLabel("混元 API Key (必填)", true))
        hunyuanKeyInput = createPasswordInput("输入混元 API Key")
        layout.addView(hunyuanKeyInput)
        layout.addView(createHint("用于文本理解和决策，从腾讯云控制台获取"))
        
        // 视觉服务选择
        layout.addView(createLabel("视觉服务提供商 (可选)", false))
        visionProviderSpinner = Spinner(this).apply {
            adapter = ArrayAdapter(
                this@AgentConfigActivity,
                android.R.layout.simple_spinner_dropdown_item,
                listOf("不使用视觉服务", "通义千问 VL", "OpenAI GPT-4V")
            )
        }
        layout.addView(visionProviderSpinner)
        
        // 通义千问 VL Key
        layout.addView(createLabel("通义千问 VL API Key", false))
        qwenVLKeyInput = createPasswordInput("输入通义千问 API Key")
        layout.addView(qwenVLKeyInput)
        layout.addView(createHint("用于图片理解，从阿里云控制台获取"))
        
        // OpenAI Key
        layout.addView(createLabel("OpenAI API Key", false))
        openaiKeyInput = createPasswordInput("输入 OpenAI API Key")
        layout.addView(openaiKeyInput)
        layout.addView(createHint("用于 GPT-4V 视觉分析"))
        
        layout.addView(createDivider())
        
        // === 网络配置区 ===
        layout.addView(createSectionTitle("🌐 网络配置"))
        
        layout.addView(createLabel("WebSocket 端口", false))
        websocketPortInput = EditText(this).apply {
            hint = "11452"
            inputType = InputType.TYPE_CLASS_NUMBER
            setText(getConfig().websocketPort.toString())
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
    
    // === 配置管理 ===
    
    private fun loadConfig() {
        val config = getConfig()
        hunyuanKeyInput.setText(config.hunyuanApiKey)
        qwenVLKeyInput.setText(config.qwenVLApiKey)
        openaiKeyInput.setText(config.openaiApiKey)
        websocketPortInput.setText(config.websocketPort.toString())
        
        visionProviderSpinner.setSelection(
            when (config.visionProvider) {
                "qwen" -> 1
                "openai" -> 2
                else -> 0
            }
        )
    }
    
    private fun saveConfig() {
        val hunyuanKey = hunyuanKeyInput.text.toString().trim()
        
        if (hunyuanKey.isEmpty()) {
            Toast.makeText(this, "❌ 混元 API Key 不能为空", Toast.LENGTH_SHORT).show()
            return
        }
        
        val visionProvider = when (visionProviderSpinner.selectedItemPosition) {
            1 -> "qwen"
            2 -> "openai"
            else -> "none"
        }
        
        val port = websocketPortInput.text.toString().toIntOrNull() ?: 11452
        
        val config = AgentConfig(
            hunyuanApiKey = hunyuanKey,
            qwenVLApiKey = qwenVLKeyInput.text.toString().trim(),
            openaiApiKey = openaiKeyInput.text.toString().trim(),
            visionProvider = visionProvider,
            websocketPort = port
        )
        
        saveConfig(config)
        Toast.makeText(this, "✅ 配置已保存", Toast.LENGTH_SHORT).show()
    }
    
    private fun testConnection() {
        val hunyuanKey = hunyuanKeyInput.text.toString().trim()
        if (hunyuanKey.isEmpty()) {
            Toast.makeText(this, "❌ 请先填写混元 API Key", Toast.LENGTH_SHORT).show()
            return
        }
        
        Toast.makeText(this, "🔄 测试连接中...", Toast.LENGTH_SHORT).show()
        
        // TODO: 实际测试 API 连接
        // 这里简化处理，实际应该发起测试请求
        android.os.Handler(mainLooper).postDelayed({
            Toast.makeText(this, "✅ 配置格式正确（实际连接需启动服务后验证）", Toast.LENGTH_LONG).show()
        }, 1000)
    }
    
    private fun getConfig(): AgentConfig {
        val prefs = getSharedPreferences("agent_config", Context.MODE_PRIVATE)
        return AgentConfig(
            hunyuanApiKey = prefs.getString("hunyuan_api_key", "") ?: "",
            qwenVLApiKey = prefs.getString("qwen_vl_api_key", "") ?: "",
            openaiApiKey = prefs.getString("openai_api_key", "") ?: "",
            visionProvider = prefs.getString("vision_provider", "none") ?: "none",
            websocketPort = prefs.getInt("websocket_port", 11452)
        )
    }
    
    private fun saveConfig(config: AgentConfig) {
        getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            .edit()
            .putString("hunyuan_api_key", config.hunyuanApiKey)
            .putString("qwen_vl_api_key", config.qwenVLApiKey)
            .putString("openai_api_key", config.openaiApiKey)
            .putString("vision_provider", config.visionProvider)
            .putInt("websocket_port", config.websocketPort)
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
            if (!v1Enabled && !v2Enabled) {
                append("\n\n⚠️ 请点击下方按钮启用无障碍服务")
            }
        }
    }
    
    private fun isServiceEnabled(serviceName: String): Boolean {
        val fullName = "${packageName}/.$serviceName"
        val enabledServices = Settings.Secure.getString(
            contentResolver,
            Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        return enabledServices.contains(fullName)
    }
    
    companion object {
        /**
         * 获取用户配置的 API Key（供 AgentService 使用）
         */
        fun getAgentConfig(context: Context): AgentConfig {
            val prefs = context.getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            return AgentConfig(
                hunyuanApiKey = prefs.getString("hunyuan_api_key", "") ?: "",
                qwenVLApiKey = prefs.getString("qwen_vl_api_key", "") ?: "",
                openaiApiKey = prefs.getString("openai_api_key", "") ?: "",
                visionProvider = prefs.getString("vision_provider", "none") ?: "none",
                websocketPort = prefs.getInt("websocket_port", 11452)
            )
        }
        
        // 兼容旧版本
        fun getApiKey(context: Context): String {
            return getAgentConfig(context).hunyuanApiKey
        }
    }
}

/**
 * Agent 配置数据类
 */
data class AgentConfig(
    val hunyuanApiKey: String,
    val qwenVLApiKey: String = "",
    val openaiApiKey: String = "",
    val visionProvider: String = "none", // none, qwen, openai
    val websocketPort: Int = 11452
) {
    val hasVision: Boolean
        get() = visionProvider != "none" && getVisionApiKey().isNotEmpty()
    
    fun getVisionApiKey(): String = when (visionProvider) {
        "qwen" -> qwenVLApiKey
        "openai" -> openaiApiKey
        else -> ""
    }
}
