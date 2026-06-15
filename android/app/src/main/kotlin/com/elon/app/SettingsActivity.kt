package com.elon.app

import android.app.Activity
import android.os.Bundle
import android.view.MenuItem
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import kotlinx.coroutines.*
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import com.elon.app.agent.infrastructure.voice.EnginePreference
import com.elon.app.agent.infrastructure.voice.RecognitionEngineSelector
import com.elon.app.update.AppUpdateManager

/**
 * 用户 AI 代理设置页面
 *
 * 功能：
 * - 从服务器加载当前配置和可用全局代理列表
 * - 三种模式：服务器默认 / 选择预设代理 / 完全自定义
 * - 保存后通过 PUT /api/user/{user_id}/agent 上传
 */
class SettingsActivity : AppCompatActivity() {

    private val SERVER_URL = "http://43.139.149.158:8080"
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val http = OkHttpClient()
    private val prefs by lazy { getSharedPreferences("elon", MODE_PRIVATE) }

    private lateinit var userId: String
    private data class AgentOption(val name: String, val label: String)
    private var availableAgents: List<AgentOption> = emptyList()
    private var codexCliOnly = true
    private var userByokApiEnabled = true
    private var voicePreviewSpeaker: VoiceSpeaker? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        supportActionBar?.apply {
            title = "AI 代理设置"
            setDisplayHomeAsUpEnabled(true)
        }

        userId = prefs.getString("user_id", "") ?: ""

        findViewById<TextView>(R.id.userIdText).text = "用户 ID: $userId"

        setupModeToggle()
        setupVoiceModeToggle()
        setupTtsVoiceSection()
        setupAsrChainSection()
        loadCurrentConfig()

        findViewById<Button>(R.id.saveButton).setOnClickListener { saveConfig() }
        findViewById<Button>(R.id.tokenUsageButton).setOnClickListener {
            startActivity(android.content.Intent(this, TokenUsageActivity::class.java))
        }
        findViewById<Button>(R.id.checkUpdateButton).setOnClickListener {
            AppUpdateManager(this).manualCheck()
        }
        findViewById<Button>(R.id.manageEngineButton).setOnClickListener {
            startActivity(android.content.Intent(this, VoiceEngineActivity::class.java))
            // 返回时刷新链路摘要
        }
    }

    // ── ASR 回退链摘要 + 服务器兜底开关 ──────────────────────
    private fun setupAsrChainSection() {
        // 初始化服务器兜底 Switch
        val sw = findViewById<Switch>(R.id.switchServerFallback)
        sw.isChecked = AsrFallbackSettings.isServerFallbackEnabled(this)
        sw.setOnCheckedChangeListener { _, checked ->
            AsrFallbackSettings.setServerFallbackEnabled(this, checked)
            refreshAsrChainSummary()
            val msg = if (checked) "云端 Whisper 兜底已开启" else "云端 Whisper 兜底已关闭"
            Toast.makeText(this, msg, Toast.LENGTH_SHORT).show()
        }
        refreshAsrChainSummary()
    }

    /** 构建并刷新回退链摘要文字（onResume 时也应调用）。 */
    private fun refreshAsrChainSummary() {
        val summaryView = findViewById<TextView>(R.id.asrChainSummary) ?: return
        val disabled = AsrFallbackSettings.getDisabledEngineKeys(this)
        val serverOn = AsrFallbackSettings.isServerFallbackEnabled(this)
        val ordered = RecognitionEngineSelector.listForUse(this).filter { it.key() !in disabled }
        val chainParts = ordered.mapIndexed { i, e -> "#${i + 1} ${e.label.take(16)}" }.toMutableList()
        if (serverOn) chainParts.add("☁️ 服务器 Whisper")
        summaryView.text = if (chainParts.isEmpty()) {
            "⚠️ 所有本地引擎已排除且云端兜底关闭，语音识别将不可用"
        } else {
            "识别链路: " + chainParts.joinToString(" → ")
        }
    }

    override fun onResume() {
        super.onResume()
        refreshAsrChainSummary()
    }

    // ── AI 回复语音 ──────────────────────
    private fun setupTtsVoiceSection() {
        refreshTtsVoiceSummary()
        findViewById<Button>(R.id.ttsVoiceButton).setOnClickListener {
            VoiceTtsVoicePicker.show(
                context = this,
                onVoiceChanged = { refreshTtsVoiceSummary() },
                onPreviewVoice = { selected -> previewTtsVoice(selected) }
            )
        }
    }

    private fun refreshTtsVoiceSummary() {
        val selected = VoiceTtsVoiceCatalog.findById(VoiceTtsPreferences.getSelectedVoiceId(this))
        findViewById<TextView>(R.id.ttsVoiceSummary).text =
            "${selected.displayName} · ${selected.description}"
    }

    private fun previewTtsVoice(selected: VoiceTtsVoiceOption) {
        val speaker = voicePreviewSpeaker ?: VoiceSpeaker(this, respectUserToggle = false).also {
            voicePreviewSpeaker = it
        }
        speaker.speak(
            text = selected.previewText,
            profile = VoiceTtsEmotion.profileFor(""),
            voiceIdOverride = selected.id
        )
    }

    // ── 语音输入模式切换 ──────────────────────
    private fun setupVoiceModeToggle() {
        val group = findViewById<RadioGroup>(R.id.voiceModeGroup)
        val current = VoiceInputModeSettings.get(this)
        group.check(
            when (current) {
                VoiceInputMode.CLOUD_REALTIME -> R.id.voiceModeCloud
                VoiceInputMode.VOICE_MESSAGE  -> R.id.voiceModeVoiceMsg
                else                          -> R.id.voiceModeAgent
            }
        )
        group.setOnCheckedChangeListener { _, checkedId ->
            val mode = when (checkedId) {
                R.id.voiceModeCloud    -> VoiceInputMode.CLOUD_REALTIME
                R.id.voiceModeVoiceMsg -> VoiceInputMode.VOICE_MESSAGE
                else                   -> VoiceInputMode.LOCAL_AGENT_ASR
            }
            VoiceInputModeSettings.set(this, mode)
            val label = when (mode) {
                VoiceInputMode.LOCAL_AGENT_ASR -> "已切换：端上识别"
                VoiceInputMode.CLOUD_REALTIME  -> "已切换：云端直连"
                VoiceInputMode.VOICE_MESSAGE   -> "已切换：语音消息"
            }
            Toast.makeText(this, label, Toast.LENGTH_SHORT).show()
        }
    }

    // ── 模式切换 ──────────────────────────────
    private fun setupModeToggle() {
        val group = findViewById<RadioGroup>(R.id.modeGroup)
        val presetLayout = findViewById<LinearLayout>(R.id.presetLayout)
        val customLayout = findViewById<LinearLayout>(R.id.customLayout)

        group.setOnCheckedChangeListener { _, checkedId ->
            presetLayout.visibility = if (checkedId == R.id.modePreset) View.VISIBLE else View.GONE
            customLayout.visibility  = if (checkedId == R.id.modeCustom)  View.VISIBLE else View.GONE
        }
    }

    // ── 加载当前配置 ──────────────────────────
    private fun loadCurrentConfig() {
        scope.launch {
            try {
                val resp = withContext(Dispatchers.IO) {
                    http.newCall(Request.Builder()
                        .url("$SERVER_URL/api/user/$userId/agent")
                        .get().build()).execute()
                }
                val body = resp.body?.string() ?: return@launch
                val json = JSONObject(body)
                codexCliOnly = json.optBoolean("codex_cli_only", false)
                userByokApiEnabled = json.optBoolean("user_byok_api_enabled", false)

                if (codexCliOnly && !userByokApiEnabled) {
                    availableAgents = parseAgentOptions(json)
                        .ifEmpty { listOf(AgentOption("codex_cli", "Codex CLI")) }
                    setupSpinner()
                    applyCodexOnlyUi()
                    cacheModelSelection(null, "Codex CLI")
                    return@launch
                }

                // 解析可用代理列表
                availableAgents = parseAgentOptions(json)
                setupSpinner()

                // 填充当前配置
                val cfg = json.optJSONObject("config") ?: return@launch
                applyConfig(cfg)

            } catch (e: Exception) {
                Toast.makeText(this@SettingsActivity, "加载配置失败: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun parseAgentOptions(json: JSONObject): List<AgentOption> {
        val agentsArr: JSONArray = json.optJSONArray("available_agents") ?: JSONArray()
        return (0 until agentsArr.length()).map { i ->
            val a = agentsArr.getJSONObject(i)
            val name = a.getString("name")
            val provider = a.optString("provider", name)
            val model = a.optString("model", "")
            AgentOption(name, displayModelLabel(provider, model, a.optString("label", "")))
        }
    }

    private fun applyCodexOnlyUi() {
        val group = findViewById<RadioGroup>(R.id.modeGroup)
        group.check(R.id.modeDefault)
        findViewById<RadioButton>(R.id.modeDefault).apply {
            text = "Codex CLI（已锁定）"
            isEnabled = false
        }
        findViewById<RadioButton>(R.id.modePreset).isEnabled = false
        findViewById<RadioButton>(R.id.modeCustom).isEnabled = false
        findViewById<LinearLayout>(R.id.presetLayout).visibility = View.GONE
        findViewById<LinearLayout>(R.id.customLayout).visibility = View.GONE
        findViewById<Button>(R.id.saveButton).apply {
            text = "已锁定 Codex CLI"
            isEnabled = false
        }
    }

    private fun setupSpinner() {
        val spinner = findViewById<Spinner>(R.id.agentSpinner)
        val labels = availableAgents.map { it.label }
        spinner.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_item, labels).also {
            it.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
        }
    }

    private fun applyConfig(cfg: JSONObject) {
        val useAgent = jsonStringOrNull(cfg, "use_agent").orEmpty()
        val apiBase  = jsonStringOrNull(cfg, "api_base").orEmpty()
        val apiKey   = jsonStringOrNull(cfg, "api_key").orEmpty()
        val model    = jsonStringOrNull(cfg, "model").orEmpty()
        val nickname = jsonStringOrNull(cfg, "nickname").orEmpty()

        val group = findViewById<RadioGroup>(R.id.modeGroup)

        when {
            apiBase.isNotEmpty() || apiKey.isNotEmpty() -> {
                group.check(R.id.modeCustom)
                findViewById<EditText>(R.id.customBase).setText(apiBase)
                // 密钥不回填（安全起见）
                findViewById<EditText>(R.id.customModel).setText(model)
            }
            useAgent.isNotEmpty() -> {
                group.check(R.id.modePreset)
                val idx = availableAgents.indexOfFirst { it.name == useAgent }
                if (idx >= 0) findViewById<Spinner>(R.id.agentSpinner).setSelection(idx)
            }
            else -> group.check(R.id.modeDefault)
        }

        if (nickname.isNotEmpty()) {
            findViewById<EditText>(R.id.nicknameEdit).setText(nickname)
        }
    }

    // ── 保存配置 ──────────────────────────────
    private fun saveConfig() {
        if (codexCliOnly && !userByokApiEnabled) {
            Toast.makeText(this, "当前已锁定使用 Codex CLI", Toast.LENGTH_SHORT).show()
            return
        }
        val btn = findViewById<Button>(R.id.saveButton)
        btn.isEnabled = false
        btn.text = "保存中..."

        val group = findViewById<RadioGroup>(R.id.modeGroup)
        val nickname = findViewById<EditText>(R.id.nicknameEdit).text.toString().trim()

        val payload = JSONObject()
        var cachedAgentName: String? = null
        var cachedModelLabel = "服务器默认"

        when (group.checkedRadioButtonId) {
            R.id.modeDefault -> {
                // 清空所有自定义，回到服务器默认
                payload.put("use_agent", JSONObject.NULL)
                payload.put("api_base",  JSONObject.NULL)
                payload.put("api_key",   JSONObject.NULL)
                payload.put("model",     JSONObject.NULL)
            }
            R.id.modePreset -> {
                val idx = findViewById<Spinner>(R.id.agentSpinner).selectedItemPosition
                val selected = availableAgents.getOrNull(idx)
                if (selected == null) {
                    Toast.makeText(this, "请选择一个代理", Toast.LENGTH_SHORT).show()
                    btn.isEnabled = true; btn.text = "保存配置"
                    return
                }
                cachedAgentName = selected.name
                cachedModelLabel = selected.label
                payload.put("use_agent", selected.name)
                payload.put("api_base",  JSONObject.NULL)
                payload.put("api_key",   JSONObject.NULL)
                payload.put("model",     JSONObject.NULL)
            }
            R.id.modeCustom -> {
                val base  = findViewById<EditText>(R.id.customBase).text.toString().trim()
                val key   = findViewById<EditText>(R.id.customKey).text.toString().trim()
                val model = findViewById<EditText>(R.id.customModel).text.toString().trim()
                if (base.isEmpty() || model.isEmpty()) {
                    Toast.makeText(this, "请填写 API 地址和模型名称", Toast.LENGTH_SHORT).show()
                    btn.isEnabled = true; btn.text = "保存配置"
                    return
                }
                cachedModelLabel = "自定义模型"
                payload.put("use_agent", JSONObject.NULL)
                payload.put("api_base",  base)
                // 密钥为空则不修改服务端已保存的密钥
                if (key.isNotEmpty()) payload.put("api_key", key)
                payload.put("model",     model)
            }
        }

        if (nickname.isNotEmpty()) payload.put("nickname", nickname)

        scope.launch {
            try {
                val body = payload.toString().toRequestBody("application/json".toMediaType())
                val resp = withContext(Dispatchers.IO) {
                    http.newCall(Request.Builder()
                        .url("$SERVER_URL/api/user/$userId/agent")
                        .put(body).build()).execute()
                }
                val respBody = resp.body?.string() ?: ""
                if (resp.isSuccessful) {
                    cacheModelSelection(cachedAgentName, cachedModelLabel)
                    Toast.makeText(this@SettingsActivity, "✅ 配置已保存", Toast.LENGTH_SHORT).show()
                    setResult(Activity.RESULT_OK)
                    finish()
                } else {
                    val msg = runCatching { JSONObject(respBody).getString("error") }.getOrDefault(respBody)
                    Toast.makeText(this@SettingsActivity, "保存失败: $msg", Toast.LENGTH_LONG).show()
                    btn.isEnabled = true; btn.text = "保存配置"
                }
            } catch (e: Exception) {
                Toast.makeText(this@SettingsActivity, "网络错误: ${e.message}", Toast.LENGTH_SHORT).show()
                btn.isEnabled = true; btn.text = "保存配置"
            }
        }
    }

    private fun jsonStringOrNull(json: JSONObject, name: String): String? {
        if (!json.has(name) || json.isNull(name)) return null
        return json.optString(name, "")
            .trim()
            .takeIf { it.isNotBlank() && it != "null" }
    }

    private fun cacheModelSelection(agentName: String?, label: String) {
        prefs.edit().apply {
            if (agentName.isNullOrBlank()) remove("selected_agent_name")
            else putString("selected_agent_name", agentName)
            putString("selected_model_label", label)
        }.apply()
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == android.R.id.home) { finish(); return true }
        return super.onOptionsItemSelected(item)
    }

    override fun onDestroy() {
        super.onDestroy()
        voicePreviewSpeaker?.release()
        voicePreviewSpeaker = null
        scope.cancel()
    }
}
