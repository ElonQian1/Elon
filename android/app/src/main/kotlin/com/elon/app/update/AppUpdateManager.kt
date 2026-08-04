package com.elon.app.update

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import androidx.lifecycle.lifecycleScope
import com.elon.app.BuildConfig
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.lang.ref.WeakReference
import java.util.concurrent.atomic.AtomicBoolean

/** Activity 侧的轻量更新入口；检测、持久化、下载和 UI 分别由独立模块负责。 */
class AppUpdateManager(private val activity: AppCompatActivity) {
    private val store = AppUpdateStore(activity)
    private val updatePolicy = appUpdatePolicy(BuildConfig.DEBUG)

    fun autoCheck() {
        checkForUpdate(forceNetwork = false, userInitiated = false, prompt = true)
    }

    /** 每次回到前台都先恢复可安装/下载状态，再用短冷却刷新服务器版本。 */
    fun resumeCheck() {
        showStoredUpdateIfNeeded()
        checkForUpdate(forceNetwork = false, userInitiated = false, prompt = true)
    }

    fun manualCheck() {
        if (!ensureSelfUpdateEnabled()) return
        val storedVersion = store.latestVersion()
        val storedSnapshot = store.snapshot()
        if (storedVersion != null &&
            storedSnapshot?.versionCode == storedVersion.versionCode &&
            storedSnapshot.phase in setOf(
                AppUpdatePhase.QUEUED,
                AppUpdatePhase.DOWNLOADING,
                AppUpdatePhase.VERIFYING,
                AppUpdatePhase.READY,
                AppUpdatePhase.FAILED,
            )
        ) {
            showUpdateSheet(storedVersion, ignorePromptDedupe = true)
            return
        }
        toast("正在检查更新…")
        checkForUpdate(forceNetwork = true, userInitiated = true, prompt = true)
    }

    fun realtimeCheck(remoteVersionCode: Int = 0) {
        if (!ensureSelfUpdateEnabled(showMessage = false)) return
        if (remoteVersionCode in 1..BuildConfig.VERSION_CODE) return
        checkForUpdate(forceNetwork = true, userInitiated = false, prompt = true)
    }

    fun openFromNotification() {
        if (!ensureSelfUpdateEnabled()) return
        val version = store.latestVersion()?.takeIf { it.versionCode > BuildConfig.VERSION_CODE }
        if (version != null) {
            showUpdateSheet(version, ignorePromptDedupe = true)
        } else {
            checkForUpdate(forceNetwork = true, userInitiated = true, prompt = true)
        }
    }

    private fun checkForUpdate(forceNetwork: Boolean, userInitiated: Boolean, prompt: Boolean) {
        if (!ensureSelfUpdateEnabled(showMessage = userInitiated)) return
        val now = System.currentTimeMillis()
        if (!forceNetwork && !store.shouldCheck(now, FOREGROUND_CHECK_INTERVAL_MS)) return
        if (!checkInFlight.compareAndSet(false, true)) return
        store.markCheckAttempt(now)

        activity.lifecycleScope.launch {
            val version = try {
                withContext(Dispatchers.IO) { AppUpdateRepository().fetchLatest() }
            } finally {
                checkInFlight.set(false)
            }
            if (activity.isFinishing || activity.isDestroyed) return@launch
            when {
                version == null && userInitiated -> toast("检查失败，请检查网络后重试")
                version == null -> Unit
                version.versionCode <= BuildConfig.VERSION_CODE && userInitiated ->
                    toast("已是最新版本 v${BuildConfig.VERSION_NAME}")
                version.versionCode <= BuildConfig.VERSION_CODE -> Unit
                else -> {
                    store.recordAvailable(version)
                    if (prompt && canPrompt(version, userInitiated)) {
                        showUpdateSheet(version, ignorePromptDedupe = userInitiated)
                    }
                }
            }
        }
    }

    private fun showStoredUpdateIfNeeded() {
        val version = store.latestVersion()?.takeIf { it.versionCode > BuildConfig.VERSION_CODE } ?: return
        val snapshot = store.snapshot()?.takeIf { it.versionCode == version.versionCode }
        val shouldRestore = snapshot?.phase in setOf(AppUpdatePhase.READY, AppUpdatePhase.FAILED)
        if (shouldRestore || canPrompt(version, userInitiated = false)) {
            showUpdateSheet(version, ignorePromptDedupe = shouldRestore)
        }
    }

    private fun canPrompt(version: AppUpdateVersion, userInitiated: Boolean): Boolean {
        if (userInitiated || version.forceUpdate) return true
        val now = System.currentTimeMillis()
        if (store.isDismissed(version.versionCode, now, DISMISS_EXPIRY_MS)) return false
        return !store.wasPromptedRecently(version.versionCode, now, PROMPT_DEDUPE_MS)
    }

    private fun showUpdateSheet(version: AppUpdateVersion, ignorePromptDedupe: Boolean) {
        val existing = activeSheet?.get()
        if (existing?.isShowing == true) {
            existing.bindVersion(version)
            return
        }
        if (!ignorePromptDedupe) store.markPrompted(version.versionCode)
        val sheet = AppUpdateSheet(
            activity = activity,
            store = store,
            startDownload = { AppUpdateDownloadWorker.enqueue(activity, it) },
            cancelDownload = { AppUpdateDownloadWorker.cancel(activity, it) },
            installUpdate = ::installUpdate,
            remindLater = { store.dismiss(it.versionCode) },
        )
        activeSheet = WeakReference(sheet)
        sheet.show(version)
    }

    private fun installUpdate(snapshot: AppUpdateSnapshot) {
        val apkFile = snapshot.apkPath.takeIf { it.isNotBlank() }?.let(::File)
        if (apkFile?.isFile != true) {
            toast("安装包不存在，请重新下载")
            store.latestVersion()?.let { AppUpdateDownloadWorker.enqueue(activity, it) }
            return
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
            !activity.packageManager.canRequestPackageInstalls()
        ) {
            MaterialAlertDialogBuilder(activity)
                .setTitle("允许安装更新")
                .setMessage("Android 需要你允许一龙安装下载好的更新。开启后返回本页，再点一次“安装更新”。")
                .setPositiveButton("前往设置") { _, _ ->
                    activity.startActivity(
                        Intent(
                            Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                            Uri.parse("package:${activity.packageName}"),
                        )
                    )
                }
                .setNegativeButton("稍后安装", null)
                .show()
            return
        }
        val uri = FileProvider.getUriForFile(
            activity,
            "${activity.packageName}.update_provider",
            apkFile,
        )
        activity.startActivity(
            Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION
            }
        )
    }

    private fun ensureSelfUpdateEnabled(showMessage: Boolean = true): Boolean {
        if (updatePolicy.selfUpdateEnabled) return true
        if (showMessage) toast(updatePolicy.disabledMessage ?: "当前安装包不支持应用内更新")
        return false
    }

    private fun toast(message: String) {
        Toast.makeText(activity, message, Toast.LENGTH_LONG).show()
    }

    companion object {
        private const val FOREGROUND_CHECK_INTERVAL_MS = 5 * 60 * 1000L
        private const val DISMISS_EXPIRY_MS = 24 * 60 * 60 * 1000L
        private const val PROMPT_DEDUPE_MS = 60 * 1000L
        private val checkInFlight = AtomicBoolean(false)
        @Volatile
        private var activeSheet: WeakReference<AppUpdateSheet>? = null

        fun profileStatusLine(context: android.content.Context): String {
            val snapshot = AppUpdateStore(context).snapshot()
                ?.takeIf { it.versionCode > BuildConfig.VERSION_CODE }
                ?: return "一龙 v${BuildConfig.VERSION_NAME}  (build ${BuildConfig.VERSION_CODE})"
            return when (snapshot.phase) {
                AppUpdatePhase.AVAILABLE -> "发现新版 v${snapshot.versionName} · 点击下载"
                AppUpdatePhase.QUEUED -> "v${snapshot.versionName} 已加入后台下载队列"
                AppUpdatePhase.DOWNLOADING -> "v${snapshot.versionName} 下载中 ${snapshot.progressPercent}% · 可后台继续"
                AppUpdatePhase.VERIFYING -> "v${snapshot.versionName} 正在校验安装包"
                AppUpdatePhase.READY -> "v${snapshot.versionName} 已下载并校验 · 点击安装"
                AppUpdatePhase.FAILED -> "v${snapshot.versionName} 下载中断 · 点击继续"
            }
        }

        fun observeProfileStatus(
            context: android.content.Context,
            onChanged: (String) -> Unit,
        ): () -> Unit = AppUpdateStore(context).observeSnapshot {
            onChanged(profileStatusLine(context))
        }
    }
}
