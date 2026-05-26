package com.elon.app.update

import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import com.elon.app.BuildConfig
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * 应用自更新管理器
 *
 * 工作流程:
 * 1. 从服务器 /app/version.json 获取最新版本信息
 * 2. 对比本地 versionCode，判断是否需要更新
 * 3. 下载 APK 到 getExternalFilesDir(null)/elon_update.apk
 * 4. 通过 FileProvider + 系统安装器安装
 *
 * 服务端只需在 $DATA_DIR/app/ 目录放置:
 *   - version.json      版本信息（由 publish-apk.ps1 生成）
 *   - ElonSpeed-latest.apk  最新 APK
 */
class AppUpdateManager(private val activity: AppCompatActivity) {

    companion object {
        private const val VERSION_URL = "http://43.139.149.158:8080/app/version.json"
        private const val PREFS_NAME = "elon_update"
        private const val KEY_DISMISSED_CODE = "dismissed_code"
        private const val KEY_DISMISSED_AT = "dismissed_at"
        private const val KEY_LAST_CHECK = "last_check"
        private const val KEY_REALTIME_PROMPT_CODE = "realtime_prompt_code"
        private const val KEY_REALTIME_PROMPT_AT = "realtime_prompt_at"
        /** 自动检查冷却时间：12 小时 */
        private const val AUTO_CHECK_INTERVAL_MS = 12 * 60 * 60 * 1000L
        /** 用户点"稍后"后屏蔽同版本弹窗：3 天 */
        private const val DISMISS_EXPIRY_MS = 3 * 24 * 60 * 60 * 1000L
        /** 防止通知点击和队列恢复同时触发两个弹窗 */
        private const val REALTIME_PROMPT_DEDUPE_MS = 30 * 1000L
    }

    private val prefs: SharedPreferences =
        activity.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    private val http = OkHttpClient.Builder()
        .connectTimeout(8, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .build()

    /** 对同WiFi 种子节点使用较短超时（对方可能离线或半死状态，快速失败回落服务器） */
    private val mirrorHttp = OkHttpClient.Builder()
        .connectTimeout(4, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()

    // ── 数据类 ──────────────────────────────────────────────

    private data class MirrorSource(
        val url: String,
        val type: String,
        val priority: Int
    )

    private data class VersionInfo(
        val versionCode: Int,
        val versionName: String,
        val downloadUrl: String,
        val changelog: String,
        val forceUpdate: Boolean,
        val fileSize: Long,
        val mirrors: List<MirrorSource> = emptyList()
    )

    // ── 公共 API ────────────────────────────────────────────────────────────

    /**
     * 自动检查（APP 启动时调用）：
     * - 遵守 12 小时冷却期
     * - 用户忽略过的版本 3 天内不再提示
     * - 静默失败，不打扰用户
     */
    fun autoCheck() {
        val lastCheck = prefs.getLong(KEY_LAST_CHECK, 0)
        if (System.currentTimeMillis() - lastCheck < AUTO_CHECK_INTERVAL_MS) return

        Thread {
            val info = fetchVersionInfo() ?: return@Thread
            prefs.edit().putLong(KEY_LAST_CHECK, System.currentTimeMillis()).apply()

            if (info.versionCode <= BuildConfig.VERSION_CODE) return@Thread
            if (isDismissedRecently(info.versionCode)) return@Thread

            activity.runOnUiThread { showUpdateDialog(info) }
        }.start()
    }

    /**
     * 手动检查（用户从菜单主动触发）：
     * - 始终显示检查结果
     * - 忽略冷却期
     */
    fun manualCheck() {
        val loadingDialog = AlertDialog.Builder(activity)
            .setTitle("检查更新")
            .setMessage("正在检查...")
            .setCancelable(false)
            .show()

        Thread {
            val info = fetchVersionInfo()
            prefs.edit().putLong(KEY_LAST_CHECK, System.currentTimeMillis()).apply()

            activity.runOnUiThread {
                loadingDialog.dismiss()
                when {
                    info == null ->
                        toast("检查失败，请检查网络后重试")
                    info.versionCode <= BuildConfig.VERSION_CODE ->
                        toast("已是最新版本 v${BuildConfig.VERSION_NAME}")
                    else ->
                        showUpdateDialog(info)
                }
            }
        }.start()
    }

    /**
     * WebSocket 收到服务端实时更新事件后调用：
     * 事件只作为提醒信号，真正弹窗前仍重新拉取 version.json，避免使用过期数据。
     */
    fun realtimeCheck(remoteVersionCode: Int = 0) {
        if (remoteVersionCode > 0 && remoteVersionCode <= BuildConfig.VERSION_CODE) return

        Thread {
            val info = fetchVersionInfo() ?: return@Thread
            prefs.edit().putLong(KEY_LAST_CHECK, System.currentTimeMillis()).apply()

            if (info.versionCode <= BuildConfig.VERSION_CODE) return@Thread
            if (!info.forceUpdate && isDismissedRecently(info.versionCode)) return@Thread
            if (isRealtimePromptedMomentsAgo(info.versionCode)) return@Thread
            markRealtimePrompted(info.versionCode)

            activity.runOnUiThread { showUpdateDialog(info) }
        }.start()
    }

    // ── 私有方法 ────────────────────────────────────────────────────────────

    private fun fetchVersionInfo(): VersionInfo? = try {
        val request = Request.Builder()
            .url(VERSION_URL)
            .addHeader("Cache-Control", "no-cache")
            .build()
        http.newCall(request).execute().use { resp ->
            if (!resp.isSuccessful) return null
            val body = resp.body?.string() ?: return null
            val json = JSONObject(body)

            // 解析同WiFi 种子的 mirrors 数组
            val mirrors = mutableListOf<MirrorSource>()
            val mirrorsArr = json.optJSONArray("mirrors")
            if (mirrorsArr != null) {
                for (i in 0 until mirrorsArr.length()) {
                    val m = mirrorsArr.optJSONObject(i) ?: continue
                    mirrors.add(
                        MirrorSource(
                            url = m.optString("url", ""),
                            type = m.optString("type", "server"),
                            priority = m.optInt("priority", 0)
                        )
                    )
                }
            }

            VersionInfo(
                versionCode = json.optInt("versionCode", 0),
                versionName = json.optString("versionName", ""),
                downloadUrl = json.optString("downloadUrl", ""),
                changelog = json.optString("changelog", ""),
                forceUpdate = json.optBoolean("forceUpdate", false),
                fileSize = json.optLong("fileSize", 0),
                mirrors = mirrors
            )
        }
    } catch (e: Exception) {
        null
    }

    private fun showUpdateDialog(info: VersionInfo) {
        val message = buildString {
            append("v${info.versionName}")
            if (info.changelog.isNotEmpty()) {
                append("\n\n更新内容：\n${info.changelog}")
            }
            if (info.fileSize > 0) {
                append("\n\n大小：${"%.1f".format(info.fileSize / 1_048_576.0)} MB")
            }
        }

        AlertDialog.Builder(activity)
            .setTitle("发现新版本")
            .setMessage(message)
            .setCancelable(!info.forceUpdate)
            .apply {
                if (!info.forceUpdate) {
                    setNegativeButton("稍后再说") { _, _ ->
                        prefs.edit()
                            .putInt(KEY_DISMISSED_CODE, info.versionCode)
                            .putLong(KEY_DISMISSED_AT, System.currentTimeMillis())
                            .apply()
                    }
                }
            }
            .setPositiveButton("立即更新") { _, _ -> downloadAndInstall(info) }
            .show()
    }

    private fun downloadAndInstall(info: VersionInfo) {
        // 进度弹窗
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
            .setTitle("下载 v${info.versionName}")
            .setView(layout)
            .setCancelable(false)
            .show()

        Thread {
            try {
                // 优先尝试同WiFi 种子节点（按 priority 降序，连接失败则回落到服务器）
                val candidates: List<Pair<OkHttpClient, String>> =
                    info.mirrors
                        .filter { it.url.isNotEmpty() }
                        .sortedByDescending { it.priority }
                        .map { Pair(mirrorHttp, it.url) } +
                    listOf(Pair(http, info.downloadUrl))

                var lastError: Exception? = null
                for ((httpClient, url) in candidates) {
                    try {
                        val isMirror = httpClient === mirrorHttp
                        activity.runOnUiThread {
                            progressText.text = if (isMirror) "正在从同WiFi设备获取..." else "正在从服务器下载..."
                        }
                        val request = Request.Builder().url(url).build()
                        httpClient.newCall(request).execute().use { resp ->
                            if (!resp.isSuccessful) throw Exception("HTTP ${resp.code}")
                            val body = resp.body ?: throw Exception("空响应体")
                            val totalBytes = body.contentLength()
                            val apkFile = File(activity.getExternalFilesDir(null), "elon_update.apk")

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
                                                progressText.text = "$pct%  (${"%.1f".format(downloaded / 1_048_576.0)} MB)"
                                            }
                                        }
                                    }
                                }
                            }

                            activity.runOnUiThread {
                                progressDialog.dismiss()
                                installApk(apkFile)
                            }
                            return@Thread // 下载成功，退出循环
                        }
                    } catch (e: Exception) {
                        lastError = e
                        // 尝试下一个候选
                    }
                }

                // 所有候选全部失败
                throw lastError ?: Exception("未知错误")

            } catch (e: Exception) {
                activity.runOnUiThread {
                    progressDialog.dismiss()
                    toast("下载失败：${e.message}")
                }
            }
        }.start()
    }

    private fun installApk(apkFile: File) {
        // Android 8+ 需要用户开启"安装未知来源"权限
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
            !activity.packageManager.canRequestPackageInstalls()
        ) {
            AlertDialog.Builder(activity)
                .setTitle("需要安装权限")
                .setMessage("安装更新需要允许「安装未知来源应用」，请在设置中开启后重试。")
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

    private fun isDismissedRecently(versionCode: Int): Boolean {
        val code = prefs.getInt(KEY_DISMISSED_CODE, 0)
        if (code != versionCode) return false
        val elapsed = System.currentTimeMillis() - prefs.getLong(KEY_DISMISSED_AT, 0)
        return elapsed < DISMISS_EXPIRY_MS
    }

    private fun isRealtimePromptedMomentsAgo(versionCode: Int): Boolean {
        val code = prefs.getInt(KEY_REALTIME_PROMPT_CODE, 0)
        if (code != versionCode) return false
        val elapsed = System.currentTimeMillis() - prefs.getLong(KEY_REALTIME_PROMPT_AT, 0)
        return elapsed < REALTIME_PROMPT_DEDUPE_MS
    }

    private fun markRealtimePrompted(versionCode: Int) {
        prefs.edit()
            .putInt(KEY_REALTIME_PROMPT_CODE, versionCode)
            .putLong(KEY_REALTIME_PROMPT_AT, System.currentTimeMillis())
            .apply()
    }

    private fun toast(msg: String) =
        Toast.makeText(activity, msg, Toast.LENGTH_LONG).show()
}
