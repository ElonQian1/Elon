package com.elon.app

import android.content.Intent
import android.provider.Settings
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SwitchCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.elon.app.mcp.McpDebugKeepAliveService

/** Android-only service controls; lifecycle refresh also reflects notification stop actions. */
internal fun installChatNotificationSettings(activity: AppCompatActivity) {
    val anchor = activity.findViewById<TextView>(R.id.agentReadinessText)
    val parent = anchor.parent as LinearLayout
    val section = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        val padding = (12 * resources.displayMetrics.density).toInt()
        setPadding(padding, padding, padding, padding)
        setBackgroundResource(R.drawable.bg_orbital_panel)
    }
    fun description(text: String) = TextView(activity).apply {
        this.text = text; textSize = 13f
        setTextColor(ContextCompat.getColor(activity, R.color.elon_text_secondary))
    }
    fun toggle(title: String) = SwitchCompat(activity).apply {
        text = title; textSize = 16f
        minHeight = (48 * resources.displayMetrics.density).toInt()
        setTextColor(ContextCompat.getColor(activity, R.color.elon_text_primary))
    }
    val background = toggle("后台收消息")
    val debug = toggle("后台调试连接")
    var refreshing = false
    fun refresh() {
        refreshing = true
        background.isChecked = ChatBackgroundPrefs.isKeepAliveEnabled(activity)
        debug.isChecked = McpDebugKeepAliveService.shouldAutoStart(activity)
        refreshing = false
    }
    section.addView(description("消息与后台运行"))
    section.addView(background)
    section.addView(description("开启后保留一条静默状态通知，接收好友和群聊消息。关闭后，离开应用可能延迟收消息。"))
    section.addView(Button(activity).apply {
        text = "消息声音与系统通知"
        setOnClickListener {
            activity.startActivity(Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                .putExtra(Settings.EXTRA_APP_PACKAGE, activity.packageName))
        }
    })
    section.addView(debug)
    section.addView(description("默认关闭。仅在需要桌面工具持续连接手机调试时开启；不影响正常聊天。"))
    parent.addView(section, parent.indexOfChild(anchor))
    refresh()
    background.setOnCheckedChangeListener { _, enabled ->
        if (!refreshing) {
            ChatBackgroundPrefs.setKeepAliveEnabled(activity, enabled)
            if (enabled) ChatBackgroundService.start(activity) else ChatBackgroundService.stop(activity)
            if (enabled && !AuthManager.isLoggedIn(activity)) Toast.makeText(activity, "登录后开始后台收消息", Toast.LENGTH_SHORT).show()
        }
    }
    debug.setOnCheckedChangeListener { _, enabled ->
        if (!refreshing) runCatching {
            if (enabled) McpDebugKeepAliveService.requestStart(activity) else McpDebugKeepAliveService.requestStop(activity)
        }.onFailure {
            McpDebugKeepAliveService.requestStop(activity)
            refresh()
            Toast.makeText(activity, "后台调试未能开启，请重试", Toast.LENGTH_SHORT).show()
        }
    }
    activity.lifecycle.addObserver(object : DefaultLifecycleObserver {
        override fun onResume(owner: LifecycleOwner) = refresh()
    })
}
