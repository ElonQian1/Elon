package com.elon.app

import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.os.Build
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import java.security.MessageDigest

internal enum class ProjectApkSignatureCompatibility {
    FRESH_INSTALL,
    COMPATIBLE_UPDATE,
    SIGNATURE_CONFLICT,
    UNVERIFIABLE,
}

internal data class ProjectApkSignatureInspection(
    val compatibility: ProjectApkSignatureCompatibility,
    val packageName: String?,
    val versionCode: Long?,
) {
    val allowed: Boolean
        get() = compatibility == ProjectApkSignatureCompatibility.FRESH_INSTALL ||
            compatibility == ProjectApkSignatureCompatibility.COMPATIBLE_UPDATE
}

internal data class ProjectApkSignatureDecision(
    val compatibility: ProjectApkSignatureCompatibility,
    val title: String,
    val message: String,
) {
    val allowed: Boolean
        get() = compatibility == ProjectApkSignatureCompatibility.FRESH_INSTALL ||
            compatibility == ProjectApkSignatureCompatibility.COMPATIBLE_UPDATE
}

internal fun evaluateProjectApkSignatureCompatibility(
    archivePackageName: String?,
    archiveSignerSha256: Set<String>,
    installedPackageName: String?,
    installedSignerSha256: Set<String>,
): ProjectApkSignatureCompatibility {
    if (archivePackageName.isNullOrBlank() || archiveSignerSha256.isEmpty()) {
        return ProjectApkSignatureCompatibility.UNVERIFIABLE
    }
    if (installedPackageName.isNullOrBlank()) {
        return ProjectApkSignatureCompatibility.FRESH_INSTALL
    }
    if (archivePackageName != installedPackageName || installedSignerSha256.isEmpty()) {
        return ProjectApkSignatureCompatibility.UNVERIFIABLE
    }
    return if (archiveSignerSha256.any(installedSignerSha256::contains)) {
        ProjectApkSignatureCompatibility.COMPATIBLE_UPDATE
    } else {
        ProjectApkSignatureCompatibility.SIGNATURE_CONFLICT
    }
}

internal fun projectApkSignatureDecision(
    compatibility: ProjectApkSignatureCompatibility,
): ProjectApkSignatureDecision = when (compatibility) {
    ProjectApkSignatureCompatibility.FRESH_INSTALL -> ProjectApkSignatureDecision(
        compatibility,
        title = "可以安装",
        message = "安装包签名已验证。",
    )
    ProjectApkSignatureCompatibility.COMPATIBLE_UPDATE -> ProjectApkSignatureDecision(
        compatibility,
        title = "可以更新",
        message = "安装包与手机旧版使用同一签名。",
    )
    ProjectApkSignatureCompatibility.SIGNATURE_CONFLICT -> ProjectApkSignatureDecision(
        compatibility,
        title = "签名不一致，无法更新",
        message = "手机上已有同包名应用，但它与新安装包不是同一发布证书。Android 会拒绝覆盖安装。" +
            "请先确认旧版数据已经同步或备份，再到系统设置卸载旧版，然后重新安装。卸载会清除旧版的本地数据。",
    )
    ProjectApkSignatureCompatibility.UNVERIFIABLE -> ProjectApkSignatureDecision(
        compatibility,
        title = "无法验证安装包",
        message = "无法读取安装包或已安装应用的签名信息。为避免安装错误版本，本次安装已停止。",
    )
}

internal fun inspectProjectApkSignature(
    activity: AppCompatActivity,
    apkFile: File,
): ProjectApkSignatureInspection {
    val archive = readArchivePackageInfo(activity.packageManager, apkFile)
        ?: return ProjectApkSignatureInspection(
            compatibility = ProjectApkSignatureCompatibility.UNVERIFIABLE,
            packageName = null,
            versionCode = null,
        )
    val packageName = archive.packageName?.trim()?.takeIf(String::isNotBlank)
    val archiveSigners = packageSignerSha256(archive)
    val installed = packageName?.let { readInstalledPackageInfo(activity.packageManager, it) }
    val compatibility = evaluateProjectApkSignatureCompatibility(
        archivePackageName = packageName,
        archiveSignerSha256 = archiveSigners,
        installedPackageName = installed?.packageName,
        installedSignerSha256 = installed?.let(::packageSignerSha256).orEmpty(),
    )
    return ProjectApkSignatureInspection(
        compatibility = compatibility,
        packageName = packageName,
        versionCode = archive.projectApkVersionCode(),
    )
}

@Suppress("DEPRECATION")
private fun readArchivePackageInfo(packageManager: PackageManager, apkFile: File): PackageInfo? {
    val flags = projectApkSigningFlags()
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        packageManager.getPackageArchiveInfo(
            apkFile.absolutePath,
            PackageManager.PackageInfoFlags.of(flags.toLong()),
        )
    } else {
        packageManager.getPackageArchiveInfo(apkFile.absolutePath, flags)
    }
}

@Suppress("DEPRECATION")
private fun readInstalledPackageInfo(packageManager: PackageManager, packageName: String): PackageInfo? {
    val flags = projectApkSigningFlags()
    return runCatching {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            packageManager.getPackageInfo(
                packageName,
                PackageManager.PackageInfoFlags.of(flags.toLong()),
            )
        } else {
            packageManager.getPackageInfo(packageName, flags)
        }
    }.getOrNull()
}

@Suppress("DEPRECATION")
private fun projectApkSigningFlags(): Int = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
    PackageManager.GET_SIGNING_CERTIFICATES
} else {
    PackageManager.GET_SIGNATURES
}

@Suppress("DEPRECATION")
private fun packageSignerSha256(info: PackageInfo): Set<String> {
    val signatures = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        val signingInfo = info.signingInfo ?: return emptySet()
        if (signingInfo.hasMultipleSigners()) {
            signingInfo.apkContentsSigners
        } else {
            signingInfo.signingCertificateHistory
        }
    } else {
        info.signatures
    }
    return signatures.orEmpty()
        .map { signature -> sha256Hex(signature.toByteArray()) }
        .toSet()
}

private fun PackageInfo.projectApkVersionCode(): Long = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
    longVersionCode
} else {
    @Suppress("DEPRECATION")
    versionCode.toLong()
}

private fun sha256Hex(value: ByteArray): String = MessageDigest.getInstance("SHA-256")
    .digest(value)
    .joinToString(separator = "") { byte -> "%02x".format(byte) }
