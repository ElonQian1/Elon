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
import org.json.JSONObject
import org.json.JSONArray

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

    private lateinit var userId: String
    private var availableAgents: List<Pair<String, String>> = emptyList() // name -> model

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        supportActionBar?.apply {
            title = "AI 代理设置"
            setDisplayHomeAsUpEnabled(true)
        }

        userId = getSharedPreferences("elon", MODE_PRIVATE)
            .getString("user_id", "") ?: ""

        findViewById<TextView>(R.id.userIdText).text = "用户 ID: $userId"

        setupModeToggle()
        loadCurrentConfig()

        findViewById<Button>(R.id.saveButton).setOnClickListener { saveConfig() }
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

                // 解析可用代理列表
                val agentsArr: JSONArray = json.optJSONArray("available_agents") ?: JSONArray()
                availableAgents = (0 until agentsArr.length()).map { i ->
                    val a = agentsArr.getJSONObject(i)
                    Pair(a.getString("name"), a.optString("model", ""))
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

    private fun setupSpinner() {
        val spinner = findViewById<Spinner>(R.id.agentSpinner)
        val labels = availableAgents.map { (name, model) ->
            if (model.isNotEmpty()) "$name  [$model]" else name
        }
        spinner.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_item, labels).also {
            it.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
        }
    }

    private fun applyConfig(cfg: JSONObject) {
        val useAgent = cfg.optString("use_agent", "")
        val apiBase  = cfg.optString("api_base", "")
        val apiKey   = cfg.optString("api_key", "")
        val model    = cfg.optString("model", "")
        val nickname = cfg.optString("nickname", "")

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
                val idx = availableAgents.indexOfFirst { it.first == useAgent }
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
        val btn = findViewById<Button>(R.id.saveButton)
        btn.isEnabled = false
        btn.text = "保存中..."

        val group = findViewById<RadioGroup>(R.id.modeGroup)
        val nickname = findViewById<EditText>(R.id.nicknameEdit).text.toString().trim()

        val payload = JSONObject()

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
                val selected = availableAgents.getOrNull(idx)?.first ?: ""
                if (selected.isEmpty()) {
                    Toast.makeText(this, "请选择一个代理", Toast.LENGTH_SHORT).show()
                    btn.isEnabled = true; btn.text = "保存配置"
                    return
                }
                payload.put("use_agent", selected)
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

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == android.R.id.home) { finish(); return true }
        return super.onOptionsItemSelected(item)
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
    }
}
