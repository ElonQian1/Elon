// VoiceEngineActivity.kt — 语音识别引擎管理界面
// 功能：列出所有候选引擎、显示健康度、用户可单测某引擎、可选为偏好、可清空记录

package com.elon.app

import android.os.Bundle
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
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
        engines = RecognitionEngineSelector.list(this)
        val preferredKey = EnginePreference.getPreferredKey(this)
        var ok = 0
        var failed = 0
        var unknown = 0
        engines.forEach {
            when (EnginePreference.getHealth(this, it.key())) {
                EngineHealth.OK -> ok++
                EngineHealth.FAILED -> failed++
                else -> unknown++
            }
        }
        val preferredLabel = engines.firstOrNull { it.key() == preferredKey }?.label
        summary.text = buildString {
            append("共 ${engines.size} 个候选引擎 — ✅ $ok 正常 / ❌ $failed 失败 / ❓ $unknown 未测试")
            if (preferredLabel != null) append("\n当前偏好: $preferredLabel")
        }
        container.removeAllViews()
        engines.forEach { addEngineCard(it, preferredKey) }
    }

    private fun addEngineCard(engine: RecognitionEngine, preferredKey: String?) {
        val key = engine.key()
        val health = if (probingKeys.contains(key)) EngineHealth.PROBING
        else EnginePreference.getHealth(this, key)
        val isPreferred = preferredKey == key

        val pad = (12 * resources.displayMetrics.density).toInt()
        val card = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, pad, pad, pad)
            setBackgroundColor(if (isPreferred) 0xFFFFF8E1.toInt() else 0xFFFFFFFF.toInt())
            val lp = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT)
            lp.topMargin = (8 * resources.displayMetrics.density).toInt()
            layoutParams = lp
        }

        val title = TextView(this).apply {
            text = buildString {
                append(badge(health))
                append(" ")
                append(engine.label)
                if (isPreferred) append("  ⭐ 偏好")
            }
            textSize = 15f
            setTextColor(0xFF222222.toInt())
        }
        card.addView(title)

        val pkg = TextView(this).apply {
            text = engine.packageName + "  ·  " + healthText(health)
            textSize = 11f
            setTextColor(0xFF777777.toInt())
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
        }

        val btnRow = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            val lp = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT)
            lp.topMargin = (8 * resources.displayMetrics.density).toInt()
            layoutParams = lp
        }

        val probeBtn = Button(this).apply {
            text = if (health == EngineHealth.PROBING) "测试中…" else "测试"
            isEnabled = health != EngineHealth.PROBING
            textSize = 12f
            val lp = LinearLayout.LayoutParams(0, (40 * resources.displayMetrics.density).toInt(), 1f)
            lp.marginEnd = (6 * resources.displayMetrics.density).toInt()
            layoutParams = lp
            setOnClickListener { probeOne(engine) }
        }
        btnRow.addView(probeBtn)

        val preferBtn = Button(this).apply {
            text = if (isPreferred) "取消偏好" else "设为偏好"
            textSize = 12f
            val lp = LinearLayout.LayoutParams(0, (40 * resources.displayMetrics.density).toInt(), 1f)
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
