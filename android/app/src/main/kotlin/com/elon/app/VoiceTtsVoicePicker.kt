package com.elon.app

import android.app.AlertDialog
import android.content.Context
import android.graphics.Typeface
import android.os.Handler
import android.os.Looper
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
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

        // 状态栏视图（在声线列表上方），后续会根据 catalog 结果更新
        val statusView = TextView(context).apply {
            textSize = 12f
            setPadding(0, 0, 0, dp(context, 10))
            text = "⏳ 正在检查服务器声线状态…"
            alpha = 0.8f
        }

        // 立即用缓存或内置预设构建对话框，不阻塞 UI
        val initialVoices = VoiceTtsCatalogFetcher.getCachedOrNull()?.let {
            VoiceTtsVoiceCatalog.updateFromServer(it.voices)
            it
        }

        val dialog = buildDialog(
            context = context,
            statusView = statusView,
            onVoiceChanged = onVoiceChanged,
            onPreviewVoice = onPreviewVoice,
        )
        dialog.show()

        // 更新状态栏文字（从任意线程安全调用）
        fun applyStatus(result: TtsCatalogResult) {
            val statusText = when {
                result.isFallback && initialVoices == null ->
                    "⚠️ 无法连接服务器，显示内置声线预设"
                result.isFallback ->
                    "⚠️ 使用缓存数据（服务器未响应）"
                !result.workerConfigured ->
                    "⚠️ 服务器 TTS 引擎未配置，选服务器声线时将回退到系统 TTS"
                else -> {
                    val engine = result.defaultProvider
                        .replace("index_tts2", "IndexTTS2")
                        .replace("cosyvoice3", "CosyVoice3")
                    "✅ 服务器 TTS 就绪（${result.voices.size} 个声线，引擎：$engine）"
                }
            }
            mainHandler.post {
                if (dialog.isShowing) {
                    statusView.text = statusText
                }
            }
        }

        // 如果已有缓存立即显示状态
        if (initialVoices != null) applyStatus(initialVoices)

        // 后台拉取最新 catalog（5 分钟 TTL，通常不会重复请求）
        VoiceTtsCatalogFetcher.fetchIfStale(context) { result ->
            VoiceTtsVoiceCatalog.updateFromServer(result.voices)
            applyStatus(result)
        }
    }

    // ── 构建对话框 ──────────────────────────────────────────────────────────

    private fun buildDialog(
        context: Context,
        statusView: TextView,
        onVoiceChanged: ((VoiceTtsVoiceOption) -> Unit)?,
        onPreviewVoice: ((VoiceTtsVoiceOption) -> Unit)?,
    ): AlertDialog {
        val voices = VoiceTtsVoiceCatalog.allVoices
        var selectedVoiceId = VoiceTtsPreferences.getSelectedVoiceId(context)
        val radios = mutableMapOf<String, RadioButton>()
        val selectButtons = mutableMapOf<String, Button>()

        val list = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(context, 20), dp(context, 8), dp(context, 20), 0)
        }

        list.addView(statusView)

        list.addView(TextView(context).apply {
            text = "可选择手机系统 TTS，也可试听服务器女声预设后设为默认。"
            textSize = 13f
            alpha = 0.72f
            setPadding(0, 0, 0, dp(context, 10))
        })

        fun refreshRows() {
            voices.forEach { option ->
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
                    text = option.displayName
                    textSize = 16f
                    typeface = Typeface.DEFAULT_BOLD
                })
                addView(TextView(context).apply {
                    text = option.description
                    textSize = 13f
                    alpha = 0.78f
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            row.addView(titleRow)

            row.addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                setPadding(dp(context, 48), dp(context, 8), 0, 0)
                addView(Button(context).apply {
                    text = "试听"
                    setOnClickListener {
                        onPreviewVoice?.invoke(option)
                            ?: Toast.makeText(context, "请在 APK 中试听女声", Toast.LENGTH_SHORT).show()
                    }
                })
                addView(Button(context).apply {
                    selectButtons[option.id] = this
                    setOnClickListener { selectVoice(option) }
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { marginStart = dp(context, 10) })
            })
            list.addView(row)
            if (index != voices.lastIndex) {
                list.addView(View(context).apply {
                    alpha = 0.18f
                    setBackgroundColor(0xFF8A8A8A.toInt())
                }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(context, 1)))
            }
        }

        refreshRows()

        return AlertDialog.Builder(context)
            .setTitle("选择 AI 回复语音")
            .setView(ScrollView(context).apply { addView(list) })
            .setNegativeButton("关闭", null)
            .create()
    }

    private fun dp(context: Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()
}

