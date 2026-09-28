package com.elon.app.chatrecords

import android.app.Activity
import android.os.Bundle
import androidx.appcompat.app.AppCompatDelegate
import com.elon.app.PendingAttachment
import com.elon.app.sharing.ShareDraft
import com.elon.app.sharing.ShareDraftStore
import java.io.File
import java.util.UUID

/** Offline acceptance launcher, only usable in the separately installed recordtest package. */
class ChatRecordPreviewActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (!packageName.endsWith(".recordtest")) { finish(); return }
        val mode = if (intent.getStringExtra("theme") == "dark") AppCompatDelegate.MODE_NIGHT_YES else AppCompatDelegate.MODE_NIGHT_NO
        getSharedPreferences("mobile_appearance", MODE_PRIVATE).edit().putInt("mode", mode).apply()
        AppCompatDelegate.setDefaultNightMode(mode)
        val id = UUID.randomUUID().toString()
        val dir = File(cacheDir, "external_shares/$id").apply { mkdirs() }
        val source = File(getExternalFilesDir(null), "record-preview.webm")
        val files = if (source.isFile) {
            val local = source.copyTo(File(dir, "record_preview-video"), overwrite = true)
            listOf(PendingAttachment("file", "验收附件", "视频示例.webm", "视频示例.webm", "video/webm", local))
        } else emptyList()
        val video = RecordRow("video", null, "示例发送者", "2026年09月29日 00:00", "video", "[视频] 视频示例.webm", "视频示例.webm", if (files.isEmpty()) null else "preview-video")
        val messages = listOf(
            RecordRow("text", null, "示例发送者", "2026年09月29日 00:00", "text", "这是一份离线验收记录。原文保留，视频点击后播放。"),
            video,
            RecordRow("missing", null, "另一位发送者", "2026年09月29日 00:01", "image", "[图片] 缺失图片.jpg", "缺失图片.jpg"),
            RecordRow("forward", null, "另一位发送者", "2026年09月29日 00:02", "forward", "[聊天记录]"),
            RecordRow("nested", "forward", "示例发送者", "2026年09月29日 00:02", "text", "嵌套记录保留原来的返回位置。")
        )
        ShareDraftStore(this).save(ShareDraft(id, "", files, record = ChatRecordDocument("聊天记录 · 离线验收", messages.joinToString("\n") { it.text }, messages)))
        ChatRecordReaderActivity.preview(this, id); finish()
    }
}
