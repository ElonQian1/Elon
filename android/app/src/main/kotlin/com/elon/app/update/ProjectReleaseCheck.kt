package com.elon.app.update

import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.ElonApplication
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/**
 * 无 Activity 上下文的成员项目发版检查。
 *
 * 只读版本号比对，不下载 APK 字节、不缓存 token 之外的任何凭据；
 * 供跨进程 [ProjectReleaseCheckProvider] 和主 APP 自己的后台任务共用。
 */
sealed class ProjectReleaseCheckResult {
    object NotLoggedIn : ProjectReleaseCheckResult()
    object UpToDate : ProjectReleaseCheckResult()
    data class UpdateAvailable(
        val versionName: String?,
        val versionCode: Long,
        val changelog: String?,
    ) : ProjectReleaseCheckResult()
    object Error : ProjectReleaseCheckResult()
}

object ProjectReleaseCheck {
    private val http = OkHttpClient.Builder()
        .connectTimeout(8, TimeUnit.SECONDS)
        .readTimeout(8, TimeUnit.SECONDS)
        .build()

    /** 阻塞调用，调用方必须在后台线程执行。 */
    fun check(context: Context, projectId: String, installedVersionCode: Long): ProjectReleaseCheckResult {
        if (installedVersionCode <= 0L) return ProjectReleaseCheckResult.Error
        if (!AuthManager.isLoggedIn(context)) return ProjectReleaseCheckResult.NotLoggedIn
        val token = AuthManager.token(context) ?: return ProjectReleaseCheckResult.NotLoggedIn
        val serverUrl = ElonApplication.activeServerUrl(context).trimEnd('/')
        val request = Request.Builder()
            .url("$serverUrl/api/projects/$projectId/releases")
            .header("Authorization", "Bearer $token")
            .get()
            .build()
        return runCatching {
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@use ProjectReleaseCheckResult.Error
                val body = response.body?.string().orEmpty()
                val releases = JSONObject(body).optJSONArray("releases")
                    ?: return@use ProjectReleaseCheckResult.UpToDate
                var best: ProjectReleaseCheckResult.UpdateAvailable? = null
                for (index in 0 until releases.length()) {
                    val item = releases.optJSONObject(index) ?: continue
                    if (!item.optBoolean("installable", false)) continue
                    if (!item.has("version_code") || item.isNull("version_code")) continue
                    val code = item.optLong("version_code", -1L)
                    if (code <= 0L) continue
                    if (best == null || code > best.versionCode) {
                        best = ProjectReleaseCheckResult.UpdateAvailable(
                            item.optString("version_name").takeIf { it.isNotBlank() },
                            code,
                            item.optString("changelog").takeIf { it.isNotBlank() },
                        )
                    }
                }
                val candidate = best
                if (candidate == null || candidate.versionCode <= installedVersionCode) {
                    ProjectReleaseCheckResult.UpToDate
                } else {
                    candidate
                }
            }
        }.getOrElse { ProjectReleaseCheckResult.Error }
    }
}
