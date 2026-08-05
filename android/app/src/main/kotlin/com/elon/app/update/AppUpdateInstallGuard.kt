package com.elon.app.update

import android.content.Context
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.os.Build
import java.io.File

internal data class AppUpdatePruneDecision(
    val clearLatestVersion: Boolean,
    val clearSnapshot: Boolean,
) {
    val changed: Boolean
        get() = clearLatestVersion || clearSnapshot
}

internal fun appUpdatePruneDecision(
    installedVersionCode: Int,
    latestVersionCode: Int?,
    snapshotVersionCode: Int?,
): AppUpdatePruneDecision = AppUpdatePruneDecision(
    clearLatestVersion = latestVersionCode != null && latestVersionCode <= installedVersionCode,
    clearSnapshot = snapshotVersionCode != null && (
        snapshotVersionCode <= installedVersionCode ||
            latestVersionCode == null ||
            snapshotVersionCode != latestVersionCode
        ),
)

internal data class AppUpdateArchiveIdentity(
    val packageName: String,
    val versionCode: Long,
)

internal enum class AppUpdateInstallBlockReason {
    UNREADABLE,
    WRONG_PACKAGE,
    NOT_NEWER,
    VERSION_MISMATCH,
}

internal data class AppUpdateInstallDecision(
    val allowed: Boolean,
    val blockReason: AppUpdateInstallBlockReason? = null,
    val message: String = "",
)

internal fun validateAppUpdateArchive(
    expectedPackageName: String,
    installedVersionCode: Long,
    expectedVersionCode: Long,
    archive: AppUpdateArchiveIdentity?,
): AppUpdateInstallDecision = when {
    archive == null -> AppUpdateInstallDecision(
        allowed = false,
        blockReason = AppUpdateInstallBlockReason.UNREADABLE,
        message = "安装包无法读取，已清理旧文件，请重新下载",
    )
    archive.packageName != expectedPackageName -> AppUpdateInstallDecision(
        allowed = false,
        blockReason = AppUpdateInstallBlockReason.WRONG_PACKAGE,
        message = "安装包身份不匹配，已阻止安装，请重新下载",
    )
    archive.versionCode <= installedVersionCode -> AppUpdateInstallDecision(
        allowed = false,
        blockReason = AppUpdateInstallBlockReason.NOT_NEWER,
        message = "安装包 build ${archive.versionCode} 已过期，当前已是 build $installedVersionCode",
    )
    archive.versionCode != expectedVersionCode -> AppUpdateInstallDecision(
        allowed = false,
        blockReason = AppUpdateInstallBlockReason.VERSION_MISMATCH,
        message = "安装包版本与更新任务不一致，已清理旧文件，请重新下载",
    )
    else -> AppUpdateInstallDecision(allowed = true)
}

internal fun readAppUpdateArchiveIdentity(context: Context, file: File): AppUpdateArchiveIdentity? {
    if (!file.isFile) return null
    val packageInfo = readArchivePackageInfo(context.packageManager, file) ?: return null
    return AppUpdateArchiveIdentity(
        packageName = packageInfo.packageName.orEmpty(),
        versionCode = packageInfoVersionCode(packageInfo),
    ).takeIf { it.packageName.isNotBlank() && it.versionCode > 0L }
}

@Suppress("DEPRECATION")
private fun readArchivePackageInfo(packageManager: PackageManager, file: File): PackageInfo? =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        packageManager.getPackageArchiveInfo(file.absolutePath, PackageManager.PackageInfoFlags.of(0L))
    } else {
        packageManager.getPackageArchiveInfo(file.absolutePath, 0)
    }

@Suppress("DEPRECATION")
private fun packageInfoVersionCode(packageInfo: PackageInfo): Long =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        packageInfo.longVersionCode
    } else {
        packageInfo.versionCode.toLong()
    }
