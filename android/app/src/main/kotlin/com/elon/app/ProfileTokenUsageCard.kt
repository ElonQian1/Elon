package com.elon.app

import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.view.Gravity
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.elon.app.databinding.ActivityMainBinding
import kotlin.concurrent.thread
import kotlin.math.roundToInt

internal class ProfileTokenUsageCard(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding
) {
    private var root: LinearLayout? = null
    private var loadSerial = 0
    private var selectedDays = 7

    private lateinit var weekButton: TextView
    private lateinit var monthButton: TextView
    private lateinit var gauge: ProfileQuotaGaugeView

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

    private fun buildCard(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            minimumHeight = activity.dp(284)
            orientation = LinearLayout.VERTICAL
            setPadding(activity.dp(14), activity.dp(20), activity.dp(14), activity.dp(12))
            setBackgroundResource(R.drawable.profile_panel_quota)
            isClickable = true
            isFocusable = true
            contentDescription = "Token 额度，点按查看用量明细"
            setOnClickListener { openUsageDetails() }

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "Token 额度"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 17f
            })
            addView(periodActions())
            gauge = ProfileQuotaGaugeView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    activity.dp(190)
                )
                setOnClickListener { openUsageDetails() }
            }
            addView(gauge)
        }
    }

    private fun periodActions(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = activity.dp(13)
            }
            orientation = LinearLayout.HORIZONTAL

            weekButton = periodButton("7天", endMarginDp = 28) {
                selectDays(7)
            }
            monthButton = periodButton("30天", endMarginDp = 28) {
                selectDays(30)
            }
            val rechargeButton = periodButton("充值") {
                showRechargeDialog()
            }
            addView(weekButton)
            addView(monthButton)
            addView(rechargeButton)
            applyPeriodSelection()
        }
    }

    private fun periodButton(
        label: String,
        endMarginDp: Int = 0,
        onClick: () -> Unit
    ): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, activity.dp(38), 1f).apply {
                marginEnd = activity.dp(endMarginDp)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            isClickable = true
            isFocusable = true
            text = label
            textSize = 15f
            setTypeface(typeface, Typeface.NORMAL)
            setOnClickListener { onClick() }
        }
    }

    private fun selectDays(days: Int) {
        if (selectedDays == days) return
        selectedDays = days
        applyPeriodSelection()
        refresh()
    }

    private fun applyPeriodSelection() {
        stylePeriodButton(weekButton, selectedDays == 7)
        stylePeriodButton(monthButton, selectedDays == 30)
    }

    private fun stylePeriodButton(button: TextView, selected: Boolean) {
        button.setBackgroundResource(
            if (selected) R.drawable.profile_pill_selected
            else R.drawable.profile_pill_unselected
        )
        button.setTextColor(
            ContextCompat.getColor(
                activity,
                if (selected) R.color.elon_text_primary
                else R.color.elon_profile_quota_button_text
            )
        )
        button.isSelected = selected
    }

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            loadSerial += 1
            gauge.showState("—", "登录后查看")
            return
        }

        val serial = ++loadSerial
        gauge.showState("…", "同步中")
        val appContext = activity.applicationContext
        val days = selectedDays
        thread(name = "profile-token-usage") {
            val result = runCatching { TokenUsageSummaryClient.fetch(appContext, days) }
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) {
                    return@runOnUiThread
                }
                result
                    .onSuccess(::renderSummary)
                    .onFailure { gauge.showState("—", "暂不可用") }
            }
        }
    }

    private fun renderSummary(summary: TokenUsageSummary) {
        val limit = summary.limitTokens?.takeIf { it > 0 }
        val remaining = summary.remainingTokens
        if (limit == null || remaining == null) {
            gauge.showState("—", "额度未配置")
            return
        }
        val percent = ((remaining.toDouble() / limit) * 100)
            .roundToInt()
            .coerceIn(0, 100)
        gauge.showQuota(percent)
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
}

private fun Context.dp(value: Int): Int =
    (value * resources.displayMetrics.density + 0.5f).toInt()
