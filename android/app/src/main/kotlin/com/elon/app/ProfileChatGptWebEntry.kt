package com.elon.app

import android.content.Intent
import android.view.Gravity
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebTestActivity
import com.elon.app.databinding.ActivityMainBinding

internal class ProfileChatGptWebEntry(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
) {
    private var root: LinearLayout? = null

    fun attach() {
        val host = binding.profilePrimaryActionsCard
        val row = root ?: buildRow().also { root = it }
        if (row.parent !== host) {
            (row.parent as? ViewGroup)?.removeView(row)
            val settingsIndex = host.indexOfChild(binding.profileSettingsButton).coerceAtLeast(0)
            host.addView(row, settingsIndex)
        }
    }

    private fun buildRow() = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(76),
        )
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        isClickable = true
        isFocusable = true
        contentDescription = "ChatGPT 网页账号，在本机登录或打开"
        setPadding(dp(16), 0, dp(10), 0)
        foreground = activity.obtainStyledAttributes(
            intArrayOf(android.R.attr.selectableItemBackground),
        ).let { values -> values.getDrawable(0).also { values.recycle() } }
        setOnClickListener {
            activity.startActivity(Intent(activity, ChatGptWebTestActivity::class.java))
        }

        addView(ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(32), dp(32))
            contentDescription = null
            scaleType = ImageView.ScaleType.FIT_CENTER
            setImageResource(R.drawable.profile_icon_ai_settings)
        })
        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1f,
            ).also { it.marginStart = dp(16) }
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "ChatGPT 网页账号"
                setTextColor(activity.elonColor(R.color.elon_text_primary))
                textSize = 16f
            })
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                ).also { it.topMargin = dp(5) }
                includeFontPadding = false
                maxLines = 1
                text = "在本机登录并进入 ChatGPT"
                setTextColor(activity.elonColor(R.color.profile_text_secondary))
                textSize = 12f
            })
        })
        addView(ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(32), dp(48))
            setPadding(dp(8), dp(8), dp(8), dp(8))
            contentDescription = null
            scaleType = ImageView.ScaleType.FIT_CENTER
            setImageResource(R.drawable.profile_icon_chevron)
        })
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()
}
