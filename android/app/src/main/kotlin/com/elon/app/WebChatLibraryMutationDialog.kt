package com.elon.app

import android.os.SystemClock
import android.text.InputFilter
import android.view.View
import android.widget.EditText
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebLibraryCommands

internal class WebChatLibraryMutationDialog(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val onSettled: () -> Unit,
) {
    private var dialog: AlertDialog? = null
    private var poll: Runnable? = null
    private var busy = false

    fun show(owner: WebChatConsumerPort, file: WebChatLibraryEntry, operation: String) {
        if (busy || consumerPort() !== owner || activity.isFinishing || activity.isDestroyed) return
        dismiss()
        val rename = operation == "rename"
        if (if (rename) !file.canRename else operation != "trash" || !file.canTrash) return
        val name = EditText(activity).apply {
            setSingleLine(true)
            filters = arrayOf(InputFilter.LengthFilter(180))
            setText(file.name)
            selectAll()
            contentDescription = "web-chat-library-rename-input"
        }
        val builder = AlertDialog.Builder(activity).setTitle(if (rename) "重命名文件" else "移到最近删除")
            .setNegativeButton("取消", null).setPositiveButton(if (rename) "保存" else "移到最近删除", null)
        if (rename) builder.setView(name) else builder.setMessage("将“${file.name}”移到最近删除？")
        val window = builder.create()
        dialog = window
        window.show()
        window.getButton(AlertDialog.BUTTON_POSITIVE).apply {
            contentDescription = "web-chat-library-mutation-confirm"
            setOnClickListener {
                if (consumerPort() !== owner) { dismiss(); return@setOnClickListener }
                val value = if (rename) name.text.toString().trim() else ""
                if (rename && !ChatGptWebLibraryCommands.validName(value)) {
                    name.error = "请输入有效文件名"
                    return@setOnClickListener
                }
                val result = owner.mutateLibraryFile(file.handle, operation, value, true)
                if (!result.accepted || result.requestId == null) {
                    Toast.makeText(activity, "文件状态已变化，请刷新后重试", Toast.LENGTH_SHORT).show()
                    return@setOnClickListener
                }
                window.dismiss()
                track(owner, result.requestId, rename)
            }
        }
    }

    private fun track(owner: WebChatConsumerPort, requestId: String, rename: Boolean) {
        val status = TextView(activity).apply {
            text = if (rename) "正在保存文件名" else "正在移到最近删除"
            textSize = 16f
            val padding = (24 * resources.displayMetrics.density).toInt()
            setPadding(padding, padding, padding, padding)
            contentDescription = "web-chat-library-mutation-status"
            accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
        }
        val window = AlertDialog.Builder(activity).setTitle(if (rename) "重命名文件" else "移到最近删除")
            .setView(status).setNegativeButton("收起", null).create()
        dialog = window
        busy = true
        window.show()
        val started = SystemClock.elapsedRealtime()
        val task = object : Runnable {
            override fun run() {
                if (poll !== this) return
                if (consumerPort() !== owner || activity.isFinishing || activity.isDestroyed) { dismiss(); return }
                val command = owner.state().commandRequests.firstOrNull { it.id == requestId }
                val success = command?.status == WebChatConsumerCommandStatus.SUCCEEDED && command.detail == "library_mutation_acknowledged"
                val terminal = command?.status in setOf(WebChatConsumerCommandStatus.SUCCEEDED,
                    WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT)
                if (!terminal && SystemClock.elapsedRealtime() - started < 30_000) {
                    host.postDelayed(this, 500)
                    return
                }
                poll = null
                busy = false
                val message = if (success) {
                    if (rename) "文件名已更新" else "已移到最近删除"
                } else when (command?.detail) {
                    "library_selection_expired" -> "文件列表已过期，请刷新后重试"
                    "library_mutation_http_401", "library_mutation_http_403" -> "当前账号无法执行此操作"
                    else -> "尚未确认操作结果，请刷新文件库查看"
                }
                status.text = message
                window.getButton(AlertDialog.BUTTON_NEGATIVE)?.text = "关闭"
                if (!window.isShowing) Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
                onSettled()
            }
        }
        poll = task
        host.post(task)
    }

    fun dismiss() {
        poll?.let(host::removeCallbacks)
        poll = null
        busy = false
        dialog?.dismiss()
        dialog = null
    }
}
