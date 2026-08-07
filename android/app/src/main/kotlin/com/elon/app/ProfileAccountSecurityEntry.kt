package com.elon.app

import android.content.Intent
import android.view.Gravity
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.elon.app.databinding.ActivityMainBinding
import kotlinx.coroutines.launch

internal class ProfileAccountSecurityEntry(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
) {
    private val auth by lazy { GoogleFederatedAuth(activity) }
    private var root: LinearLayout? = null
    private lateinit var subtitle: TextView
    private var refreshSerial = 0

    fun attachAndRefresh() {
        val host = binding.profilePrimaryActionsCard
        val row = root ?: buildRow().also { root = it }
        if (row.parent !== host) {
            (row.parent as? ViewGroup)?.removeView(row)
            val settingsIndex = host.indexOfChild(binding.profileSettingsButton).coerceAtLeast(0)
            host.addView(row, settingsIndex)
        }
        refresh()
    }

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            subtitle.text = "登录后可绑定 Google"
            return
        }
        val serial = ++refreshSerial
        subtitle.text = "正在读取 Google 绑定状态…"
        activity.lifecycleScope.launch {
            runCatching {
                auth.identities() to auth.isGoogleConfigured()
            }.onSuccess { (identities, configured) ->
                if (serial == refreshSerial) {
                    subtitle.text = googleBindingSummary(identities, configured)
                }
            }.onFailure {
                if (serial == refreshSerial) subtitle.text = "Google 绑定状态暂不可用"
            }
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
        contentDescription = "账号与安全，管理 Google 登录方式"
        setPadding(dp(16), 0, dp(10), 0)
        foreground = activity.obtainStyledAttributes(
            intArrayOf(android.R.attr.selectableItemBackground),
        ).let { values -> values.getDrawable(0).also { values.recycle() } }
        setOnClickListener {
            val destination = if (AuthManager.isLoggedIn(activity)) {
                AccountIdentityActivity::class.java
            } else {
                LoginActivity::class.java
            }
            activity.startActivity(Intent(activity, destination))
        }

        addView(ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(32), dp(32))
            contentDescription = null
            scaleType = ImageView.ScaleType.FIT_CENTER
            setImageResource(R.drawable.profile_icon_account_security)
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
                text = "账号与安全"
                setTextColor(activity.elonColor(R.color.elon_text_primary))
                textSize = 16f
            })
            subtitle = TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                ).also { it.topMargin = dp(5) }
                includeFontPadding = false
                maxLines = 1
                text = "读取 Google 绑定状态…"
                setTextColor(activity.elonColor(R.color.profile_text_secondary))
                textSize = 12f
            }
            addView(subtitle)
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
