package com.elon.app

import android.app.AlertDialog
import android.content.Context
import android.widget.Toast

internal object VoiceTtsVoicePicker {
    fun show(
        context: Context,
        onVoiceChanged: ((VoiceTtsVoiceOption) -> Unit)? = null
    ) {
        val voices = VoiceTtsVoiceCatalog.presetVoices
        val currentVoiceId = VoiceTtsPreferences.getSelectedVoiceId(context)
        val checkedIndex = voices.indexOfFirst { it.id == currentVoiceId }.takeIf { it >= 0 } ?: 0
        val labels = voices.map { "${it.displayName}\n${it.description}" }.toTypedArray()

        AlertDialog.Builder(context)
            .setTitle("选择 AI 女声")
            .setSingleChoiceItems(labels, checkedIndex) { dialog, which ->
                val selected = voices[which]
                VoiceTtsPreferences.setSelectedVoiceId(context, selected.id)
                Toast.makeText(context, "已切换为：${selected.displayName}", Toast.LENGTH_SHORT).show()
                onVoiceChanged?.invoke(selected)
                dialog.dismiss()
            }
            .setNegativeButton("取消", null)
            .show()
    }
}
