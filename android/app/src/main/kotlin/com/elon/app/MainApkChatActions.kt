package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient

internal fun handleApkChatAction(activity: AppCompatActivity, http: OkHttpClient, action: String, url: String) {
    when (action) {
        "install" -> ApkChatInstaller.downloadAndInstall(activity, url, http)
        "copy" -> {
            val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("apk_url", url))
            Toast.makeText(activity, "链接已复制", Toast.LENGTH_SHORT).show()
        }
        "share" -> activity.startActivity(Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, url)
        })
    }
}
