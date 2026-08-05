package com.elon.app.update

import android.content.Context
import com.elon.app.BuildConfig
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.Data
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest
import java.util.concurrent.TimeUnit

internal class AppUpdateDownloadWorker(
    context: Context,
    params: WorkerParameters,
) : CoroutineWorker(context, params) {
    private val store = AppUpdateStore(context)
    private val officialHttp = AppUpdateRepository.defaultClient(readTimeoutSeconds = 45L)
    private val mirrorHttp = AppUpdateRepository.defaultClient(readTimeoutSeconds = 12L)

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        val version = AppUpdateVersion.parse(inputData.getString(KEY_VERSION_JSON).orEmpty())
            ?: return@withContext Result.failure()
        if (version.versionCode <= BuildConfig.VERSION_CODE) {
            store.pruneInstalledOrOlder(BuildConfig.VERSION_CODE)
            deleteDownloadedArtifacts(applicationContext)
            AppUpdateNotifications.cancelDownloadNotification(applicationContext)
            return@withContext Result.success()
        }
        val partFile = File(applicationContext.getExternalFilesDir(null), PART_FILE_NAME)
        val apkFile = File(applicationContext.getExternalFilesDir(null), APK_FILE_NAME)
        val initialBytes = partFile.takeIf { it.isFile }?.length() ?: 0L
        val queued = snapshot(
            version,
            AppUpdatePhase.QUEUED,
            initialBytes,
            version.fileSize,
            source = if (initialBytes > 0L) "准备从断点继续" else "正在启动系统下载服务",
        )
        store.saveSnapshot(queued)

        try {
            try {
                setForeground(AppUpdateNotifications.foregroundInfo(applicationContext, queued))
            } catch (error: Throwable) {
                throw AppUpdateBackgroundServiceException(error)
            }
            if (apkFile.isFile && verifyFile(apkFile, version)) {
                publishReady(version, apkFile)
                return@withContext Result.success()
            }
            if (apkFile.exists()) apkFile.delete()

            var lastError: Throwable? = null
            for (source in version.downloadSources()) {
                currentCoroutineContext().ensureActive()
                try {
                    downloadSource(version, source, partFile)
                    if (!verifyFile(partFile, version)) {
                        throw AppUpdateIntegrityException("文件校验失败，已停止安装")
                    }
                    if (apkFile.exists()) apkFile.delete()
                    if (!partFile.renameTo(apkFile)) {
                        throw IllegalStateException("无法保存已校验的安装包")
                    }
                    publishReady(version, apkFile)
                    return@withContext Result.success()
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (error: Throwable) {
                    lastError = error
                }
            }
            throw lastError ?: IllegalStateException("没有可用的下载源")
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (error: Throwable) {
            val disposition = classifyAppUpdateFailure(error, runAttemptCount)
            val willRetry = disposition == AppUpdateFailureDisposition.RETRY
            val failed = snapshot(
                version = version,
                phase = if (willRetry) AppUpdatePhase.QUEUED else AppUpdatePhase.FAILED,
                downloaded = partFile.takeIf { it.exists() }?.length() ?: 0L,
                total = version.fileSize,
                source = if (willRetry) "等待网络恢复后自动续传" else "",
                error = appUpdateFailureMessage(error),
            )
            store.saveSnapshot(failed)
            if (willRetry) {
                Result.retry()
            } else {
                AppUpdateNotifications.notifyFailed(applicationContext, failed)
                Result.failure(workDataOf(KEY_ERROR to failed.errorMessage))
            }
        }
    }

    private suspend fun downloadSource(
        version: AppUpdateVersion,
        source: AppUpdateSource,
        partFile: File,
        allowResume: Boolean = true,
    ) {
        var resumeBytes = if (allowResume) partFile.takeIf { it.isFile }?.length() ?: 0L else 0L
        currentCoroutineContext().ensureActive()
        val request = Request.Builder().url(source.url).apply {
            if (resumeBytes > 0L) header("Range", "bytes=$resumeBytes-")
        }.build()
        val client: OkHttpClient = if (source.type.equals("server", true)) officialHttp else mirrorHttp
        client.newCall(request).execute().use { response ->
            if (response.code == 416) {
                if (verifyFile(partFile, version)) return
                if (!allowResume) throw IllegalStateException("下载源拒绝重新下载")
                partFile.delete()
                return downloadSource(version, source, partFile, allowResume = false)
            }
            if (!response.isSuccessful) throw AppUpdateHttpException(response.code)
            val body = response.body ?: throw IllegalStateException("下载源没有返回文件")
            val append = response.code == 206 && resumeBytes > 0L
            if (!append) resumeBytes = 0L
            val responseBytes = body.contentLength()
            val totalBytes = when {
                version.fileSize > 0L -> version.fileSize
                responseBytes > 0L -> resumeBytes + responseBytes
                else -> 0L
            }
            streamBody(version, source, body.byteStream(), partFile, append, resumeBytes, totalBytes)
        }
    }

    private suspend fun streamBody(
        version: AppUpdateVersion,
        source: AppUpdateSource,
        input: java.io.InputStream,
        partFile: File,
        append: Boolean,
        startingBytes: Long,
        totalBytes: Long,
    ) {
        var downloaded = startingBytes
        val startedAt = System.currentTimeMillis()
        var lastPublishedAt = 0L
        var lastPercent = -1
        input.use { stream ->
            FileOutputStream(partFile, append).use { output ->
                val buffer = ByteArray(DEFAULT_BUFFER_SIZE * 4)
                while (true) {
                    currentCoroutineContext().ensureActive()
                    val count = stream.read(buffer)
                    if (count < 0) break
                    output.write(buffer, 0, count)
                    downloaded += count
                    val now = System.currentTimeMillis()
                    val percent = progressPercent(downloaded, totalBytes)
                    if (now - lastPublishedAt >= PROGRESS_INTERVAL_MS || percent != lastPercent) {
                        val elapsed = (now - startedAt).coerceAtLeast(1L)
                        val speed = ((downloaded - startingBytes) * 1_000L / elapsed).coerceAtLeast(0L)
                        val current = snapshot(
                            version = version,
                            phase = AppUpdatePhase.DOWNLOADING,
                            downloaded = downloaded,
                            total = totalBytes,
                            speed = speed,
                            source = source.displayName,
                        )
                        store.saveSnapshot(current)
                        setProgress(progressData(current))
                        AppUpdateNotifications.notifyProgress(applicationContext, current)
                        lastPublishedAt = now
                        lastPercent = percent
                    }
                }
                output.fd.sync()
            }
        }
        val verifying = snapshot(
            version,
            AppUpdatePhase.VERIFYING,
            downloaded,
            totalBytes,
            source = source.displayName,
        )
        store.saveSnapshot(verifying)
        AppUpdateNotifications.notifyProgress(applicationContext, verifying)
    }

    private fun publishReady(version: AppUpdateVersion, apkFile: File) {
        val ready = snapshot(
            version = version,
            phase = AppUpdatePhase.READY,
            downloaded = apkFile.length(),
            total = version.fileSize.takeIf { it > 0L } ?: apkFile.length(),
            source = "完整性校验通过",
            apkPath = apkFile.absolutePath,
        )
        store.saveSnapshot(ready)
        AppUpdateNotifications.notifyReady(applicationContext, ready)
    }

    private fun verifyFile(file: File, version: AppUpdateVersion): Boolean {
        if (!file.isFile || file.length() <= 0L) return false
        if (version.fileSize > 0L && file.length() != version.fileSize) return false
        if (version.sha256.isBlank()) return true
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(DEFAULT_BUFFER_SIZE * 4)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                digest.update(buffer, 0, count)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it.toInt() and 0xff) }
            .equals(version.sha256, ignoreCase = true)
    }

    private fun snapshot(
        version: AppUpdateVersion,
        phase: AppUpdatePhase,
        downloaded: Long,
        total: Long,
        speed: Long = 0L,
        source: String = "",
        error: String = "",
        apkPath: String = "",
    ) = AppUpdateSnapshot(
        versionCode = version.versionCode,
        versionName = version.versionName,
        phase = phase,
        downloadedBytes = downloaded,
        totalBytes = total,
        bytesPerSecond = speed,
        sourceName = source,
        errorMessage = error,
        apkPath = apkPath,
    )

    private fun progressData(snapshot: AppUpdateSnapshot): Data = workDataOf(
        KEY_PROGRESS to snapshot.progressPercent,
        KEY_DOWNLOADED to snapshot.downloadedBytes,
        KEY_TOTAL to snapshot.totalBytes,
    )

    companion object {
        private const val UNIQUE_WORK_NAME = "elon_app_update_download"
        private const val KEY_VERSION_JSON = "version_json"
        private const val KEY_PROGRESS = "progress"
        private const val KEY_DOWNLOADED = "downloaded"
        private const val KEY_TOTAL = "total"
        private const val KEY_ERROR = "error"
        private const val PART_FILE_NAME = "elon_update.apk.part"
        private const val APK_FILE_NAME = "elon_update.apk"
        private const val PROGRESS_INTERVAL_MS = 500L

        fun enqueue(context: Context, version: AppUpdateVersion) {
            val request = OneTimeWorkRequestBuilder<AppUpdateDownloadWorker>()
                .setInputData(workDataOf(KEY_VERSION_JSON to version.toJson()))
                .setConstraints(
                    Constraints.Builder()
                        .setRequiredNetworkType(NetworkType.CONNECTED)
                        .build()
                )
                .setBackoffCriteria(
                    BackoffPolicy.EXPONENTIAL,
                    15L,
                    TimeUnit.SECONDS,
                )
                .build()
            AppUpdateStore(context).saveSnapshot(
                AppUpdateSnapshot(
                    versionCode = version.versionCode,
                    versionName = version.versionName,
                    phase = AppUpdatePhase.QUEUED,
                    totalBytes = version.fileSize,
                )
            )
            WorkManager.getInstance(context).enqueueUniqueWork(
                UNIQUE_WORK_NAME,
                ExistingWorkPolicy.REPLACE,
                request,
            )
        }

        fun cancel(context: Context, version: AppUpdateVersion) {
            WorkManager.getInstance(context).cancelUniqueWork(UNIQUE_WORK_NAME)
            AppUpdateStore(context).recordAvailable(version)
            AppUpdateNotifications.cancelDownloadNotification(context)
        }

        fun discardDownloadedPackage(context: Context) {
            WorkManager.getInstance(context).cancelUniqueWork(UNIQUE_WORK_NAME)
            deleteDownloadedArtifacts(context)
            AppUpdateNotifications.cancelDownloadNotification(context)
        }

        private fun deleteDownloadedArtifacts(context: Context) {
            val directory = context.getExternalFilesDir(null) ?: return
            File(directory, PART_FILE_NAME).delete()
            File(directory, APK_FILE_NAME).delete()
        }

        private fun progressPercent(downloaded: Long, total: Long): Int =
            if (total > 0L) ((downloaded * 100L) / total).toInt().coerceIn(0, 100) else 0
    }
}
