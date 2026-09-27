package com.elon.app.chatrecords

import android.app.Activity
import android.content.Intent
import android.widget.ImageView
import android.widget.VideoView
import androidx.appcompat.app.AlertDialog
import androidx.core.content.FileProvider
import java.io.File

internal object ChatRecordMedia {
    fun open(activity: Activity, row: RecordRow, file: File, videoChanged: (VideoView?) -> Unit) {
        val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", file)
        when (row.kind) {
            "image" -> {
                val image = ImageView(activity).apply { adjustViewBounds = true; contentDescription = row.filename; scaleType = ImageView.ScaleType.FIT_CENTER }
                val dialog = AlertDialog.Builder(activity).setTitle(row.filename).setView(image).setPositiveButton("关闭", null).show()
                com.elon.app.ChatImagePreviewLoader.load(activity, file.path) { bitmap -> image.post { if (dialog.isShowing) image.setImageBitmap(bitmap) } }
            }
            "video" -> {
                val video = VideoView(activity); videoChanged(video)
                val frame = android.widget.FrameLayout(activity).apply { minimumHeight = (320 * resources.displayMetrics.density).toInt(); addView(video, android.widget.FrameLayout.LayoutParams(-1, -1)) }
                val dialog = AlertDialog.Builder(activity).setTitle(row.filename).setView(frame).setPositiveButton("关闭", null).create()
                dialog.setOnDismissListener { video.stopPlayback(); videoChanged(null) }; dialog.show()
                video.setMediaController(android.widget.MediaController(activity).apply { setAnchorView(video) })
                video.setVideoURI(uri); video.setOnPreparedListener { video.start() }
                video.setOnErrorListener { _, _, _ -> android.widget.Toast.makeText(activity, "视频格式暂不支持播放", android.widget.Toast.LENGTH_LONG).show(); true }
            }
            else -> {
                val ext = row.filename.substringAfterLast('.', "").lowercase()
                val mime = android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext) ?: "application/octet-stream"
                runCatching { activity.startActivity(Intent.createChooser(Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION), "打开附件")) }
                    .onFailure { android.widget.Toast.makeText(activity, "没有可打开此附件的应用", android.widget.Toast.LENGTH_LONG).show() }
            }
        }
    }
}
