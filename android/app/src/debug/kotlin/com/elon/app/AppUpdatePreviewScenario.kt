package com.elon.app

import android.content.Context
import android.content.res.ColorStateList
import android.view.LayoutInflater
import android.view.View
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode

internal fun appUpdatePreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId: String = "elon.app.update"
    override val supportedScenarios: Set<String> = setOf("available", "downloading", "ready", "error")

    override fun createView(context: Context, request: UiRuntimePreviewRequest): View {
        val root = LayoutInflater.from(context).inflate(R.layout.sheet_app_update, null, false)
        root.findViewById<TextView>(R.id.updateVersionMeta).text = "v1.1.853 · 7.8 MB · 可后台下载"
        root.findViewById<TextView>(R.id.updateChangelog).text =
            "更新提醒更可靠；支持后台下载、断点续传与安装包完整性校验。"
        val dot = root.findViewById<View>(R.id.updateStatusDot)
        val status = root.findViewById<TextView>(R.id.updateStatusText)
        val title = root.findViewById<TextView>(R.id.updateTitle)
        val group = root.findViewById<LinearLayout>(R.id.updateProgressGroup)
        val changelog = root.findViewById<LinearLayout>(R.id.updateChangelogGroup)
        val progress = root.findViewById<ProgressBar>(R.id.updateProgress)
        val percent = root.findViewById<TextView>(R.id.updateProgressPercent)
        val detail = root.findViewById<TextView>(R.id.updateProgressDetail)
        val source = root.findViewById<TextView>(R.id.updateSourceText)
        val primary = root.findViewById<TextView>(R.id.updatePrimaryButton)
        val secondary = root.findViewById<TextView>(R.id.updateSecondaryButton)

        fun state(text: String, color: Int) {
            status.text = text
            dot.backgroundTintList = ColorStateList.valueOf(ContextCompat.getColor(context, color))
        }
        when (request.scenario) {
            "downloading" -> {
                title.text = "正在后台下载"
                state("离开此页面也会继续下载", R.color.elon_status_info)
                group.visibility = View.VISIBLE
                changelog.visibility = View.GONE
                progress.progress = 42
                percent.text = "42%"
                detail.text = "3.3 MB / 7.8 MB · 1.2 MB/s · 约 4 秒"
                source.text = "正在从官方服务器下载"
                primary.text = "隐藏到后台"
                secondary.text = "取消下载"
            }
            "ready" -> {
                title.text = "更新已准备好"
                state("完整性校验通过，可以安全安装", R.color.elon_status_success)
                group.visibility = View.VISIBLE
                changelog.visibility = View.GONE
                progress.progress = 100
                percent.text = "100%"
                detail.text = "7.8 MB"
                source.text = "下载任务已完成，安装由 Android 系统确认"
                primary.text = "安装更新"
                secondary.text = "稍后安装"
            }
            "error" -> {
                title.text = "下载暂未完成"
                state("网络中断，已保留进度", R.color.elon_status_danger)
                group.visibility = View.VISIBLE
                changelog.visibility = View.GONE
                progress.progress = 42
                percent.text = "42%"
                detail.text = "3.3 MB / 7.8 MB"
                source.text = "重试时将优先从 42% 继续"
                primary.text = "继续下载"
                secondary.text = "关闭"
            }
            else -> {
                title.text = "发现新版本"
                state("已准备好，可在后台下载", R.color.elon_status_success)
                group.visibility = View.GONE
                changelog.visibility = View.VISIBLE
                primary.text = "后台下载"
                secondary.text = "稍后提醒"
            }
        }
        return root.uiNode("app.update.sheet")
    }
}
