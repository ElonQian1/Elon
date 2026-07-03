package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.gson.JsonArray
import okhttp3.OkHttpClient

internal class MainAttachmentSendActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val userId: () -> String,
    private val pendingAttachments: () -> List<PendingAttachment>,
    private val setSendEnabled: (Boolean) -> Unit,
    private val appendMessage: (ChatMessage) -> Unit,
    private val updateMessage: (ChatMessage) -> Unit,
    private val startPreparedMessageAfterUserBubble:
        (String, String, JsonArray, SendTarget, List<ChatAttachment>, ProjectRequestExecutionMode) -> Unit
) {
    fun uploadAttachmentsThenSend(
        visibleText: String,
        outgoingText: String,
        target: SendTarget,
        executionMode: ProjectRequestExecutionMode
    ) {
        uploadPreparedAttachments(
            visibleText = visibleText,
            outgoingText = outgoingText,
            target = target,
            attachments = pendingAttachments(),
            existingMessage = null,
            executionMode = executionMode
        )
    }

    fun retryFailedAttachmentMessage(
        message: ChatMessage,
        visibleText: String,
        outgoingText: String,
        target: SendTarget
    ) {
        val attachments = pendingAttachmentsFromChatAttachments(message.attachments.orEmpty())
        if (attachments.isEmpty()) {
            message.sendStatus = "原图已失效，请重新选择"
            updateMessage(message)
            Toast.makeText(activity, "原图已失效，请重新选择图片", Toast.LENGTH_SHORT).show()
            return
        }
        uploadPreparedAttachments(
            visibleText = visibleText,
            outgoingText = outgoingText,
            target = target,
            attachments = attachments,
            existingMessage = message,
            executionMode = ProjectRequestExecutionMode.Execute
        )
    }

    private fun uploadPreparedAttachments(
        visibleText: String,
        outgoingText: String,
        target: SendTarget,
        attachments: List<PendingAttachment>,
        existingMessage: ChatMessage?,
        executionMode: ProjectRequestExecutionMode
    ) {
        setSendEnabled(false)
        val optimisticMessage = existingMessage ?: ChatMessage(role = "user", content = visibleText)
        optimisticMessage.content = visibleText
        optimisticMessage.attachments = chatAttachmentsFromPending(attachments).takeIf { it.isNotEmpty() }
        optimisticMessage.sendStatus = "发送中..."
        if (existingMessage == null) {
            appendMessage(optimisticMessage)
        } else {
            updateMessage(optimisticMessage)
        }
        val updateUploadStatus = uploadStatusUpdater(optimisticMessage)
        DebugTraceStore.record(
            "ui_attachment_upload_start",
            mapOf(
                "project_id" to target.projectId,
                "conversation_id" to target.conversationId,
                "count" to attachments.size,
                "retry" to (existingMessage != null)
            )
        )
        Thread {
            val startedAt = System.currentTimeMillis()
            val refs = uploadAttachmentRefsOrNull(
                http = http,
                serverUrl = serverUrl,
                userId = userId(),
                attachments = attachments,
                target = target,
                maxAttachmentBytes = MAX_ATTACHMENT_BYTES,
                showShortToast = { message ->
                    activity.runOnUiThread { Toast.makeText(activity, message, Toast.LENGTH_SHORT).show() }
                },
                showLongToast = { message ->
                    activity.runOnUiThread { Toast.makeText(activity, message, Toast.LENGTH_LONG).show() }
                },
                onProgress = { progress ->
                    updateUploadStatus(uploadProgressText(progress), false)
                }
            )
            activity.runOnUiThread {
                if (refs == null) {
                    optimisticMessage.sendStatus = "发送失败，点此重试"
                    updateMessage(optimisticMessage)
                    setSendEnabled(true)
                    return@runOnUiThread
                }
                val localAttachments = chatAttachmentsFromPending(attachments)
                val uploadedAttachments = chatAttachmentsFromRefs(refs)
                    .withMissingImageAnnotationsFrom(localAttachments)
                optimisticMessage.content = visibleText
                optimisticMessage.attachments = uploadedAttachments.takeIf { it.isNotEmpty() }
                optimisticMessage.sendStatus = null
                updateMessage(optimisticMessage)
                DebugTraceStore.record(
                    "ui_attachment_upload_done",
                    mapOf(
                        "project_id" to target.projectId,
                        "conversation_id" to target.conversationId,
                        "count" to refs.size(),
                        "elapsed_ms" to (System.currentTimeMillis() - startedAt)
                    )
                )
                startPreparedMessageAfterUserBubble(
                    visibleText,
                    outgoingText,
                    refs,
                    target,
                    uploadedAttachments,
                    executionMode
                )
            }
        }.start()
    }

    private fun uploadStatusUpdater(message: ChatMessage): (String, Boolean) -> Unit {
        var lastStatus = ""
        var lastUpdatedAt = 0L
        return updater@{ status, force ->
            val now = System.currentTimeMillis()
            if (!force && status == lastStatus) return@updater
            if (!force && now - lastUpdatedAt < 350L) return@updater
            lastStatus = status
            lastUpdatedAt = now
            activity.runOnUiThread {
                message.sendStatus = status
                updateMessage(message)
            }
        }
    }

    private fun uploadProgressText(progress: AttachmentUploadProgress): String {
        val prefix = if (progress.attachmentCount > 1) {
            "正在上传 ${progress.attachmentIndex}/${progress.attachmentCount}"
        } else {
            "正在上传"
        }
        return if (progress.totalBytes > 0L) {
            "$prefix，${progress.percent}%"
        } else {
            prefix
        }
    }
}
