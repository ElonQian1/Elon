package com.elon.app.update

import android.content.Context
import android.util.Log
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.elon.app.BuildConfig
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/**
 * 后台周期性检查 APP 更新。
 *
 * - 每 [INTERVAL_MINUTES] 分钟执行一次（系统调度，最快 15 分钟）
 * - 仅在有网络时执行
 * - 发现新版本：在通知栏显示更新提示，用户点击后打开 APP 触发下载
 * - 不自动下载，不强制弹窗（减少对用户的打扰）
 */
class UpdateCheckWorker(
    private val context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        return try {
            Log.d(TAG, "后台检查更新开始")
            val info = fetchVersionInfo() ?: return Result.success()
            if (info.versionCode > BuildConfig.VERSION_CODE) {
                Log.i(TAG, "发现新版本 v${info.versionName}(${info.versionCode})，推送通知")
                showNotification(info)
            } else {
                Log.d(TAG, "当前已是最新版本")
            }
            Result.success()
        } catch (e: Exception) {
            Log.w(TAG, "后台检查异常（静默）", e)
            Result.retry()
        }
    }

    private fun fetchVersionInfo(): VersionInfo? {
        return try {
            val http = OkHttpClient.Builder()
                .connectTimeout(10, TimeUnit.SECONDS)
                .readTimeout(15, TimeUnit.SECONDS)
                .build()
            val resp = http.newCall(
                Request.Builder()
                    .url(VERSION_URL)
                    .addHeader("Cache-Control", "no-cache")
                    .build()
            ).execute()
            if (!resp.isSuccessful) return null
            val body = resp.body?.string() ?: return null
            val json = JSONObject(body)
            VersionInfo(
                versionCode = json.optInt("versionCode", 0),
                versionName = json.optString("versionName", ""),
                changelog = json.optString("changelog", "")
            )
        } catch (e: Exception) {
            null
        }
    }

    private fun showNotification(info: VersionInfo) {
        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE)
            as android.app.NotificationManager

        // 通知渠道（Android 8+）
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
            val channel = android.app.NotificationChannel(
                CHANNEL_ID,
                "应用更新",
                android.app.NotificationManager.IMPORTANCE_DEFAULT
            ).apply { description = "一龙 APP 有新版本时通知" }
            manager.createNotificationChannel(channel)
        }

        // 点击通知 → 打开 APP 主界面
        val intent = context.packageManager
            .getLaunchIntentForPackage(context.packageName)
            ?.addFlags(android.content.Intent.FLAG_ACTIVITY_SINGLE_TOP)
        val pendingIntent = android.app.PendingIntent.getActivity(
            context, 0, intent,
            android.app.PendingIntent.FLAG_UPDATE_CURRENT or
                    android.app.PendingIntent.FLAG_IMMUTABLE
        )

        val notification = androidx.core.app.NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_download_done)
            .setContentTitle("一龙有新版本 v${info.versionName}")
            .setContentText(
                if (info.changelog.isNotEmpty()) info.changelog
                else "打开 APP 查看更新内容"
            )
            .setAutoCancel(true)
            .setContentIntent(pendingIntent)
            .build()

        manager.notify(NOTIFICATION_ID, notification)
    }

    private data class VersionInfo(
        val versionCode: Int,
        val versionName: String,
        val changelog: String
    )

    companion object {
        private const val TAG = "UpdateCheckWorker"
        private const val WORK_NAME = "elon_update_check_periodic"
        private const val VERSION_URL = "http://43.139.149.158:8080/app/version.json"
        private const val CHANNEL_ID = "elon_update"
        private const val NOTIFICATION_ID = 9001

        /** Android 最小允许 15 分钟，此处用 6 小时（后台检查不需要太频繁） */
        private const val INTERVAL_MINUTES = 360L

        /** 应用启动时注册周期任务（已存在则保留，不重复注册） */
        fun schedule(context: Context) {
            val request = PeriodicWorkRequestBuilder<UpdateCheckWorker>(
                INTERVAL_MINUTES, TimeUnit.MINUTES
            )
                .setConstraints(
                    Constraints.Builder()
                        .setRequiredNetworkType(NetworkType.CONNECTED)
                        .build()
                )
                .build()

            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                request
            )
            Log.d(TAG, "已注册后台更新检查（每 ${INTERVAL_MINUTES} 分钟）")
        }
    }
}
