package com.elon.app

import android.graphics.Color
import android.widget.TextView
import com.elon.app.databinding.ActivityMainBinding

internal class MainCollapsedInputPreviewActions(
    private val binding: ActivityMainBinding,
    private val pendingAttachments: () -> List<PendingAttachment>,
    private val collapsedInputPreview: () -> TextView?
) {
    fun updateCollapsedInputPreview() {
        val preview = collapsedInputPreview() ?: return
        val draft = binding.inputEdit.text?.toString().orEmpty()
        val attachments = pendingAttachments()
        val hasDraft = draft.isNotBlank()
        val hasAttachments = attachments.isNotEmpty()
        preview.text = when {
            hasDraft -> draft
            hasAttachments -> pendingAttachmentSummary(attachments)
            else -> "输入内容"
        }
        preview.setTextColor(
            Color.parseColor(if (hasDraft || hasAttachments) "#DCDCDC" else "#5E5E5E")
        )
    }
}
