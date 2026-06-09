package com.elon.app

import android.app.AlertDialog
import android.content.Context
import android.graphics.Typeface
import android.os.Handler
import android.os.Looper
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.RadioButton
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast

internal object VoiceTtsVoicePicker {

    fun show(
        context: Context,
        onVoiceChanged: ((VoiceTtsVoiceOption) -> Unit)? = null,
        onPreviewVoice: ((VoiceTtsVoiceOption) -> Unit)? = null
    ) {
        val mainHandler = Handler(Looper.getMainLooper())

        // ── 容器视图（对话框内容区，后续可整体重建行）────────────────────────
        val rootLayout = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(context, 20), dp(context, 8), dp(context, 20), dp(context, 8))
        }

        // 状态栏
        val statusView = TextView(context).apply {
            textSize = 12f
            setPadding(dp(context, 8), dp(context, 6), dp(context, 8), dp(context, 6))
            text = "⏳ 正在从服务器读取声线列表…"
            alpha = 0.85f
            setBackgroundColor(0x1A58A6FF.toInt())
        }
        rootLayout.addView(statusView, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { bottomMargin = dp(context, 10) })

        // 副标题
        rootLayout.addView(TextView(context).apply {
            text = "可选择手机系统 TTS，也可试听服务器女声后设为默认。"
            textSize = 13f
            alpha = 0.72f
            setPadding(0, 0, 0, dp(context, 8))
        })

        // 声线行容器（可原地重建）
        val voiceRowsContainer = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
        }
        rootLayout.addView(voiceRowsContainer)

        val dialog = AlertDialog.Builder(context)
            .setTitle("选择 AI 回复语音")
            .setView(ScrollView(context).apply { addView(rootLayout) })
            .setNegativeButton("关闭", null)
            .create()
        dialog.show()

        // ── 构建/重建声线行 ─────────────────────────────────────────────────
        var currentVoices: List<VoiceTtsVoiceOption> = emptyList()
        var selectedVoiceId = VoiceTtsPreferences.getSelectedVoiceId(context)
        val radios = mutableMapOf<String, RadioButton>()
        val selectButtons = mutableMapOf<String, Button>()

        fun refreshRows() {
            currentVoices.forEach { option ->
                val selected = option.id == selectedVoiceId
                radios[option.id]?.isChecked = selected
                selectButtons[option.id]?.apply {
                    text = if (selected) "已设为默认" else "设为默认"
                    isEnabled = !selected
                }
            }
        }

        fun selectVoice(option: VoiceTtsVoiceOption) {
            if (option.id == selectedVoiceId) return
            VoiceTtsPreferences.setSelectedVoiceId(context, option.id)
            selectedVoiceId = option.id
            refreshRows()
            Toast.makeText(context, "已切换为：${option.displayName}", Toast.LENGTH_SHORT).show()
            onVoiceChanged?.invoke(option)
        }

        fun buildVoiceRows(voices: List<VoiceTtsVoiceOption>, workerConfigured: Boolean) {
            if (voices == currentVoices) return   // 内容无变化不重建
            currentVoices = voices
            radios.clear()
            selectButtons.clear()
            voiceRowsContainer.removeAllViews()

            voices.forEachIndexed { index, option ->
                val row = LinearLayout(context).apply {
                    orientation = LinearLayout.VERTICAL
                    setPadding(0, dp(context, 10), 0, dp(context, 10))
                    isClickable = true
                    setOnClickListener { selectVoice(option) }
                }
                val titleRow = LinearLayout(context).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = android.view.Gravity.CENTER_VERTICAL
                }
                val radio = RadioButton(context).apply { isClickable = false }
                radios[option.id] = radio
                titleRow.addView(radio)

                titleRow.addView(LinearLayout(context).apply {
                    orientation = LinearLayout.VERTICAL
                    addView(TextView(context).apply {
                        // 若 worker 未配置且不是系统TTS，用淡色暗示不可用
                        val unavailable = !workerConfigured && option.usesServerTts
                        text = option.displayName + if (unavailable) "  (引擎未配置)" else ""
                        textSize = 16f
                        typeface = Typeface.DEFAULT_BOLD
                        alpha = if (unavailable) 0.5f else 1.0f
                    })
                    addView(TextView(context).apply {
                        text = option.description
                        textSize = 13f
                        alpha = 0.72f
                    })
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                row.addView(titleRow)

                row.addView(LinearLayout(context).apply {
                    orientation = LinearLayout.HORIZONTAL
                    setPadding(dp(context, 48), dp(context, 6), 0, 0)
                    if (option.usesServerTts) {
                        addView(Button(context).apply {
                            text = "试听"
                            setOnClickListener {
                                onPreviewVoice?.invoke(option)
                                    ?: Toast.makeText(context, "试听仅在 APK 内支持", Toast.LENGTH_SHORT).show()
                            }
                        })
                    }
                    addView(Button(context).apply {
                        selectButtons[option.id] = this
                        setOnClickListener { selectVoice(option) }
                    }, LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { marginStart = dp(context, 8) })
                })
                voiceRowsContainer.addView(row)

                if (index != voices.lastIndex) {
                    voiceRowsContainer.addView(View(context).apply {
                        alpha = 0.18f
                        setBackgroundColor(0xFF8A8A8A.toInt())
                    }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(context, 1)))
                }
            }
            refreshRows()
        }

        // ── 状态栏文字更新 ──────────────────────────────────────────────────
        // systemVoice 始终排第一，服务器声线追加在后面
        fun voicesWithSystem(serverVoices: List<VoiceTtsVoiceOption>): List<VoiceTtsVoiceOption> =
            listOf(VoiceTtsVoiceCatalog.systemVoice) + serverVoices

        fun applyResult(result: TtsCatalogResult) {
            val statusText = when {
                result.isFallback && VoiceTtsCatalogFetcher.getCachedOrNull() == null ->
                    "⚠️ 无法连接服务器，使用内置声线预设（共 ${result.voices.size} 个）"
                result.isFallback ->
                    "⚠️ 服务器未响应，显示缓存数据"
                !result.workerConfigured ->
                    "⚠️ 服务器 TTS 引擎未配置 — 选服务器声线时将回退到系统 TTS"
                else -> {
                    val engine = result.defaultProvider
                        .replace("index_tts2", "IndexTTS2")
                        .replace("cosyvoice3", "CosyVoice3")
                    "✅ 服务器 TTS 就绪（${result.voices.size} 个声线，引擎：$engine）"
                }
            }
            mainHandler.post {
                if (!dialog.isShowing) return@post
                statusView.text = statusText
                buildVoiceRows(voicesWithSystem(result.voices), result.workerConfigured)
            }
        }

        // ── 立即用缓存初始化（有缓存则马上显示，无缓存先显示 preset + loading）─
        val cached = VoiceTtsCatalogFetcher.getCachedOrNull()
        if (cached != null) {
            VoiceTtsVoiceCatalog.updateFromServer(cached.voices)
            applyResult(cached)
        } else {
            // 无缓存：先用完整 allVoices（含系统TTS）占位，让用户不面对空列表
            mainHandler.post {
                if (dialog.isShowing) buildVoiceRows(VoiceTtsVoiceCatalog.allVoices, false)
            }
        }

        // ── 后台拉取服务器最新 catalog（5 分钟 TTL）──────────────────────────
        VoiceTtsCatalogFetcher.fetchIfStale(context) { result ->
            VoiceTtsVoiceCatalog.updateFromServer(result.voices)
            applyResult(result)
        }
    }

    private fun dp(context: Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()
}

