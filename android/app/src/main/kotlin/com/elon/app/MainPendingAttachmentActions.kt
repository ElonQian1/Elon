package com.elon.app

import android.net.Uri
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

internal class MainPendingAttachmentActions(
    private val activity: AppCompatActivity,
    private val pendingAttachments: MutableList<PendingAttachment>,
    private val isVoiceMode: () -> Boolean,
    private val setVoiceMode: (Boolean) -> Unit,
    private val applyVoiceMode: () -> Unit,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val refreshPendingAttachmentPreview: () -> Unit
) {
    fun attachPickedFile(kind: String, uri: Uri, fallbackName: String? = null) {
        val attachment = preparePickedAttachment(kind, uri, fallbackName) ?: return
        addPreparedAttachment(attachment)
    }

    fun preparePickedAttachment(
        kind: String,
        uri: Uri,
        fallbackName: String? = null,
        attachmentIndex: Int = pendingAttachments.size + 1
    ): PendingAttachment? {
        if (pendingAttachments.size >= MAX_PENDING_ATTACHMENTS) {
            Toast.makeText(activity, "一次最多发送 $MAX_PENDING_ATTACHMENTS 个附件", Toast.LENGTH_SHORT).show()
            return null
        }
        val name = fallbackName ?: displayNameForUri(activity, uri) ?: uri.lastPathSegment ?: kind
        return runCatching {
            copyAttachmentToCache(activity, kind, uri, name, attachmentIndex)
        }.onFailure {
            Toast.makeText(activity, "附件读取失败，请重新选择", Toast.LENGTH_SHORT).show()
        }.getOrNull()
    }

    fun addPreparedAttachment(attachment: PendingAttachment) {
        if (pendingAttachments.size >= MAX_PENDING_ATTACHMENTS) {
            runCatching { attachment.file.delete() }
            Toast.makeText(activity, "一次最多发送 $MAX_PENDING_ATTACHMENTS 个附件", Toast.LENGTH_SHORT).show()
            return
        }
        pendingAttachments.add(attachment)
        if (isVoiceMode()) {
            setVoiceMode(false)
            applyVoiceMode()
        }
        inputComposerMotion()?.let { motion ->
            if (!motion.isExpanded) {
                motion.setExpanded(true, animate = true)
            }
        }
        refreshPendingAttachmentPreview()
        Toast.makeText(activity, "已添加${attachment.displayLabel}：${attachment.displayName}", Toast.LENGTH_SHORT).show()
    }

    fun addPreparedAttachments(attachments: List<PendingAttachment>) {
        if (attachments.isEmpty()) return
        val available = MAX_PENDING_ATTACHMENTS - pendingAttachments.size
        if (available <= 0) {
            attachments.forEach { attachment -> runCatching { attachment.file.delete() } }
            Toast.makeText(activity, "一次最多发送 $MAX_PENDING_ATTACHMENTS 个附件", Toast.LENGTH_SHORT).show()
            return
        }
        val accepted = attachments.take(available)
        attachments.drop(available).forEach { attachment -> runCatching { attachment.file.delete() } }
        pendingAttachments.addAll(accepted)
        if (isVoiceMode()) {
            setVoiceMode(false)
            applyVoiceMode()
        }
        inputComposerMotion()?.let { motion ->
            if (!motion.isExpanded) {
                motion.setExpanded(true, animate = true)
            }
        }
        refreshPendingAttachmentPreview()
        Toast.makeText(activity, "已添加 ${accepted.size} 张图片", Toast.LENGTH_SHORT).show()
    }

    fun clearPendingAttachments(deleteFiles: Boolean = true) {
        if (deleteFiles) {
            pendingAttachments.forEach { attachment ->
                runCatching { attachment.file.delete() }
            }
        }
        pendingAttachments.clear()
        refreshPendingAttachmentPreview()
    }
}

internal const val MAX_PENDING_ATTACHMENTS = 9
