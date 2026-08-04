package com.elon.app.update

import android.content.res.ColorStateList
import android.graphics.Color
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.elon.app.R
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog

internal class AppUpdateSheet(
    private val activity: AppCompatActivity,
    private val store: AppUpdateStore,
    private val startDownload: (AppUpdateVersion) -> Unit,
    private val cancelDownload: (AppUpdateVersion) -> Unit,
    private val installUpdate: (AppUpdateSnapshot) -> Unit,
    private val remindLater: (AppUpdateVersion) -> Unit,
) {
    private val content = LayoutInflater.from(activity).inflate(R.layout.sheet_app_update, null, false)
    private val dialog = BottomSheetDialog(activity)
    private val title = content.findViewById<TextView>(R.id.updateTitle)
    private val versionMeta = content.findViewById<TextView>(R.id.updateVersionMeta)
    private val statusDot = content.findViewById<View>(R.id.updateStatusDot)
    private val statusText = content.findViewById<TextView>(R.id.updateStatusText)
    private val progressGroup = content.findViewById<LinearLayout>(R.id.updateProgressGroup)
    private val progress = content.findViewById<ProgressBar>(R.id.updateProgress)
    private val progressPercent = content.findViewById<TextView>(R.id.updateProgressPercent)
    private val progressDetail = content.findViewById<TextView>(R.id.updateProgressDetail)
    private val sourceText = content.findViewById<TextView>(R.id.updateSourceText)
    private val changelogGroup = content.findViewById<LinearLayout>(R.id.updateChangelogGroup)
    private val changelog = content.findViewById<TextView>(R.id.updateChangelog)
    private val primaryButton = content.findViewById<TextView>(R.id.updatePrimaryButton)
    private val secondaryButton = content.findViewById<TextView>(R.id.updateSecondaryButton)

    private var version: AppUpdateVersion? = null
    private var currentSnapshot: AppUpdateSnapshot? = null
    private var stopObserving: (() -> Unit)? = null
    private var explicitAction = false

    val isShowing: Boolean
        get() = dialog.isShowing

    init {
        dialog.setContentView(content)
        dialog.window?.navigationBarColor = ContextCompat.getColor(activity, R.color.elon_bg_chrome)
        dialog.setOnShowListener {
            val bottomSheet = dialog.findViewById<View>(com.google.android.material.R.id.design_bottom_sheet)
            bottomSheet?.setBackgroundColor(Color.TRANSPARENT)
            bottomSheet?.layoutParams?.height = ViewGroup.LayoutParams.WRAP_CONTENT
            bottomSheet?.let {
                BottomSheetBehavior.from(it).apply {
                    state = BottomSheetBehavior.STATE_EXPANDED
                    skipCollapsed = true
                }
            }
        }
        dialog.setOnCancelListener {
            if (!explicitAction && currentSnapshot?.phase == AppUpdatePhase.AVAILABLE) {
                version?.let(remindLater)
            }
        }
        dialog.setOnDismissListener {
            stopObserving?.invoke()
            stopObserving = null
        }
    }

    fun show(updateVersion: AppUpdateVersion) {
        bindVersion(updateVersion)
        stopObserving?.invoke()
        stopObserving = store.observeSnapshot(::render)
        if (!dialog.isShowing) dialog.show()
    }

    fun bindVersion(updateVersion: AppUpdateVersion) {
        version = updateVersion
        versionMeta.text = buildString {
            append("v${updateVersion.versionName}")
            if (updateVersion.fileSize > 0L) append(" · ${formatUpdateBytes(updateVersion.fileSize)}")
            append(" · 可后台下载")
        }
        changelog.text = updateVersion.changelog.ifBlank { "稳定性与体验优化" }
        val snapshot = store.snapshot()?.takeIf { it.versionCode == updateVersion.versionCode }
            ?: AppUpdateSnapshot(
                versionCode = updateVersion.versionCode,
                versionName = updateVersion.versionName,
                phase = AppUpdatePhase.AVAILABLE,
                totalBytes = updateVersion.fileSize,
            )
        render(snapshot)
    }

    private fun render(snapshot: AppUpdateSnapshot?) {
        val updateVersion = version ?: return
        if (snapshot == null || snapshot.versionCode != updateVersion.versionCode) return
        currentSnapshot = snapshot
        when (snapshot.phase) {
            AppUpdatePhase.AVAILABLE -> renderAvailable(updateVersion)
            AppUpdatePhase.QUEUED -> renderDownloading(snapshot, queued = true)
            AppUpdatePhase.DOWNLOADING -> renderDownloading(snapshot, queued = false)
            AppUpdatePhase.VERIFYING -> renderVerifying(snapshot)
            AppUpdatePhase.READY -> renderReady(snapshot)
            AppUpdatePhase.FAILED -> renderFailed(snapshot)
        }
    }

    private fun renderAvailable(updateVersion: AppUpdateVersion) {
        title.text = "发现新版本"
        setStatus("已准备好，可在后台下载", R.color.elon_status_success)
        progressGroup.visibility = View.GONE
        changelogGroup.visibility = View.VISIBLE
        setButtons(
            primary = "后台下载",
            secondary = if (updateVersion.forceUpdate) null else "稍后提醒",
            onPrimary = { startDownload(updateVersion) },
            onSecondary = {
                explicitAction = true
                remindLater(updateVersion)
                dialog.dismiss()
            },
        )
    }

    private fun renderDownloading(snapshot: AppUpdateSnapshot, queued: Boolean) {
        title.text = if (queued) "等待网络连接" else "正在后台下载"
        setStatus(
            if (queued) "任务已加入后台队列" else "离开此页面也会继续下载",
            if (queued) R.color.elon_status_project else R.color.elon_status_info,
        )
        progressGroup.visibility = View.VISIBLE
        changelogGroup.visibility = View.GONE
        progress.isIndeterminate = snapshot.totalBytes <= 0L || queued
        progress.progress = snapshot.progressPercent
        progressPercent.text = if (queued) "等待中" else "${snapshot.progressPercent}%"
        progressDetail.text = progressDetail(snapshot)
        sourceText.text = snapshot.sourceName.ifBlank { "正在选择稳定的下载源" }
        setButtons(
            primary = "隐藏到后台",
            secondary = if (version?.forceUpdate == true) null else "取消下载",
            onPrimary = {
                explicitAction = true
                dialog.dismiss()
            },
            onSecondary = { version?.let(cancelDownload) },
        )
    }

    private fun renderVerifying(snapshot: AppUpdateSnapshot) {
        title.text = "正在校验安装包"
        setStatus("正在核对大小与 SHA-256", R.color.elon_status_project)
        progressGroup.visibility = View.VISIBLE
        changelogGroup.visibility = View.GONE
        progress.isIndeterminate = true
        progressPercent.text = "校验中"
        progressDetail.text = formatUpdateBytes(snapshot.downloadedBytes)
        sourceText.text = "校验完成前不会启动系统安装器"
        setButtons(
            primary = "隐藏到后台",
            secondary = null,
            onPrimary = {
                explicitAction = true
                dialog.dismiss()
            },
        )
    }

    private fun renderReady(snapshot: AppUpdateSnapshot) {
        title.text = "更新已准备好"
        setStatus("完整性校验通过，可以安全安装", R.color.elon_status_success)
        progressGroup.visibility = View.VISIBLE
        changelogGroup.visibility = View.GONE
        progress.isIndeterminate = false
        progress.progress = 100
        progressPercent.text = "100%"
        progressDetail.text = formatUpdateBytes(snapshot.totalBytes)
        sourceText.text = "下载任务已完成，安装由 Android 系统确认"
        setButtons(
            primary = "安装更新",
            secondary = "稍后安装",
            onPrimary = { installUpdate(snapshot) },
            onSecondary = {
                explicitAction = true
                dialog.dismiss()
            },
        )
    }

    private fun renderFailed(snapshot: AppUpdateSnapshot) {
        title.text = "下载暂未完成"
        setStatus(snapshot.errorMessage.ifBlank { "网络连接中断" }, R.color.elon_status_danger)
        progressGroup.visibility = View.VISIBLE
        changelogGroup.visibility = View.GONE
        progress.isIndeterminate = false
        progress.progress = snapshot.progressPercent
        progressPercent.text = "${snapshot.progressPercent}%"
        progressDetail.text = progressDetail(snapshot)
        sourceText.text = "已保留可用进度，重试时将优先续传"
        setButtons(
            primary = "继续下载",
            secondary = "关闭",
            onPrimary = { version?.let(startDownload) },
            onSecondary = {
                explicitAction = true
                dialog.dismiss()
            },
        )
    }

    private fun progressDetail(snapshot: AppUpdateSnapshot): String = buildString {
        if (snapshot.totalBytes > 0L) {
            append("${formatUpdateBytes(snapshot.downloadedBytes)} / ${formatUpdateBytes(snapshot.totalBytes)}")
        }
        if (snapshot.bytesPerSecond > 0L) {
            if (isNotEmpty()) append(" · ")
            append("${formatUpdateBytes(snapshot.bytesPerSecond)}/s")
        }
        formatUpdateEta(snapshot)?.let { eta ->
            if (isNotEmpty()) append(" · ")
            append(eta)
        }
    }

    private fun setStatus(text: String, colorRes: Int) {
        statusText.text = text
        statusDot.backgroundTintList = ColorStateList.valueOf(ContextCompat.getColor(activity, colorRes))
    }

    private fun setButtons(
        primary: String,
        secondary: String?,
        onPrimary: () -> Unit,
        onSecondary: () -> Unit = {},
    ) {
        primaryButton.text = primary
        primaryButton.setOnClickListener { onPrimary() }
        secondaryButton.visibility = if (secondary == null) View.GONE else View.VISIBLE
        secondaryButton.text = secondary.orEmpty()
        secondaryButton.setOnClickListener { onSecondary() }
    }
}
