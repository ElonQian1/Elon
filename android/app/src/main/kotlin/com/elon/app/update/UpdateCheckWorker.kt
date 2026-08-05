package com.elon.app.update

import android.content.Context
import android.util.Log
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import com.elon.app.BuildConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.concurrent.TimeUnit

/**
 * Activity 不在前台时仍可运行的更新检查。
 *
 * WebSocket 更新信号会触发一次性检查；六小时周期任务只是推送丢失后的兜底。
 * 两条路径都重新读取 version.json、持久化最新版本，并使用同一通知渠道提醒用户。
 */
class UpdateCheckWorker(
    private val context: Context,
    params: WorkerParameters,
) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        if (!appUpdatePolicy(BuildConfig.DEBUG).selfUpdateEnabled) {
            Log.d(TAG, "UI 调试版由 PC 节点管理，跳过正式版更新检查")
            return@withContext Result.success()
        }
        val store = AppUpdateStore(context)
        store.pruneInstalledOrOlder(BuildConfig.VERSION_CODE)
        val expectedVersionCode = inputData.getInt(KEY_EXPECTED_VERSION_CODE, 0)
        if (expectedVersionCode in 1..BuildConfig.VERSION_CODE) {
            return@withContext Result.success()
        }

        store.markCheckAttempt()
        val version = AppUpdateRepository().fetchLatest() ?: return@withContext Result.retry()
        if (version.versionCode <= BuildConfig.VERSION_CODE) {
            store.pruneInstalledOrOlder(BuildConfig.VERSION_CODE)
            return@withContext Result.success()
        }

        store.recordAvailable(version)
        AppUpdateNotifications.notifyAvailable(context, version)
        Log.i(TAG, "发现新版本 v${version.versionName}(${version.versionCode})")
        Result.success()
    }

    companion object {
        private const val TAG = "UpdateCheckWorker"
        private const val PERIODIC_WORK_NAME = "elon_update_check_periodic"
        private const val IMMEDIATE_WORK_NAME = "elon_update_check_realtime"
        private const val KEY_EXPECTED_VERSION_CODE = "expected_version_code"
        private const val INTERVAL_HOURS = 6L

        fun schedule(context: Context) {
            if (!appUpdatePolicy(BuildConfig.DEBUG).selfUpdateEnabled) {
                WorkManager.getInstance(context).cancelUniqueWork(PERIODIC_WORK_NAME)
                return
            }
            val request = PeriodicWorkRequestBuilder<UpdateCheckWorker>(
                INTERVAL_HOURS,
                TimeUnit.HOURS,
            )
                .setConstraints(networkConstraints())
                .build()
            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                PERIODIC_WORK_NAME,
                ExistingPeriodicWorkPolicy.UPDATE,
                request,
            )
        }

        fun enqueueImmediate(context: Context, expectedVersionCode: Int = 0) {
            if (!appUpdatePolicy(BuildConfig.DEBUG).selfUpdateEnabled) return
            val request = OneTimeWorkRequestBuilder<UpdateCheckWorker>()
                .setInputData(workDataOf(KEY_EXPECTED_VERSION_CODE to expectedVersionCode))
                .setConstraints(networkConstraints())
                .build()
            WorkManager.getInstance(context).enqueueUniqueWork(
                IMMEDIATE_WORK_NAME,
                ExistingWorkPolicy.REPLACE,
                request,
            )
        }

        private fun networkConstraints(): Constraints = Constraints.Builder()
            .setRequiredNetworkType(NetworkType.CONNECTED)
            .build()
    }
}
