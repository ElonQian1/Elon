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
    private val startPreparedMessage: (String, String, JsonArray, SendTarget, List<ChatAttachment>) -> Unit
) {
    fun uploadAttachmentsThenSend(visibleText: String, outgoingText: String, target: SendTarget) {
        val attachments = pendingAttachments()
        setSendEnabled(false)
        DebugTraceStore.record(
            "ui_attachment_upload_start",
            mapOf("project_id" to target.projectId, "conversation_id" to target.conversationId, "count" to attachments.size)
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
                }
            )
            activity.runOnUiThread {
                if (refs == null) {
                    setSendEnabled(true)
                    return@runOnUiThread
                }
                DebugTraceStore.record(
                    "ui_attachment_upload_done",
                    mapOf(
                        "project_id" to target.projectId,
                        "conversation_id" to target.conversationId,
                        "count" to refs.size(),
                        "elapsed_ms" to (System.currentTimeMillis() - startedAt)
                    )
                )
                startPreparedMessage(visibleText, outgoingText, refs, target, chatAttachmentsFromRefs(refs))
            }
        }.start()
    }
}
