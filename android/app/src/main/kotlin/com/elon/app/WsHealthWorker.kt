package com.elon.app

import android.content.Context
import android.util.Log
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import java.util.concurrent.TimeUnit

/**
 * WorkManager 后台周期任务：确保聊天 WebSocket 保活服务处于运行状态。
 *
 * 触发时机：每 15 分钟（Android WorkManager 最小间隔），仅在有网络时执行。
 *
 * 解决场景：
 *  - 前台服务被系统或厂商 ROM 杀死，但进程本身还在（孤儿状态）
 *  - Doze 唤醒后 NetworkCallback 未能及时触发
 *  - 长时间后台后 WS 静默断开但没有触发 onClosed/onFailure
 */
class WsHealthWorker(
    private val context: Context,
    params: WorkerParameters,
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        if (!AuthManager.isLoggedIn(context) ||
            !ChatBackgroundPrefs.isKeepAliveEnabled(context)
        ) {
            return Result.success()
        }
        Log.d(TAG, "健康检查：确保保活服务和 WS 在线")
        // 保证前台服务存在
        ChatBackgroundService.start(context)
        // 保证 WS 已连接（已连则 no-op）
        (context.applicationContext as? ElonApplication)?.globalWs?.start(context)
        return Result.success()
    }

    companion object {
        private const val TAG = "WsHealthWorker"
        private const val WORK_NAME = "ws_health_check_periodic"

        /** 应用启动时调度（已存在则保留，不重复注册）。 */
        fun schedule(context: Context) {
            val request = PeriodicWorkRequestBuilder<WsHealthWorker>(
                15, TimeUnit.MINUTES
            ).setConstraints(
                Constraints.Builder()
                    .setRequiredNetworkType(NetworkType.CONNECTED)
                    .build()
            ).build()

            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                request
            )
            Log.d(TAG, "已注册 WS 健康心跳（每 15 分钟）")
        }
    }
}
