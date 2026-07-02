package com.elon.app

import android.content.Context
import android.content.Intent
import android.content.pm.PackageInfo
import android.os.Build
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import java.io.File
import java.net.URLEncoder
import java.time.Instant
import java.util.Locale

internal fun isAndroidApkInstallSupported(): Boolean = Build.VERSION.SDK_INT > 0

internal fun cleanProjectApkUrl(apkUrl: String?): String? {
    val trimmedUrl = apkUrl?.trim().orEmpty()
    if (trimmedUrl.isBlank() || trimmedUrl.equals("null", ignoreCase = true)) return null
    if (!trimmedUrl.startsWith("http://", ignoreCase = true) &&
        !trimmedUrl.startsWith("https://", ignoreCase = true)
    ) {
        return null
    }
    return trimmedUrl
}

private fun cleanProjectApkMarker(value: String?): String? {
    return value?.trim()?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

private fun latestProjectApkMarker(apkIdentity: String?, apkUpdatedAt: String?): String? {
    return cleanProjectApkMarker(apkUpdatedAt)?.let { "updated:$it" }
        ?: cleanProjectApkMarker(apkIdentity)
}

private fun parseProjectApkUpdatedAtMillis(value: String?): Long? {
    val clean = cleanProjectApkMarker(value) ?: return null
    clean.toLongOrNull()?.let { epoch ->
        return if (epoch > 10_000_000_000L) epoch else epoch * 1000L
    }
    return runCatching { Instant.parse(clean).toEpochMilli() }.getOrNull()
}

internal fun projectApkUrlWithToken(apkUrl: String, token: String): String {
    val trimmedUrl = cleanProjectApkUrl(apkUrl) ?: return ""
    if (trimmedUrl.contains("token=")) return trimmedUrl
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
    apkIdentity: String? = null,
    apkUpdatedAt: String? = null,
    http: OkHttpClient = OkHttpClient()
) {
    if (!isAndroidApkInstallSupported()) {
        Toast.makeText(activity, "当前设备不是 Android，无法直接安装 APK", Toast.LENGTH_SHORT).show()
        return
    }
    val cleanUrl = cleanProjectApkUrl(apkUrl)
    if (cleanUrl == null) {
        Toast.makeText(activity, "这个项目还没有可安装 APK", Toast.LENGTH_SHORT).show()
        return
    }
    if (token.isBlank()) {
        Toast.makeText(activity, "请先登录后安装 APK", Toast.LENGTH_SHORT).show()
        return
    }

    ApkChatInstaller.downloadAndInstall(
        activity = activity,
        url = projectApkUrlWithToken(cleanUrl, token),
        http = http,
        projectId = projectId,
        projectName = projectName,
        apkIdentity = apkIdentity,
        apkUpdatedAt = apkUpdatedAt
    )
}

internal fun projectApkActionLabel(
    activity: AppCompatActivity,
    projectId: String,
    projectName: String,
    apkUrl: String?,
    apkIdentity: String? = null,
    apkUpdatedAt: String? = null
): String {
    val cleanUrl = cleanProjectApkUrl(apkUrl)
    val installed = resolveInstalledProjectApp(activity, projectId, projectName)
    if (installed != null) {
        return if (cleanUrl != null &&
            isProjectApkUpdateAvailable(activity, projectId, installed, apkIdentity, apkUpdatedAt)
        ) {
            "更新"
        } else {
            "打开应用"
        }
    }
    return if (cleanUrl == null) "暂无APK" else "安装"
}

internal fun hasProjectApkUpdate(
    activity: AppCompatActivity,
    projectId: String,
    projectName: String,
    apkIdentity: String?,
    apkUpdatedAt: String?
): Boolean {
    val installed = resolveInstalledProjectApp(activity, projectId, projectName) ?: return false
    return isProjectApkUpdateAvailable(activity, projectId, installed, apkIdentity, apkUpdatedAt)
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
    val prefs = activity.getSharedPreferences(PROJECT_APK_PREFS, Context.MODE_PRIVATE)
    val edit = prefs.edit()
        .putString(packageKey(cleanProjectId), cleanPackage)
    resolveInstalledPackage(activity, cleanPackage)?.let { installed ->
        edit
            .putLong(installedVersionCodeKey(cleanProjectId), installed.versionCode)
            .putLong(installedUpdatedAtKey(cleanProjectId), installed.lastUpdateTime)
    }
    edit.apply()
}

internal fun rememberPendingProjectApkInstall(
    activity: AppCompatActivity,
    projectId: String?,
    apkIdentity: String?,
    apkUpdatedAt: String?,
    targetVersionCode: Long?
) {
    val cleanProjectId = projectId?.trim().orEmpty()
    val marker = latestProjectApkMarker(apkIdentity, apkUpdatedAt) ?: return
    if (cleanProjectId.isBlank()) return
    val edit = activity.getSharedPreferences(PROJECT_APK_PREFS, Context.MODE_PRIVATE)
        .edit()
        .putString(pendingMarkerKey(cleanProjectId), marker)
    targetVersionCode?.takeIf { it > 0L }?.let {
        edit.putLong(pendingVersionCodeKey(cleanProjectId), it)
    }
    edit.apply()
}

internal fun projectApkPackageName(activity: AppCompatActivity, apkFile: File): String? {
    return runCatching {
        @Suppress("DEPRECATION")
        activity.packageManager.getPackageArchiveInfo(apkFile.absolutePath, 0)?.packageName
    }.getOrNull()?.trim()?.takeIf { it.isNotBlank() }
}

internal fun projectApkVersionCode(activity: AppCompatActivity, apkFile: File): Long? {
    return runCatching {
        @Suppress("DEPRECATION")
        activity.packageManager.getPackageArchiveInfo(apkFile.absolutePath, 0)
            ?.let(::packageInfoVersionCode)
    }.getOrNull()
}

private data class InstalledProjectApp(
    val packageName: String,
    val label: String,
    val versionCode: Long,
    val lastUpdateTime: Long
)

private fun isProjectApkUpdateAvailable(
    activity: AppCompatActivity,
    projectId: String,
    installed: InstalledProjectApp,
    apkIdentity: String?,
    apkUpdatedAt: String?
): Boolean {
    promotePendingProjectApkInstall(activity, projectId, installed)
    val latestMarker = latestProjectApkMarker(apkIdentity, apkUpdatedAt)
    if (latestMarker != null) {
        val installedMarker = installedProjectApkMarker(activity, projectId)
        if (installedMarker != null) return installedMarker != latestMarker
    }
    val latestUpdatedAt = parseProjectApkUpdatedAtMillis(apkUpdatedAt)
    return latestUpdatedAt != null &&
        installed.lastUpdateTime > 0L &&
        latestUpdatedAt > installed.lastUpdateTime
}

private fun resolveInstalledProjectApp(
    activity: AppCompatActivity,
    projectId: String,
    projectName: String
): InstalledProjectApp? {
    if (projectId.isNotBlank()) {
        resolveStoredPackage(activity, projectId)?.let { return it }
    }
    return resolveInstalledAppByLabel(activity, projectName)
}

private fun resolveStoredPackage(activity: AppCompatActivity, projectId: String): InstalledProjectApp? {
    val prefs = activity.getSharedPreferences(PROJECT_APK_PREFS, Context.MODE_PRIVATE)
    val packageName = prefs.getString(packageKey(projectId), null)?.trim().orEmpty()
    if (packageName.isBlank()) return null
    val installed = resolveInstalledPackage(activity, packageName)
    if (installed == null) {
        clearProjectApkPackage(prefs, projectId)
        return null
    }
    return installed
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
            return resolveInstalledPackage(activity, packageName)
        }
    }
    return null
}

private fun resolveInstalledPackage(activity: AppCompatActivity, packageName: String): InstalledProjectApp? {
    if (activity.packageManager.getLaunchIntentForPackage(packageName) == null) return null
    val packageInfo = runCatching {
        @Suppress("DEPRECATION")
        activity.packageManager.getPackageInfo(packageName, 0)
    }.getOrNull() ?: return null
    val label = runCatching {
        val appInfo = activity.packageManager.getApplicationInfo(packageName, 0)
        activity.packageManager.getApplicationLabel(appInfo).toString()
    }.getOrDefault(packageName)
    return InstalledProjectApp(
        packageName = packageName,
        label = label,
        versionCode = packageInfoVersionCode(packageInfo),
        lastUpdateTime = packageInfo.lastUpdateTime
    )
}

private fun promotePendingProjectApkInstall(
    activity: AppCompatActivity,
    projectId: String,
    installed: InstalledProjectApp
) {
    if (projectId.isBlank()) return
    val prefs = activity.getSharedPreferences(PROJECT_APK_PREFS, Context.MODE_PRIVATE)
    val pendingMarker = cleanProjectApkMarker(prefs.getString(pendingMarkerKey(projectId), null))
        ?: return
    val pendingVersionCode = prefs.getLong(pendingVersionCodeKey(projectId), -1L)
    val recordedVersionCode = prefs.getLong(installedVersionCodeKey(projectId), -1L)
    val targetReached = pendingVersionCode <= 0L || installed.versionCode >= pendingVersionCode
    val versionChanged = recordedVersionCode <= 0L || installed.versionCode > recordedVersionCode
    if (!targetReached || !versionChanged) return
    prefs.edit()
        .putString(installedMarkerKey(projectId), pendingMarker)
        .putLong(installedVersionCodeKey(projectId), installed.versionCode)
        .putLong(installedUpdatedAtKey(projectId), installed.lastUpdateTime)
        .remove(pendingMarkerKey(projectId))
        .remove(pendingVersionCodeKey(projectId))
        .apply()
}

private fun installedProjectApkMarker(activity: AppCompatActivity, projectId: String): String? {
    if (projectId.isBlank()) return null
    return cleanProjectApkMarker(
        activity.getSharedPreferences(PROJECT_APK_PREFS, Context.MODE_PRIVATE)
            .getString(installedMarkerKey(projectId), null)
    )
}

private fun clearProjectApkPackage(
    prefs: android.content.SharedPreferences,
    projectId: String
) {
    prefs.edit()
        .remove(packageKey(projectId))
        .remove(installedMarkerKey(projectId))
        .remove(installedVersionCodeKey(projectId))
        .remove(installedUpdatedAtKey(projectId))
        .remove(pendingMarkerKey(projectId))
        .remove(pendingVersionCodeKey(projectId))
        .apply()
}

private fun packageInfoVersionCode(info: PackageInfo): Long {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        info.longVersionCode
    } else {
        @Suppress("DEPRECATION")
        info.versionCode.toLong()
    }
}

private fun normalizeAppLabel(value: String): String {
    return value
        .trim()
        .lowercase(Locale.ROOT)
        .filter { it.isLetterOrDigit() }
}

private fun packageKey(projectId: String): String = "project_$projectId"
private fun installedMarkerKey(projectId: String): String = "project_${projectId}_apk_marker"
private fun installedVersionCodeKey(projectId: String): String = "project_${projectId}_version_code"
private fun installedUpdatedAtKey(projectId: String): String = "project_${projectId}_updated_at"
private fun pendingMarkerKey(projectId: String): String = "project_${projectId}_pending_apk_marker"
private fun pendingVersionCodeKey(projectId: String): String = "project_${projectId}_pending_version_code"

private const val PROJECT_APK_PREFS = "project_apk_packages"
