package com.elon.app

import android.content.Context
import android.content.Intent
import android.os.Build
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import java.io.File
import java.net.URLEncoder
import java.util.Locale

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
    token: String,
    projectId: String? = null,
    projectName: String? = null,
    http: OkHttpClient = OkHttpClient()
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

    ApkChatInstaller.downloadAndInstall(
        activity = activity,
        url = projectApkUrlWithToken(apkUrl, token),
        http = http,
        projectId = projectId,
        projectName = projectName
    )
}

internal fun projectApkActionLabel(
    activity: AppCompatActivity,
    projectId: String,
    projectName: String,
    apkUrl: String?
): String {
    if (resolveInstalledProjectApp(activity, projectId, projectName) != null) return "打开应用"
    return if (apkUrl.isNullOrBlank()) "暂无APK" else "安装"
}

internal fun isProjectAppInstalled(
    activity: AppCompatActivity,
    projectId: String,
    projectName: String
): Boolean {
    return resolveInstalledProjectApp(activity, projectId, projectName) != null
}

internal fun openInstalledProjectApp(
    activity: AppCompatActivity,
    projectId: String,
    projectName: String
): Boolean {
    val target = resolveInstalledProjectApp(activity, projectId, projectName) ?: return false
    val launchIntent = activity.packageManager.getLaunchIntentForPackage(target.packageName) ?: return false
    return runCatching {
        activity.startActivity(launchIntent)
        true
    }.getOrElse {
        Toast.makeText(activity, "无法打开${target.label}", Toast.LENGTH_SHORT).show()
        false
    }
}

internal fun rememberProjectApkPackage(activity: AppCompatActivity, projectId: String?, packageName: String?) {
    val cleanProjectId = projectId?.trim().orEmpty()
    val cleanPackage = packageName?.trim().orEmpty()
    if (cleanProjectId.isBlank() || cleanPackage.isBlank()) return
    activity.getSharedPreferences(PROJECT_APK_PREFS, Context.MODE_PRIVATE)
        .edit()
        .putString(packageKey(cleanProjectId), cleanPackage)
        .apply()
}

internal fun projectApkPackageName(activity: AppCompatActivity, apkFile: File): String? {
    return runCatching {
        @Suppress("DEPRECATION")
        activity.packageManager.getPackageArchiveInfo(apkFile.absolutePath, 0)?.packageName
    }.getOrNull()?.trim()?.takeIf { it.isNotBlank() }
}

private data class InstalledProjectApp(
    val packageName: String,
    val label: String
)

private fun resolveInstalledProjectApp(
    activity: AppCompatActivity,
    projectId: String,
    projectName: String
): InstalledProjectApp? {
    resolveStoredPackage(activity, projectId)?.let { return it }
    return resolveInstalledAppByLabel(activity, projectName)
}

private fun resolveStoredPackage(activity: AppCompatActivity, projectId: String): InstalledProjectApp? {
    val prefs = activity.getSharedPreferences(PROJECT_APK_PREFS, Context.MODE_PRIVATE)
    val packageName = prefs.getString(packageKey(projectId), null)?.trim().orEmpty()
    if (packageName.isBlank()) return null
    val launchIntent = activity.packageManager.getLaunchIntentForPackage(packageName)
    if (launchIntent == null) {
        prefs.edit().remove(packageKey(projectId)).apply()
        return null
    }
    val label = runCatching {
        val appInfo = activity.packageManager.getApplicationInfo(packageName, 0)
        activity.packageManager.getApplicationLabel(appInfo).toString()
    }.getOrDefault(packageName)
    return InstalledProjectApp(packageName, label)
}

private fun resolveInstalledAppByLabel(activity: AppCompatActivity, projectName: String): InstalledProjectApp? {
    val normalizedProjectName = normalizeAppLabel(projectName)
    if (normalizedProjectName.length < 2) return null
    val launcherIntent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
    val matches = runCatching {
        @Suppress("DEPRECATION")
        activity.packageManager.queryIntentActivities(launcherIntent, 0)
    }.getOrDefault(emptyList())
    for (info in matches) {
        val label = info.loadLabel(activity.packageManager).toString().trim()
        val packageName = info.activityInfo.packageName?.trim().orEmpty()
        if (label.isBlank() || packageName.isBlank()) continue
        if (normalizeAppLabel(label) == normalizedProjectName &&
            activity.packageManager.getLaunchIntentForPackage(packageName) != null
        ) {
            return InstalledProjectApp(packageName, label)
        }
    }
    return null
}

private fun normalizeAppLabel(value: String): String {
    return value
        .trim()
        .lowercase(Locale.ROOT)
        .filter { it.isLetterOrDigit() }
}

private fun packageKey(projectId: String): String = "project_$projectId"

private const val PROJECT_APK_PREFS = "project_apk_packages"
