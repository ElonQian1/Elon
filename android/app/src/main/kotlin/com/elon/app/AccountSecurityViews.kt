package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.ScrollView
import com.google.android.material.button.MaterialButton
import androidx.appcompat.app.AlertDialog

internal fun renderAccountSessions(
    context: Context,
    container: LinearLayout,
    sessions: List<AccountSecuritySession>,
    onRevoke: (AccountSecuritySession) -> Unit,
) {
    container.removeAllViews()
    sessions.forEach { session ->
        val row = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = android.view.Gravity.CENTER_VERTICAL
            setPadding(mobileAuthDp(context, 16), mobileAuthDp(context, 12), mobileAuthDp(context, 8), mobileAuthDp(context, 12))
            setBackgroundResource(R.drawable.bg_mobile_account_panel)
        }
        val label = TextView(context).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            text = buildString {
                append(session.deviceName)
                if (session.current) append(" · 当前设备")
                if (session.trusted) append(" · 已信任")
                append('\n').append(session.lastSeenAt ?: "尚无最近活动时间")
            }
            setTextColor(MobileColors(context).text)
            textSize = 14f
        }
        val revoke = MaterialButton(context).apply {
            text = if (session.current) "退出" else "撤销"
            minHeight = mobileAuthDp(context, 48)
            setOnClickListener { onRevoke(session) }
        }
        row.addView(label)
        row.addView(revoke)
        container.addView(row)
        val spacer = View(context).apply {
            layoutParams = LinearLayout.LayoutParams(1, mobileAuthDp(context, 8))
        }
        container.addView(spacer)
    }
}

internal fun showOneTimeRecoveryCodes(context: Context, codes: List<String>) {
    val value = codes.joinToString("\n")
    val text = TextView(context).apply {
        setPadding(mobileAuthDp(context, 24), mobileAuthDp(context, 16), mobileAuthDp(context, 24), mobileAuthDp(context, 16))
        setTextColor(MobileColors(context).text)
        textSize = 14f
        setTextIsSelectable(true)
        this.text = value.ifBlank { "恢复码没有再次返回；请重新生成一组。" }
    }
    AlertDialog.Builder(context)
        .setTitle("只显示一次的恢复码")
        .setMessage("请立即保存到可信密码管理器。生成后服务端只保留哈希，无法再次查看。")
        .setView(ScrollView(context).apply { addView(text) })
        .setNegativeButton("关闭", null)
        .setPositiveButton("复制全部") { _, _ ->
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("一龙账号恢复码", value))
        }
        .show()
}
