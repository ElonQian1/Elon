package com.elon.app.update

import android.content.ContentProvider
import android.content.ContentValues
import android.content.pm.PackageManager
import android.database.Cursor
import android.net.Uri
import android.os.Binder
import android.os.Build
import android.os.Bundle
import com.elon.app.OfficialQuantApkPolicy
import java.security.MessageDigest

/**
 * 成员项目发版只读检查桥接：把"是否有新版本"这一件事暴露给已签名的量化 APK，
 * 从不通过这条通道交换 token、账号或 APK 字节。调用方身份必须匹配
 * [OfficialQuantApkPolicy] 里为量化 APK 记录的包名和签名指纹。
 */
class ProjectReleaseCheckProvider : ContentProvider() {
    override fun onCreate(): Boolean = true

    @Suppress("DEPRECATION")
    private fun callerTrusted(): Boolean = runCatching {
        val pm = context?.packageManager ?: return false
        val uid = Binder.getCallingUid()
        val packages = pm.getPackagesForUid(uid) ?: return false
        packages.any { packageName ->
            if (packageName != OfficialQuantApkPolicy.PACKAGE_NAME) return@any false
            val flags = if (Build.VERSION.SDK_INT >= 28) {
                PackageManager.GET_SIGNING_CERTIFICATES
            } else {
                PackageManager.GET_SIGNATURES
            }
            val info = pm.getPackageInfo(packageName, flags)
            val signatures = if (Build.VERSION.SDK_INT >= 28) {
                info.signingInfo?.apkContentsSigners
            } else {
                info.signatures
            }
            val signers = signatures.orEmpty().map { signature ->
                MessageDigest.getInstance("SHA-256").digest(signature.toByteArray())
                    .joinToString("") { "%02x".format(it) }
            }.toSet()
            signers == setOf(OfficialQuantApkPolicy.SIGNER_SHA256)
        }
    }.getOrDefault(false)

    override fun call(method: String, arg: String?, extras: Bundle?): Bundle {
        val result = Bundle()
        if (!callerTrusted()) {
            result.putString("status", "error")
            return result
        }
        runCatching {
            require(method == "check_v1") { "UNSUPPORTED_METHOD" }
            val projectId = extras?.getString("project_id")?.trim().orEmpty()
            require(projectId == OfficialQuantApkPolicy.PROJECT_ID) { "UNSUPPORTED_PROJECT" }
            val installedVersionCode = extras?.getLong("installed_version_code", -1L) ?: -1L
            require(installedVersionCode > 0L) { "INVALID_VERSION_CODE" }
            when (val outcome = ProjectReleaseCheck.check(context!!, projectId, installedVersionCode)) {
                is ProjectReleaseCheckResult.NotLoggedIn -> result.putString("status", "not_logged_in")
                is ProjectReleaseCheckResult.UpToDate -> result.putString("status", "up_to_date")
                is ProjectReleaseCheckResult.Error -> result.putString("status", "error")
                is ProjectReleaseCheckResult.UpdateAvailable -> {
                    result.putString("status", "update_available")
                    result.putLong("version_code", outcome.versionCode)
                    outcome.versionName?.let { result.putString("version_name", it) }
                    outcome.changelog?.let { result.putString("changelog", it) }
                }
            }
        }.onFailure { result.putString("status", "error") }
        return result
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor? = throw SecurityException("UNSUPPORTED")

    override fun getType(uri: Uri): String? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = throw SecurityException("UNSUPPORTED")
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int =
        throw SecurityException("UNSUPPORTED")
    override fun update(
        uri: Uri,
        values: ContentValues?,
        selection: String?,
        selectionArgs: Array<out String>?,
    ): Int = throw SecurityException("UNSUPPORTED")
}
