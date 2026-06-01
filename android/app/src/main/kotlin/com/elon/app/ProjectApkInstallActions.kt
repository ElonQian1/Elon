package com.elon.app

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import java.net.URLEncoder

internal fun isAndroidApkInstallSupported(): Boolean = Build.VERSION.SDK_INT > 0

internal fun projectApkUrlWithToken(apkUrl: String, token: String): String {
    val trimmedUrl = apkUrl.trim()
    if (trimmedUrl.isBlank() || trimmedUrl.contains("token=")) return trimmedUrl
    val separator = if (trimmedUrl.contains("?")) "&" else "?"
    val encodedToken = URLEncoder.encode(token.trim(), Charsets.UTF_8.name())
    return "$trimmedUrl${separator}token=$encodedToken"
}

internal fun openProjectApkInstall(
    activity: AppCompatActivity,
    apkUrl: String,
    token: String
) {
    if (!isAndroidApkInstallSupported()) {
        Toast.makeText(activity, "当前设备不是 Android，无法直接安装 APK", Toast.LENGTH_SHORT).show()
        return
    }
    if (apkUrl.isBlank()) {
        Toast.makeText(activity, "这个项目还没有可安装 APK", Toast.LENGTH_SHORT).show()
        return
    }
    if (token.isBlank()) {
        Toast.makeText(activity, "请先登录后安装 APK", Toast.LENGTH_SHORT).show()
        return
    }

    val intent = Intent(Intent.ACTION_VIEW, Uri.parse(projectApkUrlWithToken(apkUrl, token)))
    runCatching { activity.startActivity(intent) }
        .onFailure { Toast.makeText(activity, "无法打开安装链接", Toast.LENGTH_SHORT).show() }
}
