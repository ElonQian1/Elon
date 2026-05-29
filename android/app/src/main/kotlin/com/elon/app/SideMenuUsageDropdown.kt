package com.elon.app

import android.graphics.Color
import android.graphics.drawable.Drawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import kotlin.concurrent.thread

internal class SideMenuUsageDropdown(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    val rowView: LinearLayout = buildRow()
    val detailsView: LinearLayout = buildDetailsContainer()
    private lateinit var arrow: TextView
    private var expanded = false
    private var loadSerial = 0

    fun collapse() {
        setExpanded(false)
    }

    private fun buildRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(38)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(22), 0, dp(18), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { toggle() }
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                includeFontPadding = false
                text = "剩余用量"
                setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
                textSize = 17f
            })
            arrow = TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(22), LinearLayout.LayoutParams.WRAP_CONTENT)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "▾"
                setTextColor(Color.parseColor("#4E4E4E"))
                textSize = 15f
            }
            addView(arrow)
        }
    }

    private fun buildDetailsContainer(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.VERTICAL
            visibility = View.GONE
            setPadding(dp(22), 0, dp(22), dp(8))
        }
    }

    private fun toggle() {
        if (expanded) {
            setExpanded(false)
            return
        }
        setExpanded(true)
        if (!AuthManager.isLoggedIn(activity)) {
            render(
                TokenUsageSummary(
                    listOf(
                        TokenUsageSummaryLine("账号", "未登录"),
                        TokenUsageSummaryLine("剩余", "登录后查看")
                    )
                )
            )
            return
        }
        render(TokenUsageSummary(listOf(TokenUsageSummaryLine("剩余", "读取中"))))
        load()
    }

    private fun setExpanded(value: Boolean) {
        expanded = value
        arrow.text = if (value) "▴" else "▾"
        detailsView.visibility = if (value) View.VISIBLE else View.GONE
        if (!value) loadSerial += 1
    }

    private fun load() {
        val serial = ++loadSerial
        val appContext = activity.applicationContext
        thread(name = "side-menu-token-usage") {
            val result = runCatching { TokenUsageSummaryClient.fetch(appContext) }
            activity.runOnUiThread {
                if (serial != loadSerial || !expanded || activity.isFinishing || activity.isDestroyed) {
                    return@runOnUiThread
                }
                result
                    .onSuccess { render(it) }
                    .onFailure {
                        render(TokenUsageSummary(listOf(TokenUsageSummaryLine("剩余", it.message ?: "加载失败"))))
                    }
            }
        }
    }

    private fun render(summary: TokenUsageSummary) {
        detailsView.removeAllViews()
        summary.lines.forEach { line ->
            detailsView.addView(detailRow(line))
        }
    }

    private fun detailRow(line: TokenUsageSummaryLine): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(24)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                includeFontPadding = false
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                text = line.label
                setTextColor(Color.parseColor("#2F2F2F"))
                textSize = 12.5f
            })
            addView(TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                text = line.value
                setTextColor(Color.parseColor("#2F2F2F"))
                textSize = 12.5f
            })
            line.note?.takeIf { it.isNotBlank() }?.let { note ->
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    text = note
                    setTextColor(Color.parseColor("#666666"))
                    textSize = 11.5f
                    setPadding(dp(8), 0, 0, 0)
                })
            }
        }
    }
}
