package com.elon.app

import android.graphics.Color
import android.graphics.drawable.Drawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.animation.PathInterpolator
import android.widget.ImageView
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
    private val chevronInterpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
    private lateinit var chevron: ImageView
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
            chevron = ImageView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(22), dp(22))
                setImageResource(R.drawable.ic_input_model_chevron)
                scaleType = ImageView.ScaleType.CENTER
                alpha = 0.9f
                rotation = 0f
                isClickable = false
                isFocusable = false
            }
            addView(chevron)
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
            alpha = 0f
            scaleY = 0.97f
            pivotY = 0f
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
        animateChevron(value)
        if (value) {
            showDetails()
        } else {
            loadSerial += 1
            hideDetails()
        }
    }

    private fun showDetails() {
        detailsView.animate().cancel()
        detailsView.visibility = View.VISIBLE
        detailsView.alpha = 0f
        detailsView.scaleY = 0.97f
        detailsView.pivotY = 0f
        detailsView.animate()
            .alpha(1f)
            .scaleY(1f)
            .setDuration(120L)
            .setInterpolator(chevronInterpolator)
            .start()
    }

    private fun hideDetails() {
        detailsView.animate().cancel()
        if (detailsView.visibility != View.VISIBLE) {
            detailsView.alpha = 0f
            detailsView.scaleY = 0.97f
            return
        }
        detailsView.animate()
            .alpha(0f)
            .scaleY(0.97f)
            .setDuration(90L)
            .setInterpolator(chevronInterpolator)
            .withEndAction {
                if (!expanded) detailsView.visibility = View.GONE
            }
            .start()
    }

    private fun animateChevron(value: Boolean) {
        chevron.animate().cancel()
        chevron.animate()
            .rotation(if (value) 180f else 0f)
            .setDuration(140L)
            .setInterpolator(chevronInterpolator)
            .start()
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
                    setTextColor(Color.parseColor("#6F7785"))
                    textSize = 11.5f
                    setPadding(dp(8), 0, 0, 0)
                })
            }
        }
    }
}
