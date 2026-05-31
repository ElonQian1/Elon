package com.elon.app

import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import kotlin.concurrent.thread

internal class ProfileTokenUsageCard(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding
) {
    private var root: LinearLayout? = null
    private var loadSerial = 0

    private lateinit var statusPill: TextView
    private lateinit var remainingText: TextView
    private lateinit var remainingCaption: TextView
    private lateinit var progressBar: ProgressBar
    private lateinit var progressLabel: TextView
    private lateinit var monthValue: TextView
    private lateinit var weekValue: TextView
    private lateinit var limitValue: TextView
    private lateinit var detailButton: TextView
    private lateinit var rechargeButton: TextView

    fun attachAndRefresh() {
        val host = binding.profileUsageContainer
        val card = root ?: buildCard().also { root = it }
        if (card.parent !== host) {
            (card.parent as? ViewGroup)?.removeView(card)
            host.removeAllViews()
            host.addView(card)
        }
        refresh()
    }

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            loadSerial += 1
            renderLoggedOut()
            return
        }

        val serial = ++loadSerial
        renderLoading()
        val appContext = activity.applicationContext
        thread(name = "profile-token-usage") {
            val result = runCatching { TokenUsageSummaryClient.fetch(appContext) }
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) {
                    return@runOnUiThread
                }
                result
                    .onSuccess { renderSummary(it) }
                    .onFailure { renderError(it.message ?: "加载失败") }
            }
        }
    }

    private fun buildCard(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.VERTICAL
            setPadding(activity.dp(22), activity.dp(18), activity.dp(22), activity.dp(18))
            background = roundedRect(Color.parseColor("#171B20"), activity.dp(8))
            isClickable = true
            setOnClickListener { openUsageDetails() }

            addView(headerRow())
            addView(remainingBlock())
            addView(metricRow())
            addView(actionRow())
        }
    }

    private fun headerRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL

            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                includeFontPadding = false
                text = "Token 额度"
                setTextColor(Color.parseColor("#E6EEF8"))
                textSize = 16f
                setTypeface(typeface, Typeface.BOLD)
            })

            statusPill = TextView(activity).apply {
                includeFontPadding = false
                gravity = Gravity.CENTER
                setPadding(activity.dp(10), activity.dp(5), activity.dp(10), activity.dp(5))
                text = "近30天"
                textSize = 12f
                setTextColor(Color.parseColor("#A8F0C0"))
                background = roundedRect(Color.parseColor("#163221"), activity.dp(8))
            }
            addView(statusPill)
        }
    }

    private fun remainingBlock(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = activity.dp(18)
            }
            orientation = LinearLayout.VERTICAL

            remainingText = TextView(activity).apply {
                includeFontPadding = false
                text = "读取中..."
                setTextColor(Color.WHITE)
                textSize = 32f
                setTypeface(typeface, Typeface.BOLD)
            }
            addView(remainingText)

            remainingCaption = TextView(activity).apply {
                includeFontPadding = false
                text = "正在同步服务器用量"
                setTextColor(Color.parseColor("#9BA7B5"))
                textSize = 13f
                setPadding(0, activity.dp(8), 0, 0)
            }
            addView(remainingCaption)

            progressBar = ProgressBar(activity, null, android.R.attr.progressBarStyleHorizontal).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    activity.dp(8)
                ).apply {
                    topMargin = activity.dp(14)
                }
                max = 1000
                progress = 0
                progressTintList = ColorStateList.valueOf(Color.parseColor("#07C160"))
                progressBackgroundTintList = ColorStateList.valueOf(Color.parseColor("#2A3038"))
            }
            addView(progressBar)

            progressLabel = TextView(activity).apply {
                includeFontPadding = false
                text = "额度读取中"
                setTextColor(Color.parseColor("#728092"))
                textSize = 12f
                setPadding(0, activity.dp(8), 0, 0)
            }
            addView(progressLabel)
        }
    }

    private fun metricRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = activity.dp(18)
            }
            orientation = LinearLayout.HORIZONTAL

            monthValue = addMetric("30天已用")
            weekValue = addMetric("7天已用")
            limitValue = addMetric("总额度")
        }
    }

    private fun LinearLayout.addMetric(label: String): TextView {
        val box = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            orientation = LinearLayout.VERTICAL
        }
        box.addView(TextView(activity).apply {
            includeFontPadding = false
            text = label
            setTextColor(Color.parseColor("#7C8898"))
            textSize = 11f
        })
        val value = TextView(activity).apply {
            includeFontPadding = false
            text = "—"
            setTextColor(Color.parseColor("#E5EAF1"))
            textSize = 15f
            setTypeface(typeface, Typeface.BOLD)
            setPadding(0, activity.dp(7), 0, 0)
        }
        box.addView(value)
        addView(box)
        return value
    }

    private fun actionRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = activity.dp(18)
            }
            orientation = LinearLayout.HORIZONTAL

            detailButton = actionButton("查看明细", "#243141", "#DCE9FF", endMarginDp = 8).apply {
                setOnClickListener { openUsageDetails() }
            }
            rechargeButton = actionButton("充值额度", "#07C160", "#07140C").apply {
                setOnClickListener { showRechargeDialog() }
            }
            addView(detailButton)
            addView(rechargeButton)
        }
    }

    private fun actionButton(
        textValue: String,
        bgColor: String,
        textColor: String,
        endMarginDp: Int = 0
    ): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, activity.dp(42), 1f).apply {
                marginEnd = activity.dp(endMarginDp)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            isClickable = true
            text = textValue
            setTextColor(Color.parseColor(textColor))
            textSize = 14f
            setTypeface(typeface, Typeface.BOLD)
            background = roundedRect(Color.parseColor(bgColor), activity.dp(8))
        }
    }

    private fun renderLoading() {
        statusPill.text = "同步中"
        statusPill.setTextColor(Color.parseColor("#D7E8FF"))
        statusPill.background = roundedRect(Color.parseColor("#253448"), activity.dp(8))
        remainingText.text = "读取中..."
        remainingCaption.text = "正在同步服务器用量"
        updateProgress(0, Color.parseColor("#5FA8FF"))
        progressLabel.text = "额度读取中"
        monthValue.text = "—"
        weekValue.text = "—"
        limitValue.text = "—"
        detailButton.text = "查看明细"
        detailButton.setOnClickListener { openUsageDetails() }
    }

    private fun renderLoggedOut() {
        statusPill.text = "未登录"
        statusPill.setTextColor(Color.parseColor("#FFD8A8"))
        statusPill.background = roundedRect(Color.parseColor("#3A2818"), activity.dp(8))
        remainingText.text = "登录后查看"
        remainingCaption.text = "登录账号后显示剩余额度和 token 消耗"
        updateProgress(0, Color.parseColor("#FFB65C"))
        progressLabel.text = "未连接账号额度"
        monthValue.text = "—"
        weekValue.text = "—"
        limitValue.text = "—"
        detailButton.text = "登录账号"
        detailButton.setOnClickListener {
            activity.startActivity(Intent(activity, LoginActivity::class.java))
        }
    }

    private fun renderSummary(summary: TokenUsageSummary) {
        statusPill.text = "近${summary.days}天"
        statusPill.setTextColor(Color.parseColor("#A8F0C0"))
        statusPill.background = roundedRect(Color.parseColor("#163221"), activity.dp(8))

        val remaining = summary.remainingTokens
        if (remaining == null) {
            remainingText.text = "未配置"
            remainingCaption.text = "服务器暂未返回剩余额度"
            updateProgress(0, Color.parseColor("#8FA0B8"))
            progressLabel.text = "额度上限未配置"
        } else {
            remainingText.text = TokenUsageSummaryClient.formatCount(remaining)
            remainingCaption.text = "剩余 tokens${summary.resetText?.let { " · $it" }.orEmpty()}"
            val percent = remainingPercent(summary)
            updateProgress(percent, progressColor(percent))
            progressLabel.text = summary.limitTokens
                ?.takeIf { it > 0 }
                ?.let { "剩余 ${percent / 10}% · 总额度 ${TokenUsageSummaryClient.formatTokens(it)}" }
                ?: "剩余 ${TokenUsageSummaryClient.formatTokens(remaining)}"
        }

        monthValue.text = TokenUsageSummaryClient.formatCount(summary.totalTokens)
        weekValue.text = TokenUsageSummaryClient.formatCount(summary.weekTokens)
        limitValue.text = summary.limitTokens?.let { TokenUsageSummaryClient.formatCount(it) } ?: "未配置"
        detailButton.text = "查看明细"
        detailButton.setOnClickListener { openUsageDetails() }
    }

    private fun renderError(message: String) {
        statusPill.text = "暂不可用"
        statusPill.setTextColor(Color.parseColor("#FFC3C3"))
        statusPill.background = roundedRect(Color.parseColor("#3A1E1E"), activity.dp(8))
        remainingText.text = "加载失败"
        remainingCaption.text = message
        updateProgress(0, Color.parseColor("#E86F6F"))
        progressLabel.text = "稍后进入“查看明细”可重试"
        monthValue.text = "—"
        weekValue.text = "—"
        limitValue.text = "—"
    }

    private fun remainingPercent(summary: TokenUsageSummary): Int {
        val limit = summary.limitTokens?.takeIf { it > 0 } ?: return 1000
        val remaining = summary.remainingTokens ?: return 0
        return ((remaining.toDouble() / limit) * 1000).toInt().coerceIn(0, 1000)
    }

    private fun progressColor(progress: Int): Int = when {
        progress <= 150 -> Color.parseColor("#FF6B6B")
        progress <= 350 -> Color.parseColor("#FFB65C")
        else -> Color.parseColor("#07C160")
    }

    private fun updateProgress(progress: Int, color: Int) {
        progressBar.progress = progress.coerceIn(0, 1000)
        progressBar.progressTintList = ColorStateList.valueOf(color)
        progressBar.progressBackgroundTintList = ColorStateList.valueOf(Color.parseColor("#2A3038"))
    }

    private fun openUsageDetails() {
        if (!AuthManager.isLoggedIn(activity)) {
            activity.startActivity(Intent(activity, LoginActivity::class.java))
            return
        }
        activity.startActivity(Intent(activity, TokenUsageActivity::class.java))
    }

    private fun showRechargeDialog() {
        val userId = AuthManager.userId(activity) ?: AuthManager.effectiveUserId(activity)
        val account = AuthManager.account(activity) ?: "未登录"
        AlertDialog.Builder(activity)
            .setTitle("充值额度")
            .setMessage(
                "当前充值通道由管理员开通。\n\n账号：$account\n用户 ID：$userId\n\n复制用户 ID 发给管理员，补充额度后这里会自动刷新。"
            )
            .setPositiveButton("复制用户 ID") { _, _ -> copyUserId(userId) }
            .setNeutralButton("查看明细") { _, _ -> openUsageDetails() }
            .setNegativeButton("关闭", null)
            .show()
    }

    private fun copyUserId(userId: String) {
        val manager = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        manager.setPrimaryClip(ClipData.newPlainText("elon_user_id", userId))
        Toast.makeText(activity, "已复制用户 ID", Toast.LENGTH_SHORT).show()
    }

    private fun roundedRect(color: Int, radius: Int): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = radius.toFloat()
            setColor(color)
        }
}

private fun Context.dp(value: Int): Int =
    (value * resources.displayMetrics.density).toInt()
