package com.elon.app

import android.os.Bundle
import android.view.MenuItem
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import kotlinx.coroutines.*
import okhttp3.*
import org.json.JSONObject

/**
 * Token 用量统计页面
 *
 * 调用 GET /api/user/:user_id/usage/stats?days=N
 * 展示总量 / 按功能 / 按来源 / 按天的用量数据
 */
class TokenUsageActivity : AppCompatActivity() {

    private val SERVER_URL = "http://43.139.149.158:8080"
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val http = OkHttpClient()

    private lateinit var userId: String
    private lateinit var authToken: String

    private lateinit var loadingBar: ProgressBar
    private lateinit var errorText: TextView
    private lateinit var contentScroll: ScrollView
    private lateinit var periodSpinner: Spinner

    // 总量卡片
    private lateinit var totalTokensText: TextView
    private lateinit var inputTokensText: TextView
    private lateinit var cachedTokensText: TextView
    private lateinit var outputTokensText: TextView

    // 按功能
    private lateinit var byFeatureContainer: LinearLayout
    // 按来源
    private lateinit var byModeContainer: LinearLayout
    // 近 7 天
    private lateinit var byDayContainer: LinearLayout

    private val periodOptions = listOf(7, 14, 30, 90)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_token_usage)
        supportActionBar?.apply {
            title = "Token 用量统计"
            setDisplayHomeAsUpEnabled(true)
        }

        userId = AuthManager.effectiveUserId(this)
        authToken = AuthManager.token(this) ?: ""

        if (authToken.isBlank()) {
            Toast.makeText(this, "请先登录账号才能查看用量统计", Toast.LENGTH_LONG).show()
            finish()
            return
        }

        loadingBar = findViewById(R.id.usageLoadingBar)
        errorText = findViewById(R.id.usageErrorText)
        contentScroll = findViewById(R.id.usageContentScroll)
        periodSpinner = findViewById(R.id.usagePeriodSpinner)

        totalTokensText = findViewById(R.id.totalTokensText)
        inputTokensText = findViewById(R.id.inputTokensText)
        cachedTokensText = findViewById(R.id.cachedTokensText)
        outputTokensText = findViewById(R.id.outputTokensText)

        byFeatureContainer = findViewById(R.id.byFeatureContainer)
        byModeContainer = findViewById(R.id.byModeContainer)
        byDayContainer = findViewById(R.id.byDayContainer)

        val adapter = ArrayAdapter(this, android.R.layout.simple_spinner_item,
            periodOptions.map { "近 $it 天" })
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
        periodSpinner.adapter = adapter
        periodSpinner.setSelection(2) // 默认 30 天
        periodSpinner.onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
            override fun onItemSelected(p: AdapterView<*>?, v: View?, pos: Int, id: Long) {
                loadStats(periodOptions[pos])
            }
            override fun onNothingSelected(p: AdapterView<*>?) {}
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

    private fun loadStats(days: Int) {
        loadingBar.visibility = View.VISIBLE
        errorText.visibility = View.GONE
        contentScroll.visibility = View.GONE

        scope.launch {
            try {
                val resp = withContext(Dispatchers.IO) {
                    val req = Request.Builder()
                        .url("$SERVER_URL/api/user/$userId/usage/stats?days=$days")
                        .header("Authorization", "Bearer $authToken")
                        .get().build()
                    http.newCall(req).execute()
                }
                if (!resp.isSuccessful) {
                    showError("加载失败（${resp.code}），请稍后重试")
                    return@launch
                }
                val body = resp.body?.string() ?: ""
                val json = JSONObject(body)
                renderStats(json)
            } catch (e: Exception) {
                showError("网络错误：${e.message}")
            }
        }
    }

    private fun showError(msg: String) {
        loadingBar.visibility = View.GONE
        contentScroll.visibility = View.GONE
        errorText.visibility = View.VISIBLE
        errorText.text = msg
    }

    private fun renderStats(json: JSONObject) {
        loadingBar.visibility = View.GONE
        errorText.visibility = View.GONE
        contentScroll.visibility = View.VISIBLE

        // 总量
        val total = json.optJSONObject("total") ?: JSONObject()
        val totalTokens = total.optLong("total_tokens", 0)
        val inputTokens = total.optLong("input_tokens", 0)
        val cachedTokens = total.optLong("cached_input_tokens", 0)
        val outputTokens = total.optLong("output_tokens", 0)

        totalTokensText.text = formatCount(totalTokens)
        inputTokensText.text = formatCount(inputTokens)
        cachedTokensText.text = formatCount(cachedTokens)
        outputTokensText.text = formatCount(outputTokens)

        // 按功能
        byFeatureContainer.removeAllViews()
        val byFeature = json.optJSONArray("by_feature")
        if (byFeature != null && byFeature.length() > 0) {
            for (i in 0 until byFeature.length()) {
                val row = byFeature.getJSONObject(i)
                addRow(byFeatureContainer,
                    featureLabel(row.optString("feature")),
                    row.optLong("total_tokens", 0),
                    row.optLong("call_count", 0))
            }
        } else {
            addEmptyHint(byFeatureContainer)
        }

        // 按来源
        byModeContainer.removeAllViews()
        val byMode = json.optJSONArray("by_mode")
        if (byMode != null && byMode.length() > 0) {
            for (i in 0 until byMode.length()) {
                val row = byMode.getJSONObject(i)
                addRow(byModeContainer,
                    modeLabel(row.optString("usage_mode")),
                    row.optLong("total_tokens", 0),
                    row.optLong("call_count", 0))
            }
        } else {
            addEmptyHint(byModeContainer)
        }

        // 近 N 天趋势（最多 14 条，避免太长）
        byDayContainer.removeAllViews()
        val byDay = json.optJSONArray("by_day")
        if (byDay != null && byDay.length() > 0) {
            val limit = minOf(byDay.length(), 14)
            for (i in 0 until limit) {
                val row = byDay.getJSONObject(i)
                addRow(byDayContainer,
                    row.optString("date").takeLast(5), // MM-DD
                    row.optLong("total_tokens", 0),
                    row.optLong("call_count", 0))
            }
        } else {
            addEmptyHint(byDayContainer)
        }
    }

    private fun addRow(container: LinearLayout, label: String, tokens: Long, calls: Long) {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(0, 6, 0, 6)
        }
        val labelView = TextView(this).apply {
            text = label
            textSize = 13f
            setTextColor(0xFFCCCCCC.toInt())
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }
        val valueView = TextView(this).apply {
            text = "${formatCount(tokens)} tokens  ·  ${calls}次"
            textSize = 13f
            setTextColor(0xFFFFFFFF.toInt())
            gravity = android.view.Gravity.END
        }
        row.addView(labelView)
        row.addView(valueView)
        container.addView(row)
    }

    private fun addEmptyHint(container: LinearLayout) {
        val tv = TextView(this).apply {
            text = "暂无数据"
            textSize = 12f
            setTextColor(0xFF888888.toInt())
            setPadding(0, 6, 0, 6)
        }
        container.addView(tv)
    }

    private fun formatCount(n: Long): String = when {
        n >= 1_000_000 -> "%.1fM".format(n / 1_000_000.0)
        n >= 1_000     -> "%.1fK".format(n / 1_000.0)
        else           -> n.toString()
    }

    private fun featureLabel(key: String) = when (key) {
        "chat"                -> "💬 普通对话"
        "project_chat"        -> "📁 项目对话"
        "agent_tool"          -> "🔧 Agent 工具调用"
        "social_ai"           -> "🤝 @EL 社交回复"
        "social_ai_selected"  -> "🤝 长按 AI 回复"
        "speech_translate"    -> "🎤 语音翻译"
        "codex_cli_dev"       -> "🖥️ Codex 开发任务"
        "codex_cli_chat"      -> "🖥️ Codex 聊天模式"
        "client_reported"     -> "📱 客户端上报"
        else                  -> key
    }

    private fun modeLabel(key: String) = when (key) {
        "server_api_key"   -> "🔑 服务器 API Key"
        "server_codex_cli" -> "🖥️ 服务器 CLI"
        "client_reported"  -> "📱 客户端自行上报"
        else               -> key
    }
}
