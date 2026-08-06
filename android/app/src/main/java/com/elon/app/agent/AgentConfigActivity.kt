// interface/AgentConfigActivity.kt
package com.elon.app.agent

import android.app.Activity
import android.app.AlertDialog
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.widget.*
import com.elon.app.agent.infrastructure.auth.AuthService

// Elon 暗色冷灰体系色常量，和 docs/APP 颜色规范.md 保持一致。
private const val BG          = "#0B1118"
private const val SURFACE     = "#0E1116"
private const val SURFACE2    = "#20262E"
private const val ACCENT      = "#67BEA0"
private const val ACCENT2     = "#67BEA0"
private const val TEXT_PRIM   = "#F8F7F4"
private const val TEXT_SEC    = "#B3DDDBD5"
private const val TEXT_DIM    = "#80BEBEBA"
private const val DIVIDER     = "#667B8793"
private const val BTN_PRIMARY = "#F8F7F4"
private const val BTN_DANGER  = "#B71C1C"

/**
 * Agent 配置界面 V2.0 — 暗黑主题
 * 支持多种 AI API Key 配置 + 语音助手模式选择 + 账号状态
 */
class AgentConfigActivity : Activity() {

    // 语音模式常量
    companion object {
        const val VOICE_MODE_SIMPLE = "simple"
        const val VOICE_MODE_APIKEY = "apikey"
        const val VOICE_MODE_CLI    = "cli"

        val DEFAULT_MODE_ORDER: List<String> = listOf(VOICE_MODE_APIKEY, VOICE_MODE_CLI, VOICE_MODE_SIMPLE)
        private const val DEFAULT_ORDER_STR = "cli,apikey,simple"

        fun getAgentConfig(context: Context): AgentConfig {
            val prefs = context.getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            val orderStr = prefs.getString("voice_mode_order", DEFAULT_ORDER_STR) ?: DEFAULT_ORDER_STR
            val order = orderStr.split(",").map { it.trim() }.filter { it.isNotBlank() }
                .let { if (it.containsAll(listOf(VOICE_MODE_APIKEY, VOICE_MODE_CLI, VOICE_MODE_SIMPLE))) it else DEFAULT_MODE_ORDER }
            return AgentConfig(
                hunyuanApiKey   = prefs.getString("hunyuan_api_key", "") ?: "",
                qwenVLApiKey    = prefs.getString("qwen_vl_api_key", "") ?: "",
                openaiApiKey    = prefs.getString("openai_api_key", "") ?: "",
                openaiApiBase   = prefs.getString("openai_api_base", "") ?: "",
                openaiModel     = prefs.getString("openai_api_model", "") ?: "",
                visionProvider  = prefs.getString("vision_provider", "none") ?: "none",
                websocketPort   = prefs.getInt("websocket_port", 11452),
                voiceModeOrder  = order,
                cliProjectId    = prefs.getString("cli_project_id", "") ?: "",
                cliServerUrl       = prefs.getString("cli_server_url", "http://43.139.149.158:8080") ?: "http://43.139.149.158:8080",
                fallbackServerUrl   = prefs.getString("fallback_server_url", "") ?: "",
                fallbackServerToken = prefs.getString("fallback_server_token", "") ?: ""
            )
        }

        // 兼容旧版本
        fun getApiKey(context: Context): String = getAgentConfig(context).hunyuanApiKey
    }

    private lateinit var statusText: TextView
    private lateinit var accountStatusText: TextView
    private lateinit var hunyuanKeyInput: EditText
    private lateinit var qwenVLKeyInput: EditText
    private lateinit var openaiKeyInput: EditText
    private lateinit var visionProviderSpinner: Spinner
    private lateinit var websocketPortInput: EditText
    private lateinit var authService: AuthService

    // 语音模式回退排序
    private val modeOrderList: MutableList<String> = mutableListOf(
        VOICE_MODE_APIKEY, VOICE_MODE_CLI, VOICE_MODE_SIMPLE
    )
    private lateinit var modeOrderLayout: LinearLayout

    // CLI 模式配置
    private lateinit var cliSection: LinearLayout
    private lateinit var cliProjectIdInput: EditText
    private lateinit var cliServerUrlInput: EditText
    private lateinit var fallbackServerUrlInput: EditText
    private lateinit var fallbackServerTokenInput: EditText

    // API Key 区域
    private lateinit var apiKeySection: LinearLayout

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        authService = AuthService(this)

        val scrollView = ScrollView(this).apply {
            setBackgroundColor(Color.parseColor(BG))
        }
        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(16), dp(20), dp(32))
            setBackgroundColor(Color.parseColor(BG))
        }

        // === 顶部标题栏 ===
        layout.addView(createTopBar())

        // === 账号状态卡片 ===
        layout.addView(createAccountCard())

        // === 重构说明：默认走服务器 CLI ===
        layout.addView(createHint(
            "💡 默认：不填任何 Key 也能用 —— 只要上面已登录主账号且在主页选中一个项目，Agent 会自动走服务器 CLI。\n" +
            "填入 API Key 后会优先使用你自己的混元/OpenAI 兑现金。"
        ))

        layout.addView(createDivider())

        // === 无障碍服务状态 ===
        statusText = TextView(this).apply {
            text = "检查中..."
            textSize = 13f
            setTextColor(Color.parseColor(TEXT_SEC))
            setPadding(0, 0, 0, dp(8))
        }
        layout.addView(statusText)

        // ================================================================
        // === 🎙️ 语音助手模式回退顺序 ===
        // ================================================================
        layout.addView(createSectionTitle("🎙️ 语音助手模式（回退顺序）"))
        layout.addView(createHint("按顺序尝试，未填写配置的模式自动跳过；简单模式无需配置，始终兜底"))

        modeOrderLayout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(4), 0, dp(4))
        }
        layout.addView(modeOrderLayout)

        layout.addView(createDivider())

        // ================================================================
        // === 🧠 API Key 配置区 ===
        // ================================================================
        apiKeySection = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        apiKeySection.addView(createSectionTitle("🧠 混元 AI 配置"))
        apiKeySection.addView(createLabel("混元 API Key", true))
        hunyuanKeyInput = createTextInput("输入混元 API Key", password = true)
        apiKeySection.addView(hunyuanKeyInput)
        apiKeySection.addView(createHint("从腾讯云控制台获取，用于语音意图分析和对话回复"))

        apiKeySection.addView(createLabel("视觉服务提供商 (可选)", false))
        visionProviderSpinner = Spinner(this).apply {
            adapter = ArrayAdapter(
                this@AgentConfigActivity,
                android.R.layout.simple_spinner_dropdown_item,
                listOf("不使用视觉服务", "通义千问 VL", "OpenAI GPT-4V")
            )
            setBackgroundColor(Color.parseColor(SURFACE2))
        }
        apiKeySection.addView(visionProviderSpinner)

        apiKeySection.addView(createLabel("通义千问 VL API Key", false))
        qwenVLKeyInput = createTextInput("输入通义千问 API Key", password = true)
        apiKeySection.addView(qwenVLKeyInput)
        apiKeySection.addView(createHint("用于图片理解，从阿里云控制台获取"))

        apiKeySection.addView(createLabel("OpenAI API Key", false))
        openaiKeyInput = createTextInput("输入 OpenAI API Key", password = true)
        apiKeySection.addView(openaiKeyInput)
        apiKeySection.addView(createHint("用于 GPT-4V 视觉分析"))
        layout.addView(apiKeySection)

        // ================================================================
        // === 🖥️ 服务器 CLI 配置区 ===
        // ================================================================
        cliSection = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        cliSection.addView(createSectionTitle("🖥️ 服务器 CLI 配置"))
        cliSection.addView(createHint("语音输入将发送给 elon 服务器由 AI CLI 回答\n需要已登录 elon APP 并拥有一个项目"))
        cliSection.addView(createLabel("Project ID", true))
        cliProjectIdInput = createTextInput("例如：abc123def456")
        cliSection.addView(cliProjectIdInput)
        cliSection.addView(createHint("在 elon APP 项目设置中可查看项目 ID"))
        cliSection.addView(createLabel("服务器地址 (可选)", false))
        cliServerUrlInput = createTextInput("http://43.139.149.158:8080",
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI)
        cliSection.addView(cliServerUrlInput)
        cliSection.addView(createHint("留空则使用默认服务器"))
        cliSection.addView(createLabel("备用服务器地址 (可选)", false))
        fallbackServerUrlInput = createTextInput("例如: http://192.168.31.100:8081 或 Tailscale IP",
            inputType = android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_VARIATION_URI)
        cliSection.addView(fallbackServerUrlInput)
        cliSection.addView(createHint("主服务器断开连续 3 次后自动切换到备用\n备用需运行本地 homecli，推荐用 Tailscale IP"))
        cliSection.addView(createLabel("备用服务器 Token (可选)", false))
        fallbackServerTokenInput = createTextInput("homecli config.toml [owner].access_token 的值",
            password = false)
        cliSection.addView(fallbackServerTokenInput)
        cliSection.addView(createHint("切换到备用服务器时使用此 Token（留空则复用云端登录 Token）"))
        layout.addView(cliSection)

        layout.addView(createDivider())

        // ================================================================
        // === 🌐 网络配置区 ===
        // ================================================================
        layout.addView(createSectionTitle("🌐 网络配置"))
        layout.addView(createLabel("WebSocket 端口", false))
        websocketPortInput = createTextInput("11452", inputType = InputType.TYPE_CLASS_NUMBER)
        layout.addView(websocketPortInput)
        layout.addView(createHint("PC 端连接使用的端口"))

        layout.addView(createDivider())

        // === 操作按钮区 ===
        layout.addView(createActionButtons())

        // 无障碍设置
        layout.addView(createDarkButton("⚙️ 打开无障碍设置", color = "#37474F") {
            openAccessibilitySettings()
        })

        // 版本信息
        layout.addView(TextView(this).apply {
            text = "Android AI Agent V2.0"
            textSize = 11f
            setTextColor(Color.parseColor(TEXT_DIM))
            gravity = Gravity.CENTER
            setPadding(0, dp(24), 0, 0)
        })

        scrollView.addView(layout)
        setContentView(scrollView)

        loadConfig()
        updateServiceStatus()
    }

    override fun onResume() {
        super.onResume()
        updateServiceStatus()
        refreshAccountCard()
    }

    // ================================================================
    // === UI 构建方法 ===
    // ================================================================

    private fun createTopBar(): LinearLayout = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, dp(8), 0, dp(16))

        addView(Button(this@AgentConfigActivity).apply {
            text = "← 返回"
            textSize = 14f
            setTextColor(Color.parseColor(ACCENT))
            setBackgroundColor(Color.TRANSPARENT)
            setPadding(0, 0, dp(8), 0)
            setOnClickListener { finish() }
        })

        addView(TextView(this@AgentConfigActivity).apply {
            text = "🤖 AI Agent 配置"
            textSize = 20f
            setTypeface(null, Typeface.BOLD)
            setTextColor(Color.parseColor(TEXT_PRIM))
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        })
    }

    private lateinit var accountCard: LinearLayout

    private fun createAccountCard(): LinearLayout {
        val user = authService.getCurrentUser()
        val isLoggedIn = authService.isLoggedIn()
        accountCard = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setBackgroundColor(Color.parseColor(SURFACE))
            setPadding(dp(16), dp(12), dp(16), dp(12))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { bottomMargin = dp(8) }

            // 头像
            addView(TextView(this@AgentConfigActivity).apply {
                text = if (isLoggedIn) "👤" else "🔓"
                textSize = 22f
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { rightMargin = dp(12) }
            })

            // 账号信息
            addView(LinearLayout(this@AgentConfigActivity).apply {
                orientation = LinearLayout.VERTICAL
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)

                addView(TextView(this@AgentConfigActivity).apply {
                    text = if (isLoggedIn) (user?.nickname ?: user?.username ?: "已登录") else "未登录"
                    textSize = 15f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(TEXT_PRIM))
                })
                addView(TextView(this@AgentConfigActivity).apply {
                    text = if (isLoggedIn) "@${user?.username ?: ""}  ☁️ 已同步" else "登录后可同步配置、使用服务器 CLI"
                    textSize = 12f
                    setTextColor(Color.parseColor(if (isLoggedIn) ACCENT else TEXT_SEC))
                })
            })

            // 登录/登出按钮
            addView(Button(this@AgentConfigActivity).apply {
                accountStatusText = this
                text = if (isLoggedIn) "登出" else "登录"
                textSize = 13f
                setTextColor(Color.parseColor(if (isLoggedIn) TEXT_PRIM else "#0B1118"))
                setBackgroundColor(Color.parseColor(if (isLoggedIn) BTN_DANGER else BTN_PRIMARY))
                setPadding(dp(16), dp(4), dp(16), dp(4))
                setOnClickListener {
                    if (authService.isLoggedIn()) {
                        AlertDialog.Builder(this@AgentConfigActivity)
                            .setTitle("退出登录")
                            .setMessage("确定要退出吗？")
                            .setPositiveButton("确定") { _, _ ->
                                authService.logout()
                                refreshAccountCard()
                            }
                            .setNegativeButton("取消", null)
                            .show()
                    } else {
                        startActivity(Intent(this@AgentConfigActivity,
                            com.elon.app.agent.ui.LoginActivity::class.java))
                    }
                }
            })
        }
        return accountCard
    }

    private fun refreshAccountCard() {
        // 重新渲染账号卡片内容
        val parent = accountCard.parent as? LinearLayout ?: return
        val idx = (0 until parent.childCount).firstOrNull { parent.getChildAt(it) == accountCard } ?: return
        parent.removeViewAt(idx)
        parent.addView(createAccountCard(), idx)
    }

    private fun createActionButtons(): LinearLayout = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER
        setPadding(0, dp(16), 0, dp(8))

        val lp = LinearLayout.LayoutParams(0, dp(48), 1f).apply { setMargins(dp(4), 0, dp(4), 0) }

        addView(createDarkButton("💾 保存", color = BTN_PRIMARY, lp = lp) { saveConfig() })
        addView(createDarkButton("🔍 检查", color = SURFACE2, lp = lp) { testConnection() })
    }

    private fun createDarkButton(
        label: String,
        color: String = BTN_PRIMARY,
        lp: LinearLayout.LayoutParams? = null,
        onClick: () -> Unit
    ): Button = Button(this).apply {
        text = label
        textSize = 14f
        setTextColor(Color.parseColor(if (color == BTN_PRIMARY) "#0B1118" else TEXT_PRIM))
        setBackgroundColor(Color.parseColor(color))
        layoutParams = lp ?: LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, dp(44)
        ).apply { setMargins(0, dp(4), 0, dp(4)) }
        setOnClickListener { onClick() }
    }

    private fun createTitle(): TextView = TextView(this).apply {
        text = "🤖 AI Agent 配置中心"
        textSize = 22f
        setTypeface(null, Typeface.BOLD)
        setTextColor(Color.parseColor(TEXT_PRIM))
        gravity = Gravity.CENTER
        setPadding(0, 0, 0, dp(12))
    }

    private fun createSectionTitle(title: String): TextView = TextView(this).apply {
        text = title
        textSize = 16f
        setTypeface(null, Typeface.BOLD)
        setTextColor(Color.parseColor(ACCENT))
        setPadding(0, dp(16), 0, dp(6))
    }

    private fun createLabel(text: String, required: Boolean): TextView = TextView(this).apply {
        this.text = if (required) "$text *" else text
        textSize = 13f
        setTextColor(Color.parseColor(if (required) ACCENT2 else TEXT_SEC))
        setPadding(0, dp(12), 0, dp(2))
    }

    private fun createHint(hint: String): TextView = TextView(this).apply {
        text = hint
        textSize = 11f
        setTextColor(Color.parseColor(TEXT_DIM))
        setPadding(0, 0, 0, dp(4))
    }

    private fun createTextInput(
        hint: String,
        password: Boolean = false,
        inputType: Int = InputType.TYPE_CLASS_TEXT
    ): EditText = EditText(this).apply {
        this.hint = hint
        this.inputType = if (password) InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                        else inputType
        isSingleLine = true
        setTextColor(Color.parseColor(TEXT_PRIM))
        setHintTextColor(Color.parseColor(TEXT_DIM))
        setBackgroundColor(Color.parseColor(SURFACE2))
        setPadding(dp(12), dp(10), dp(12), dp(10))
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(2); bottomMargin = dp(2) }
    }

    private fun createDivider(): View = View(this).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, 1
        ).apply { setMargins(0, dp(16), 0, dp(16)) }
        setBackgroundColor(Color.parseColor(DIVIDER))
    }

    private fun dp(dp: Int): Int = (dp * resources.displayMetrics.density + 0.5f).toInt()

    // === 模式排序 UI ===

    private fun refreshModeOrderUI() {
        modeOrderLayout.removeAllViews()
        modeOrderList.forEachIndexed { index, mode ->
            val row = LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setBackgroundColor(Color.parseColor(SURFACE))
                setPadding(dp(12), dp(8), dp(8), dp(8))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { setMargins(0, dp(3), 0, dp(3)) }
            }
            val label = TextView(this).apply {
                text = "#${index + 1}  ${modeName(mode)}"
                textSize = 14f
                setTextColor(Color.parseColor(if (mode == VOICE_MODE_SIMPLE) TEXT_SEC else TEXT_PRIM))
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            }
            row.addView(label)

            if (index > 0) {
                row.addView(createOrderBtn("↑") {
                    val tmp = modeOrderList[index - 1]; modeOrderList[index - 1] = modeOrderList[index]; modeOrderList[index] = tmp
                    refreshModeOrderUI()
                })
            } else {
                row.addView(View(this).apply { layoutParams = LinearLayout.LayoutParams(dp(52), 1) })
            }

            if (index < modeOrderList.size - 1) {
                row.addView(createOrderBtn("↓") {
                    val tmp = modeOrderList[index + 1]; modeOrderList[index + 1] = modeOrderList[index]; modeOrderList[index] = tmp
                    refreshModeOrderUI()
                })
            }

            modeOrderLayout.addView(row)
        }
    }

    private fun createOrderBtn(label: String, onClick: () -> Unit): Button = Button(this).apply {
        text = label
        textSize = 14f
        setTextColor(Color.parseColor(ACCENT))
        setBackgroundColor(Color.parseColor(SURFACE2))
        setPadding(dp(12), dp(2), dp(12), dp(2))
        layoutParams = LinearLayout.LayoutParams(dp(52), dp(36)).apply { setMargins(dp(4), 0, 0, 0) }
        setOnClickListener { onClick() }
    }

    private fun dp2px(dp: Int): Int = dp(dp)

    // === 配置管理 ===

    private fun loadConfig() {
        val config = getConfig()
        hunyuanKeyInput.setText(config.hunyuanApiKey)
        qwenVLKeyInput.setText(config.qwenVLApiKey)
        openaiKeyInput.setText(config.openaiApiKey)
        websocketPortInput.setText(config.websocketPort.toString())
        cliProjectIdInput.setText(config.cliProjectId)
        cliServerUrlInput.setText(config.cliServerUrl)
        fallbackServerUrlInput.setText(config.fallbackServerUrl)
        fallbackServerTokenInput.setText(config.fallbackServerToken)

        visionProviderSpinner.setSelection(
            when (config.visionProvider) {
                "qwen" -> 1
                "openai" -> 2
                else -> 0
            }
        )

        modeOrderList.clear()
        modeOrderList.addAll(config.voiceModeOrder)
        refreshModeOrderUI()
    }

    private fun saveConfig() {
        val order = modeOrderList.toMutableList()
        // 简单模式必须在列表里（如果用户手动删除就补入末尾）
        if (!order.contains(VOICE_MODE_SIMPLE)) order.add(VOICE_MODE_SIMPLE)

        val visionProvider = when (visionProviderSpinner.selectedItemPosition) {
            1 -> "qwen"; 2 -> "openai"; else -> "none"
        }
        val serverUrl = cliServerUrlInput.text.toString().trim()
            .ifBlank { "http://43.139.149.158:8080" }
        val existing = getConfig()

        val config = AgentConfig(
            hunyuanApiKey  = hunyuanKeyInput.text.toString().trim(),
            qwenVLApiKey   = qwenVLKeyInput.text.toString().trim(),
            openaiApiKey   = openaiKeyInput.text.toString().trim(),
            openaiApiBase  = existing.openaiApiBase,
            openaiModel    = existing.openaiModel,
            visionProvider = visionProvider,
            websocketPort  = websocketPortInput.text.toString().toIntOrNull() ?: 11452,
            voiceModeOrder = order,
            cliProjectId      = cliProjectIdInput.text.toString().trim(),
            cliServerUrl      = serverUrl,
            fallbackServerUrl  = fallbackServerUrlInput.text.toString().trim(),
            fallbackServerToken = fallbackServerTokenInput.text.toString().trim()
        )

        saveConfig(config)
        val orderDesc = order.joinToString(" → ") { modeName(it) }
        Toast.makeText(this, "✅ 已保存！回退顺序：$orderDesc", Toast.LENGTH_LONG).show()
    }

    private fun modeName(mode: String) = when (mode) {
        VOICE_MODE_APIKEY -> "混元 API Key"
        VOICE_MODE_CLI    -> "服务器 CLI"
        else              -> "简单"
    }

    private fun testConnection() {
        val hasApiKey = hunyuanKeyInput.text.isNotBlank()
        val hasCli    = cliProjectIdInput.text.isNotBlank()
        val sb = StringBuilder("🔍 配置状态检查：\n")
        sb.append(if (hasApiKey) "✅ 混元 API Key：已填写\n" else "⚪ 混元 API Key：未填写（该顺序将被跳过）\n")
        sb.append(if (hasCli)    "✅ Project ID：已填写\n"   else "⚪ Project ID：未填写（该顺序将被跳过）\n")
        sb.append("✅ 简单模式：始终可用（最终兜底）\n")
        sb.append("\n回退顺序：${modeOrderList.joinToString(" → ") { modeName(it) }}")
        Toast.makeText(this, sb.toString(), Toast.LENGTH_LONG).show()
    }

    private fun getConfig(): AgentConfig {
        return getAgentConfig(this)
    }

    private fun saveConfig(config: AgentConfig) {
        getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            .edit()
            .putString("hunyuan_api_key",   config.hunyuanApiKey)
            .putString("qwen_vl_api_key",   config.qwenVLApiKey)
            .putString("openai_api_key",    config.openaiApiKey)
            .putString("openai_api_base",   config.openaiApiBase)
            .putString("openai_api_model",  config.openaiModel)
            .putString("vision_provider",   config.visionProvider)
            .putInt("websocket_port",        config.websocketPort)
            .putString("voice_mode_order",   config.voiceModeOrder.joinToString(","))
            .putString("cli_project_id",       config.cliProjectId)
            .putString("cli_server_url",        config.cliServerUrl)
            .putString("fallback_server_url",    config.fallbackServerUrl)
            .putString("fallback_server_token",   config.fallbackServerToken)
            .apply()
    }

    private fun openAccessibilitySettings() {
        startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
    }

    private fun updateServiceStatus() {
        val v2Enabled = isServiceEnabled("AgentServiceV2")
        val v1Enabled = isServiceEnabled("AgentService")
        statusText.text = when {
            v2Enabled -> "✅ V2 无障碍服务运行中"
            v1Enabled -> "✅ V1 无障碍服务运行中"
            else      -> "⚠️ 无障碍服务未启用 — 点击下方按钮前往开启"
        }
        statusText.setTextColor(Color.parseColor(if (v1Enabled || v2Enabled) ACCENT else "#FF7043"))
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
    val openaiApiBase:  String = "",
    val openaiModel:    String = "",
    val visionProvider: String = "none",        // none | qwen | openai
    val websocketPort:  Int    = 11452,
    val voiceModeOrder: List<String> = AgentConfigActivity.DEFAULT_MODE_ORDER, // 回退顺序列表
    val cliProjectId:   String = "",
    val cliServerUrl:       String = "http://43.139.149.158:8080",
    val fallbackServerUrl:  String = "",
    val fallbackServerToken: String = ""
) {
    val hasVision: Boolean
        get() = visionProvider != "none" && getVisionApiKey().isNotEmpty()

    fun getVisionApiKey(): String = when (visionProvider) {
        "qwen"   -> qwenVLApiKey
        "openai" -> openaiApiKey
        else     -> ""
    }
}
