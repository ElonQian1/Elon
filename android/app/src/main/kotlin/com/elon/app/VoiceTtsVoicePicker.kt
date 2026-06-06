package com.elon.app

import android.app.AlertDialog
import android.content.Context
import android.graphics.Typeface
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
        val voices = VoiceTtsVoiceCatalog.allVoices
        var selectedVoiceId = VoiceTtsPreferences.getSelectedVoiceId(context)
        val radios = mutableMapOf<String, RadioButton>()
        val selectButtons = mutableMapOf<String, Button>()

        val list = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(context, 20), dp(context, 8), dp(context, 20), 0)
        }
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
            val radio = RadioButton(context).apply {
                isClickable = false
            }
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
                ).apply {
                    marginStart = dp(context, 10)
                })
            })
            list.addView(row)
            if (index != voices.lastIndex) {
                list.addView(View(context).apply {
                    alpha = 0.18f
                    setBackgroundColor(0xFF8A8A8A.toInt())
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(context, 1)
                ))
            }
        }

        refreshRows()
        AlertDialog.Builder(context)
            .setTitle("选择 AI 回复语音")
            .setView(ScrollView(context).apply { addView(list) })
            .setNegativeButton("关闭", null)
            .show()
    }

    private fun dp(context: Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()
}
