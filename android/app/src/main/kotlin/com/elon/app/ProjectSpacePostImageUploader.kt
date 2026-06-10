package com.elon.app

import android.net.Uri
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

private const val PROJECT_SPACE_POST_ATTACHMENT_PREFIX = "project_space_posts"

internal class ProjectSpacePostImageUploader(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: () -> String,
    private val launchPicker: () -> Unit
) {
    private var pendingRequest: PendingRequest? = null

    fun pickLocalImage(project: ProjectSpaceSummary, onComplete: (Result<String>) -> Unit) {
        if (pendingRequest != null) {
            onComplete(Result.failure(IllegalStateException("已有图片选择正在进行")))
            return
        }
        pendingRequest = PendingRequest(project, onComplete)
        runCatching { launchPicker() }
            .onFailure {
                pendingRequest = null
                onComplete(Result.failure(it))
            }
    }

    fun handlePickedImage(uri: Uri?) {
        val request = pendingRequest ?: return
        pendingRequest = null
        if (uri == null) {
            request.onComplete(Result.failure(IllegalStateException("已取消选择图片")))
            return
        }
        showToast("正在上传图片...", long = false)
        thread(name = "project-space-post-image-upload") {
            val result = runCatching { uploadPickedImage(request.project, uri) }
            activity.runOnUiThread {
                request.onComplete(result)
                result.onSuccess { showToast("图片已上传", long = false) }
            }
        }
    }

    private fun uploadPickedImage(project: ProjectSpaceSummary, uri: Uri): String {
        val displayName = displayNameForUri(activity, uri)
            ?: "post_image_${System.currentTimeMillis()}.jpg"
        val attachment = copyAttachmentToCache(
            context = activity,
            displayLabel = "帖子图片",
            uri = uri,
            displayName = displayName,
            attachmentIndex = 1
        )
        require(attachment.kind == "image" || attachment.mimeType.startsWith("image/")) {
            "请选择图片文件"
        }
        val refs = uploadAttachmentRefsOrNull(
            http = http,
            serverUrl = serverUrl().trimEnd('/'),
            userId = AuthManager.effectiveUserId(activity),
            attachments = listOf(attachment),
            target = SendTarget(
                projectId = CHAT_ATTACHMENT_TARGET_ID,
                projectTitle = project.name.ifBlank { "项目空间" },
                conversationId = "${PROJECT_SPACE_POST_ATTACHMENT_PREFIX}_${project.id}",
                conversationTitle = project.name.ifBlank { "项目空间帖子" }
            ),
            maxAttachmentBytes = MAX_ATTACHMENT_BYTES,
            showShortToast = { showToast(it, long = false) },
            showLongToast = { showToast(it, long = true) }
        ) ?: throw IllegalStateException("图片上传失败")
        val uploaded = refs.firstOrNull()?.asJsonObject
            ?: throw IllegalStateException("图片上传响应异常")
        return uploaded.get("url")?.asString?.trim()?.takeIf { it.isNotBlank() }
            ?: throw IllegalStateException("图片上传响应缺少 URL")
    }

    private fun showToast(message: String, long: Boolean) {
        activity.runOnUiThread {
            Toast.makeText(
                activity,
                message,
                if (long) Toast.LENGTH_LONG else Toast.LENGTH_SHORT
            ).show()
        }
    }

    private data class PendingRequest(
        val project: ProjectSpaceSummary,
        val onComplete: (Result<String>) -> Unit
    )
}
