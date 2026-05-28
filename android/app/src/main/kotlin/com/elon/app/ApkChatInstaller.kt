package com.elon.app

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File

/**
 * 下载并安装 AI 项目生成的 APK。
 * 复用 AppUpdateManager 的下载+安装逻辑，但只需要一个 URL。
 */
internal object ApkChatInstaller {

    fun downloadAndInstall(activity: AppCompatActivity, url: String, http: OkHttpClient) {
        val progressBar = ProgressBar(activity, null, android.R.attr.progressBarStyleHorizontal)
            .apply { max = 100 }
        val progressText = TextView(activity).apply {
            text = "正在连接..."
            setPadding(0, 8, 0, 0)
        }
        val layout = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(64, 24, 64, 8)
            addView(progressBar, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
            addView(progressText)
        }
        val progressDialog = AlertDialog.Builder(activity)
            .setTitle("下载 APK")
            .setView(layout)
            .setCancelable(false)
            .show()

        Thread {
            try {
                activity.runOnUiThread { progressText.text = "正在从服务器下载..." }
                val request = Request.Builder().url(url).build()
                http.newCall(request).execute().use { resp ->
                    if (!resp.isSuccessful) error("HTTP ${resp.code}")
                    val body = resp.body ?: error("空响应体")
                    val totalBytes = body.contentLength()
                    val apkFile = File(activity.getExternalFilesDir(null), "elon_project.apk")

                    var downloaded = 0L
                    body.byteStream().use { input ->
                        apkFile.outputStream().use { output ->
                            val buf = ByteArray(8192)
                            var n: Int
                            while (input.read(buf).also { n = it } != -1) {
                                output.write(buf, 0, n)
                                downloaded += n
                                if (totalBytes > 0) {
                                    val pct = (downloaded * 100 / totalBytes).toInt()
                                    activity.runOnUiThread {
                                        progressBar.progress = pct
                                        progressText.text =
                                            "$pct%  (${"%.1f".format(downloaded / 1_048_576.0)} MB)"
                                    }
                                }
                            }
                        }
                    }
                    activity.runOnUiThread {
                        progressDialog.dismiss()
                        installApk(activity, apkFile)
                    }
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    progressDialog.dismiss()
                    AlertDialog.Builder(activity)
                        .setTitle("下载失败")
                        .setMessage(e.message ?: "未知错误")
                        .setPositiveButton("确定", null)
                        .show()
                }
            }
        }.start()
    }

    private fun installApk(activity: AppCompatActivity, apkFile: File) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
            !activity.packageManager.canRequestPackageInstalls()
        ) {
            AlertDialog.Builder(activity)
                .setTitle("需要安装权限")
                .setMessage("安装 APK 需要允许「安装未知来源应用」，请在设置中开启后重试。")
                .setPositiveButton("前往设置") { _, _ ->
                    activity.startActivity(
                        Intent(
                            Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                            Uri.parse("package:${activity.packageName}")
                        )
                    )
                }
                .setNegativeButton("取消", null)
                .show()
            return
        }

        val uri = FileProvider.getUriForFile(
            activity,
            "${activity.packageName}.update_provider",
            apkFile
        )
        activity.startActivity(
            Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION
            }
        )
    }
}
