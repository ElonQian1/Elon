// VoiceEngineActivity.kt — 语音识别引擎管理界面
// 功能：列出所有候选引擎、显示健康度、用户可单测某引擎、可选为偏好、可排除/清空记录

package com.elon.app

import android.os.Bundle
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import android.speech.SpeechRecognizer
import com.elon.app.agent.infrastructure.voice.EngineHealth
import com.elon.app.agent.infrastructure.voice.EnginePreference
import com.elon.app.agent.infrastructure.voice.EngineProbe
import com.elon.app.agent.infrastructure.voice.RecognitionEngine
import com.elon.app.agent.infrastructure.voice.RecognitionEngineSelector

class VoiceEngineActivity : AppCompatActivity() {

    private lateinit var container: LinearLayout
    private lateinit var summary: TextView
    private lateinit var probeAllBtn: Button
    private lateinit var clearBtn: Button
    private lateinit var languageSpinner: Spinner
    private lateinit var beamSizeSpinner: Spinner
    private lateinit var vadFilterSwitch: Switch
    private lateinit var conditionSwitch: Switch
    private var engines: List<RecognitionEngine> = emptyList()
    private val probingKeys = HashSet<String>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_voice_engine)
        supportActionBar?.apply {
            title = "语音引擎管理"
            setDisplayHomeAsUpEnabled(true)
        }

        container = findViewById(R.id.enginesContainer)
        summary = findViewById(R.id.engineSummary)
        probeAllBtn = findViewById(R.id.probeAllButton)
        clearBtn = findViewById(R.id.clearHealthButton)
        languageSpinner = findViewById(R.id.whisperLanguageSpinner)
        beamSizeSpinner = findViewById(R.id.whisperBeamSizeSpinner)
        vadFilterSwitch = findViewById(R.id.whisperVadFilterSwitch)
        conditionSwitch = findViewById(R.id.whisperConditionSwitch)

        // ── 语言选择器 ──
        val langLabels = listOf("简体中文", "繁体中文", "英文 (English)", "自动检测")
        val langCodes  = listOf("zh", "zh-TW", "en", "auto")
        val adapter = ArrayAdapter(this, android.R.layout.simple_spinner_item, langLabels)
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
        languageSpinner.adapter = adapter
        val savedCode = AsrFallbackSettings.getWhisperLanguage(this)
        languageSpinner.setSelection(langCodes.indexOf(savedCode).let { if (it < 0) langCodes.indexOf("auto") else it })
        languageSpinner.onItemSelectedListener = object : android.widget.AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent: android.widget.AdapterView<*>?, view: android.view.View?, position: Int, id: Long) {
                AsrFallbackSettings.setWhisperLanguage(this@VoiceEngineActivity, langCodes[position])
            }
            override fun onNothingSelected(parent: android.widget.AdapterView<*>?) {}
        }

        // ── beam_size 选择器 ──
        val beamLabels = listOf("1 — 最快（贪心）", "3 — 较快", "5 — 平衡（推荐）", "7 — 较准", "10 — 最准")
        val beamValues = listOf(1, 3, 5, 7, 10)
        val beamAdapter = ArrayAdapter(this, android.R.layout.simple_spinner_item, beamLabels)
        beamAdapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
        beamSizeSpinner.adapter = beamAdapter
        val savedBeam = AsrFallbackSettings.getWhisperBeamSize(this)
        beamSizeSpinner.setSelection(beamValues.indexOf(savedBeam).let { if (it < 0) beamValues.indexOf(5) else it })
        beamSizeSpinner.onItemSelectedListener = object : android.widget.AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent: android.widget.AdapterView<*>?, view: android.view.View?, position: Int, id: Long) {
                AsrFallbackSettings.setWhisperBeamSize(this@VoiceEngineActivity, beamValues[position])
            }
            override fun onNothingSelected(parent: android.widget.AdapterView<*>?) {}
        }

        // ── vad_filter 开关 ──
        vadFilterSwitch.isChecked = AsrFallbackSettings.getWhisperVadFilter(this)
        vadFilterSwitch.setOnCheckedChangeListener { _, checked ->
            AsrFallbackSettings.setWhisperVadFilter(this, checked)
        }

        // ── condition_on_previous_text 开关 ──
        conditionSwitch.isChecked = AsrFallbackSettings.getWhisperConditionOnPrevious(this)
        conditionSwitch.setOnCheckedChangeListener { _, checked ->
            AsrFallbackSettings.setWhisperConditionOnPrevious(this, checked)
        }

        probeAllBtn.setOnClickListener { probeAll() }
        clearBtn.setOnClickListener {
            EnginePreference.clearAllHealth(this)
            Toast.makeText(this, "已清空测试记录", Toast.LENGTH_SHORT).show()
            refresh()
        }

        refresh()
    }

    override fun onSupportNavigateUp(): Boolean {
        finish(); return true
    }

    private fun refresh() {
        // 所有引擎（用于展示）
        engines = RecognitionEngineSelector.list(this)
        // 实际使用顺序（含偏好/健康度排序）
        val orderedForUse = RecognitionEngineSelector.listForUse(this)
        val preferredKey = EnginePreference.getPreferredKey(this)
        val disabledKeys = AsrFallbackSettings.getDisabledEngineKeys(this)
        var ok = 0; var failed = 0; var unknown = 0
        engines.forEach {
            when (EnginePreference.getHealth(this, it.key())) {
                EngineHealth.OK -> ok++
                EngineHealth.FAILED -> failed++
                else -> unknown++
            }
        }
        // 构建回退链摘要（只显示未被排除的引擎）
        val chainLabels = orderedForUse
            .filter { it.key() !in disabledKeys }
            .mapIndexed { i, e -> "#${i + 1} ${e.label.take(18)}" }
        summary.text = buildString {
            append("共 ${engines.size} 个引擎 — ✅ $ok / ❌ $failed / ❓ $unknown")
            if (disabledKeys.isNotEmpty()) append("  (${disabledKeys.size} 已排除)")
            append("\n\n回退链: ")
            if (chainLabels.isEmpty()) {
                append("⚠️ 全部引擎已排除，将只走云端兜底")
            } else {
                append(chainLabels.joinToString(" → "))
            }
        }
        container.removeAllViews()
        // 按实际使用顺序展示（排除的放最后）
        val active = orderedForUse.filter { it.key() !in disabledKeys }
        val excluded = orderedForUse.filter { it.key() in disabledKeys }
        (active + excluded).forEach { addEngineCard(it, preferredKey, disabledKeys, active.indexOf(it) + 1) }
    }

    private fun addEngineCard(engine: RecognitionEngine, preferredKey: String?, disabledKeys: Set<String>, orderIndex: Int) {
        val key = engine.key()
        val isDisabled = key in disabledKeys
        val health = if (probingKeys.contains(key)) EngineHealth.PROBING
        else EnginePreference.getHealth(this, key)
        val isPreferred = preferredKey == key

        val dp = resources.displayMetrics.density
        val pad = (12 * dp).toInt()
        val bgColor = when {
            isDisabled  -> 0xFF1A1A1A.toInt()
            isPreferred -> 0xFF1C2B1C.toInt()
            else        -> 0xFF1E1E1E.toInt()
        }
        val card = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, pad, pad, pad)
            setBackgroundColor(bgColor)
            val lp = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT)
            lp.topMargin = (8 * dp).toInt()
            layoutParams = lp
        }

        // ── 标题行：序号 + 名称 + 偏好徽标 ──
        val orderBadge = if (isDisabled) "  [排除]" else "  #$orderIndex"
        val title = TextView(this).apply {
            text = buildString {
                append(badge(health))
                append(" ")
                append(engine.label)
                append(orderBadge)
                if (isPreferred) append("  ⭐")
            }
            textSize = 15f
            setTextColor(if (isDisabled) 0xFF555555.toInt() else 0xFFEFEFEF.toInt())
        }
        card.addView(title)

        val pkg = TextView(this).apply {
            text = engine.packageName + "  ·  " + healthText(health)
            textSize = 11f
            setTextColor(if (isDisabled) 0xFF444444.toInt() else 0xFF888888.toInt())
        }
        card.addView(pkg)

        val err = EnginePreference.getLastError(this, key)
        if (err != null && health == EngineHealth.FAILED) {
            val errView = TextView(this).apply {
                text = "上次错误: code=${err.first} ${err.second}"
                textSize = 11f
                setTextColor(0xFFD32F2F.toInt())
            }
            card.addView(errView)

            // 系统常驻语音助手提示：仅在引擎未被排除时显示（排除后卡片已标灰，不再重复提醒）
            val isAlwaysOnBusy = err.first == SpeechRecognizer.ERROR_RECOGNIZER_BUSY ||
                (err.first == SpeechRecognizer.ERROR_CLIENT &&
                    (engine.packageName.contains("magicvoice", ignoreCase = true) ||
                     engine.packageName.contains("bixby", ignoreCase = true)))
            if (isAlwaysOnBusy && !isDisabled) {
                val tipView = TextView(this).apply {
                    text = "⚠️ 被系统语音助手常驻占用，建议「排除」或到手机「设置 → 智慧语音 → 语音唤醒」关闭。"
                    textSize = 11f
                    setTextColor(0xFFFF8F00.toInt())
                    val lp = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT)
                    lp.topMargin = (4 * dp).toInt()
                    layoutParams = lp
                }
                card.addView(tipView)
            }
        }

        // ── 操作行 ──
        val btnRow = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            val lp = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT)
            lp.topMargin = (8 * dp).toInt()
            layoutParams = lp
        }

        // 测试按钮（禁用状态下置灰）
        val probeBtn = Button(this).apply {
            text = if (health == EngineHealth.PROBING) "测试中…" else "测试"
            isEnabled = health != EngineHealth.PROBING && !isDisabled
            textSize = 12f
            val lp = LinearLayout.LayoutParams(0, (40 * dp).toInt(), 1f)
            lp.marginEnd = (4 * dp).toInt()
            layoutParams = lp
            setOnClickListener { probeOne(engine) }
        }
        btnRow.addView(probeBtn)

        // 偏好按钮
        val preferBtn = Button(this).apply {
            text = if (isPreferred) "取消偏好" else "设为偏好"
            isEnabled = !isDisabled
            textSize = 12f
            val lp = LinearLayout.LayoutParams(0, (40 * dp).toInt(), 1f)
            lp.marginEnd = (4 * dp).toInt()
            layoutParams = lp
            setOnClickListener {
                if (isPreferred) {
                    EnginePreference.setPreferredKey(this@VoiceEngineActivity, null)
                    Toast.makeText(this@VoiceEngineActivity, "已取消偏好", Toast.LENGTH_SHORT).show()
                } else {
                    EnginePreference.setPreferredKey(this@VoiceEngineActivity, key)
                    Toast.makeText(this@VoiceEngineActivity, "已设为偏好引擎", Toast.LENGTH_SHORT).show()
                }
                refresh()
            }
        }
        btnRow.addView(preferBtn)

        // 排除/恢复按钮
        val excludeBtn = Button(this).apply {
            text = if (isDisabled) "恢复" else "排除"
            textSize = 12f
            setBackgroundColor(if (isDisabled) 0xFF37474F.toInt() else 0xFF6D1010.toInt())
            setTextColor(0xFFFFFFFF.toInt())
            val lp = LinearLayout.LayoutParams(0, (40 * dp).toInt(), 0.8f)
            layoutParams = lp
            setOnClickListener {
                val nowDisabled = !isDisabled
                // 阻止用户排除所有引擎
                if (nowDisabled) {
                    val disabledCount = AsrFallbackSettings.getDisabledEngineKeys(this@VoiceEngineActivity).size
                    val totalActive = engines.size
                    if (disabledCount + 1 >= totalActive) {
                        Toast.makeText(this@VoiceEngineActivity, "至少保留一个引擎", Toast.LENGTH_SHORT).show()
                        return@setOnClickListener
                    }
                }
                AsrFallbackSettings.setEngineDisabled(this@VoiceEngineActivity, key, nowDisabled)
                val msg = if (nowDisabled) "已排除「${engine.label}」" else "已恢复「${engine.label}」"
                Toast.makeText(this@VoiceEngineActivity, msg, Toast.LENGTH_SHORT).show()
                refresh()
            }
        }
        btnRow.addView(excludeBtn)

        card.addView(btnRow)
        container.addView(card)
    }

    private fun probeOne(engine: RecognitionEngine) {
        val key = engine.key()
        probingKeys.add(key)
        EnginePreference.setHealth(this, key, EngineHealth.PROBING)
        refresh()
        EngineProbe.probe(this, engine) { result ->
            probingKeys.remove(key)
            EnginePreference.setHealth(this, result.key, result.health, result.errorCode, result.errorMessage)
            Toast.makeText(
                this,
                if (result.health == EngineHealth.OK) "✅ ${engine.label} 正常"
                else "❌ ${engine.label}: ${result.errorMessage ?: "未知错误"}",
                Toast.LENGTH_SHORT
            ).show()
            refresh()
        }
    }

    private fun probeAll() {
        if (engines.isEmpty()) return
        probeAllBtn.isEnabled = false
        probeAllBtn.text = "测试中…(0/${engines.size})"
        engines.forEach {
            probingKeys.add(it.key())
            EnginePreference.setHealth(this, it.key(), EngineHealth.PROBING)
        }
        refresh()
        var done = 0
        EngineProbe.probeAll(this, engines,
            onEach = { result ->
                probingKeys.remove(result.key)
                EnginePreference.setHealth(this, result.key, result.health, result.errorCode, result.errorMessage)
                done += 1
                probeAllBtn.text = "测试中…($done/${engines.size})"
                refresh()
            },
            onDone = {
                probeAllBtn.text = "测试全部引擎"
                probeAllBtn.isEnabled = true
                Toast.makeText(this, "测试完成", Toast.LENGTH_SHORT).show()
                refresh()
            }
        )
    }

    private fun badge(h: EngineHealth): String = when (h) {
        EngineHealth.OK -> "✅"
        EngineHealth.FAILED -> "❌"
        EngineHealth.PROBING -> "⏳"
        EngineHealth.UNKNOWN -> "❓"
    }

    private fun healthText(h: EngineHealth): String = when (h) {
        EngineHealth.OK -> "已通过测试"
        EngineHealth.FAILED -> "测试失败"
        EngineHealth.PROBING -> "正在测试"
        EngineHealth.UNKNOWN -> "未测试"
    }
}
