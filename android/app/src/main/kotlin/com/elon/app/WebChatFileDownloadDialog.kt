package com.elon.app

import android.view.View
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.WebChatFileDownloadState.Stage
import java.util.Locale

internal object WebChatFileDownloadPresentation {
    fun status(state: WebChatFileDownloadState, detail: String? = null): String = when (state.stage) {
        Stage.PREPARING -> "正在准备下载"
        Stage.TRANSFERRING -> "正在下载"
        Stage.SAVING -> "正在保存"
        Stage.CANCELLING -> "正在取消"
        Stage.CANCELLED -> "已取消下载"
        Stage.SAVED -> "已保存到下载目录"
        Stage.QUEUED -> "已交给系统下载"
        Stage.FAILED -> when (detail) {
            "download_confirmation_unknown" -> "尚未确认下载，请先查看下载目录"
            "download_file_unavailable" -> "此文件已失效或无法访问"
            "download_selection_expired" -> "附件列表已过期，请刷新后重试"
            "download_file_not_ready" -> "文件尚未就绪，请稍后重试"
            "download_file_too_large" -> "文件超过本次下载大小限制"
            "download_storage_failed" -> "无法保存文件，请检查存储空间后重试"
            "download_transfer_timeout" -> "文件下载超时，请检查网络后重试"
            "download_content_invalid" -> "未收到完整文件，请稍后重试"
            else -> "下载未完成"
        }
        Stage.UNCONFIRMED -> "尚未确认保存，请查看下载目录"
    }

    fun bytes(state: WebChatFileDownloadState): String {
        fun size(value: Long): String = when {
            value >= 1024 * 1024 -> String.format(Locale.ROOT, "%.1f MB", value / (1024.0 * 1024))
            value >= 1024 -> String.format(Locale.ROOT, "%.1f KB", value / 1024.0)
            else -> "$value B"
        }
        return if (state.totalBytes >= 0) "${size(state.receivedBytes)} / ${size(state.totalBytes)}"
            else size(state.receivedBytes)
    }
}

internal class WebChatFileDownloadDialog(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
) {
    private var dialog: AlertDialog? = null
    private var poll: Runnable? = null

    fun show(owner: WebChatConsumerPort, requestId: String) {
        dismiss()
        if (activity.isFinishing || activity.isDestroyed || consumerPort() !== owner) return
        val status = TextView(activity).apply {
            textSize = 16f
            contentDescription = "web-chat-file-download-status"
            accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
        }
        val progress = ProgressBar(activity, null, android.R.attr.progressBarStyleHorizontal).apply {
            max = 100
            contentDescription = "web-chat-file-download-progress"
        }
        val bytes = TextView(activity).apply { contentDescription = "web-chat-file-download-bytes" }
        val padding = (24 * activity.resources.displayMetrics.density).toInt()
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding / 2, padding, padding / 2)
            addView(status)
            addView(progress, LinearLayout.LayoutParams(-1, padding))
            addView(bytes)
        }
        val window = AlertDialog.Builder(activity).setTitle("文件下载").setView(body)
            .setPositiveButton("取消下载", null).setNegativeButton("收起", null).create()
        dialog = window
        window.setOnDismissListener {
            if (dialog === window) {
                poll?.let(host::removeCallbacks)
                poll = null
                dialog = null
            }
        }
        window.show()
        val cancel = window.getButton(AlertDialog.BUTTON_POSITIVE).apply {
            contentDescription = "web-chat-file-download-cancel"
            isEnabled = false
            setOnClickListener {
                if (consumerPort() !== owner) { dismiss(); return@setOnClickListener }
                if (!owner.cancelFileDownload(requestId).accepted) {
                    Toast.makeText(activity, "下载状态已变化，请查看当前结果", Toast.LENGTH_SHORT).show()
                }
                poll?.let { host.removeCallbacks(it); host.post(it) }
            }
        }
        val close = window.getButton(AlertDialog.BUTTON_NEGATIVE).apply {
            contentDescription = "web-chat-file-download-collapse"
        }
        val started = android.os.SystemClock.elapsedRealtime()
        var terminalChecks = 0
        val task = object : Runnable {
            override fun run() {
                if (dialog !== window) return
                if (consumerPort() !== owner || activity.isFinishing || activity.isDestroyed) { dismiss(); return }
                val current = owner.fileDownloadState()?.takeIf { it.requestId == requestId }
                val displayed = current ?: run {
                    val command = owner.state().commandRequests.firstOrNull { it.id == requestId }
                    val stage = when (command?.status) {
                        WebChatConsumerCommandStatus.SUCCEEDED -> if (command.detail == "download_saved") Stage.SAVED else Stage.QUEUED
                        WebChatConsumerCommandStatus.FAILED -> if (command.detail == "download_cancelled") Stage.CANCELLED else Stage.FAILED
                        WebChatConsumerCommandStatus.TIMED_OUT -> Stage.UNCONFIRMED
                        else -> if (android.os.SystemClock.elapsedRealtime() - started >= 25_000) Stage.UNCONFIRMED else Stage.PREPARING
                    }
                    WebChatFileDownloadState(requestId, stage)
                }
                val failure = if (displayed.stage == Stage.FAILED) owner.state().commandRequests.firstOrNull { it.id == requestId } else null
                status.text = WebChatFileDownloadPresentation.status(displayed, failure?.detail)
                bytes.text = WebChatFileDownloadPresentation.bytes(displayed)
                bytes.visibility = if (displayed.totalBytes >= 0 || displayed.receivedBytes > 0) View.VISIBLE else View.GONE
                progress.isIndeterminate = displayed.progressPercent == null && displayed.active
                progress.progress = displayed.progressPercent ?: 0
                progress.visibility = if (displayed.active) View.VISIBLE else View.GONE
                cancel.isEnabled = current?.canCancel == true
                cancel.visibility = if (displayed.active) View.VISIBLE else View.GONE
                close.text = if (displayed.active) "收起" else "关闭"
                val awaitFailureDetail = displayed.stage == Stage.FAILED &&
                    failure?.status !in setOf(WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT) && terminalChecks++ < 3
                if (displayed.active || awaitFailureDetail) host.postDelayed(this, 1_000) else poll = null
            }
        }
        poll = task
        host.post(task)
    }

    fun dismiss() {
        poll?.let(host::removeCallbacks)
        poll = null
        val current = dialog
        dialog = null
        current?.dismiss()
    }
}
