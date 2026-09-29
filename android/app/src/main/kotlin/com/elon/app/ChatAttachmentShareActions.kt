package com.elon.app

import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

internal object ChatAttachmentShareActions {
    fun start(activity: AppCompatActivity, message: ChatMessage, text: String, copy: Boolean) {
        if (message.isRecalled()) return
        val host = activity.window.decorView
        if ((host.getTag(R.id.chatAttachmentShareJob) as? Job)?.isActive == true) {
            Toast.makeText(activity, "正在准备附件", Toast.LENGTH_SHORT).show()
            return
        }
        val snapshot = message.copyForSharing()
        val identity = AuthManager.token(activity)
        Toast.makeText(activity, "正在准备附件", Toast.LENGTH_SHORT).show()
        val job = activity.lifecycleScope.launch {
            try {
                val files = withContext(Dispatchers.IO) { ChatAttachmentExport.prepare(activity, snapshot.attachments.orEmpty()) }
                if (activity.isFinishing || activity.isDestroyed || identity != AuthManager.token(activity)) return@launch
                if (message.isRecalled() || message.revision != snapshot.revision || message.attachments != snapshot.attachments) {
                    Toast.makeText(activity, "消息已变化，请重新选择", Toast.LENGTH_SHORT).show()
                    return@launch
                }
                if (copy) {
                    val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                    clipboard.setPrimaryClip(ChatAttachmentExport.clip(activity, files, text))
                    Toast.makeText(activity, "已复制附件", Toast.LENGTH_SHORT).show()
                } else activity.startActivity(Intent.createChooser(ChatAttachmentExport.intent(activity, files, text), "转发聊天附件"))
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                if (!activity.isFinishing && !activity.isDestroyed) {
                    Toast.makeText(activity, "附件准备失败，请检查网络后重试；每个附件最多 12 MB", Toast.LENGTH_LONG).show()
                }
            } finally { host.setTag(R.id.chatAttachmentShareJob, null) }
        }
        host.setTag(R.id.chatAttachmentShareJob, job)
    }
}
