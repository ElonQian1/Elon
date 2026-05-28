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
import com.elon.app.update.AppUpdateManager
import java.util.Locale

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
        loadCurrentConfig()

        findViewById<Button>(R.id.saveButton).setOnClickListener { saveConfig() }
        findViewById<Button>(R.id.checkUpdateButton).setOnClickListener {
            AppUpdateManager(this).manualCheck()
        }
        findViewById<Button>(R.id.manageEngineButton).setOnClickListener {
            startActivity(android.content.Intent(this, VoiceEngineActivity::class.java))
        }
    }

    // ── 语音输入模式切换 ──────────────────────
    private fun setupVoiceModeToggle() {
        val group = findViewById<RadioGroup>(R.id.voiceModeGroup)
        val current = VoiceInputModeSettings.get(this)
        group.check(
            if (current == VoiceInputMode.CLOUD_REALTIME) R.id.voiceModeCloud
            else R.id.voiceModeAgent
        )
        group.setOnCheckedChangeListener { _, checkedId ->
            val mode = if (checkedId == R.id.voiceModeCloud) {
                VoiceInputMode.CLOUD_REALTIME
            } else {
                VoiceInputMode.LOCAL_AGENT_ASR
            }
            VoiceInputModeSettings.set(this, mode)
            Toast.makeText(
                this,
                if (mode == VoiceInputMode.LOCAL_AGENT_ASR) "已切换：端上识别"
                else "已切换：云端直连",
                Toast.LENGTH_SHORT
            ).show()
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

                if (codexCliOnly) {
                    availableAgents = listOf(AgentOption("codex_cli", "Codex CLI"))
                    setupSpinner()
                    applyCodexOnlyUi()
                    cacheModelSelection(null, "Codex CLI")
                    return@launch
                }

                // 解析可用代理列表
                val agentsArr: JSONArray = json.optJSONArray("available_agents") ?: JSONArray()
                availableAgents = (0 until agentsArr.length()).map { i ->
                    val a = agentsArr.getJSONObject(i)
                    val name = a.getString("name")
                    val provider = a.optString("provider", name)
                    val model = a.optString("model", "")
                    AgentOption(name, displayModelLabel(provider, model, a.optString("label", "")))
                }
                setupSpinner()

                // 填充当前配置
                val cfg = json.optJSONObject("config") ?: return@launch
                applyConfig(cfg)

            } catch (e: Exception) {
                Toast.makeText(this@SettingsActivity, "加载配置失败: ${e.message}", Toast.LENGTH_SHORT).show()
            }
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

    private fun displayModelLabel(provider: String, model: String, rawLabel: String): String {
        val modelName = model.trim()
        if (modelName.isNotBlank() && !modelName.equals("default", ignoreCase = true)) {
            return friendlyModelName(modelName)
        }
        val fallback = if (modelName.isNotBlank()) "$provider [$modelName]" else provider
        return cleanModelLabel(rawLabel.ifBlank { fallback })
    }

    private fun cleanModelLabel(label: String): String {
        val withoutCopilot = label.trim().replace(Regex("^copilot\\s*/\\s*", RegexOption.IGNORE_CASE), "")
        val bracket = Regex("\\[([^\\]]+)\\]\\s*$").find(withoutCopilot)
        if (bracket != null) return friendlyModelName(bracket.groupValues[1].trim())
        if (withoutCopilot.contains("/")) {
            val tail = withoutCopilot.substringAfterLast("/").trim()
            if (tail.isNotBlank()) return friendlyModelName(tail)
        }
        return friendlyModelName(withoutCopilot)
    }

    private fun friendlyModelName(model: String): String {
        return when (model.trim().lowercase(Locale.US)) {
            "gpt-4o" -> "GPT-4o"
            "gpt-4o-mini" -> "GPT-4o mini"
            "gpt-4.1" -> "GPT-4.1"
            "gpt-4.5-preview" -> "GPT-4.5 Preview"
            "claude-3.5-sonnet", "claude-3-5-sonnet-20241022" -> "Claude 3.5 Sonnet"
            "claude-3.7-sonnet", "claude-3-7-sonnet-20250219" -> "Claude 3.7 Sonnet"
            "claude-sonnet-4", "claude-sonnet-4-5" -> "Claude Sonnet 4"
            "o1-mini" -> "o1 mini"
            "o1-preview" -> "o1 preview"
            "o3-mini" -> "o3 mini"
            "gemini-2.0-flash", "gemini-2.0-flash-001" -> "Gemini 2.0 Flash"
            "gemini-2.5-pro", "gemini-2.5-pro-preview" -> "Gemini 2.5 Pro"
            else -> model.trim()
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
        if (codexCliOnly) {
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
        scope.cancel()
    }
}
