package com.elon.app

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject

internal const val CHAT_ATTACHMENT_TARGET_ID = "__chat_attachments__"

internal fun uploadAttachmentRefsOrNull(
    http: OkHttpClient,
    serverUrl: String,
    userId: String,
    attachments: List<PendingAttachment>,
    target: SendTarget,
    maxAttachmentBytes: Int,
    showShortToast: (String) -> Unit,
    showLongToast: (String) -> Unit,
    onProgress: (AttachmentUploadProgress) -> Unit = {}
): JsonArray? {
    val array = JsonArray()
    for ((index, attachment) in attachments.withIndex()) {
        if (!attachment.file.exists()) {
            showShortToast("附件已失效，请重新选择：${attachment.displayName}")
            return null
        }
        if (attachment.file.length() > maxAttachmentBytes) {
            showShortToast("附件过大，请重新选择较小文件：${attachment.displayName}")
            return null
        }
        val url = attachmentUploadUrl(serverUrl, userId, target, attachment)
        val mediaType = attachment.mimeType.toMediaTypeOrNull()
            ?: "application/octet-stream".toMediaType()
        val response = try {
            val body = AttachmentUploadProgressBody(
                file = attachment.file,
                mediaType = mediaType,
                attachmentIndex = index + 1,
                attachmentCount = attachments.size,
                onProgress = onProgress
            )
            http.newCall(
                Request.Builder()
                    .url(url)
                    .post(body)
                    .build()
            ).execute()
        } catch (e: Exception) {
            showLongToast("附件上传失败：${e.message}")
            DebugTraceStore.record(
                "ui_attachment_upload_failed",
                mapOf("project_id" to target.projectId, "file" to attachment.displayName, "error" to e.message)
            )
            return null
        }
        response.use {
            val body = it.body?.string().orEmpty()
            if (!it.isSuccessful) {
                showLongToast("附件上传失败：HTTP ${it.code}")
                DebugTraceStore.record(
                    "ui_attachment_upload_failed",
                    mapOf("project_id" to target.projectId, "file" to attachment.displayName, "http_code" to it.code)
                )
                return null
            }
            val uploaded = runCatching { JSONObject(body).optJSONObject("attachment") }.getOrNull()
            if (uploaded == null) {
                showLongToast("附件上传响应异常：${attachment.displayName}")
                return null
            }
            array.add(JsonObject().apply {
                uploaded.optString("attachment_id", "").takeIf { it.isNotBlank() }?.let {
                    addProperty("attachment_id", it)
                }
                addProperty("kind", uploaded.optString("kind", attachment.kind))
                addProperty("display_name", uploaded.optString("display_name", attachment.displayName))
                addProperty("file_name", uploaded.optString("file_name", attachment.fileName))
                addProperty("mime_type", uploaded.optString("mime_type", attachment.mimeType))
                addProperty("path", uploaded.optString("path", ""))
                addProperty("local_path", attachment.file.absolutePath)
                uploaded.optString("url", "").takeIf { it.isNotBlank() }?.let {
                    addProperty("url", it)
                }
                uploaded.optString("sha256", "").takeIf { it.isNotBlank() }?.let {
                    addProperty("sha256", it)
                }
                addProperty("size_bytes", uploaded.optLong("size_bytes", attachment.file.length()))
                uploaded.optIntOrNull("image_width", attachment.imageWidth)?.let {
                    addProperty("image_width", it)
                }
                uploaded.optIntOrNull("image_height", attachment.imageHeight)?.let {
                    addProperty("image_height", it)
                }
                attachment.durationSeconds?.let {
                    addProperty("duration_seconds", it)
                }
                attachment.transcription?.takeIf { it.isNotBlank() }?.let {
                    addProperty("transcription", it)
                }
            })
        }
    }
    return array
}

private fun attachmentUploadUrl(
    serverUrl: String,
    userId: String,
    target: SendTarget,
    attachment: PendingAttachment
): String {
    return buildString {
        if (target.projectId == CHAT_ATTACHMENT_TARGET_ID) {
            append("$serverUrl/api/user/${urlPart(userId)}/chat-attachments")
        } else {
            append("$serverUrl/api/user/${urlPart(userId)}/projects/${urlPart(target.projectId)}/attachments")
            append("?title=${urlPart(target.projectTitle)}")
        }
        append(if (contains("?")) "&" else "?")
        append("conversation_id=${urlPart(target.conversationId)}")
        append("&conversation_title=${urlPart(target.conversationTitle)}")
        append("&kind=${urlPart(attachment.kind)}")
        append("&display_name=${urlPart(attachment.displayName)}")
        append("&file_name=${urlPart(attachment.fileName)}")
        append("&mime_type=${urlPart(attachment.mimeType)}")
        attachment.imageWidth?.let { append("&image_width=$it") }
        attachment.imageHeight?.let { append("&image_height=$it") }
    }
}

private fun JSONObject.optIntOrNull(name: String, fallback: Int?): Int? {
    if (!has(name) || isNull(name)) return fallback
    return optInt(name).takeIf { it > 0 } ?: fallback
}
