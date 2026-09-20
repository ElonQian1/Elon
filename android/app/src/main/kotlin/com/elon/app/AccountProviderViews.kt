package com.elon.app

import android.app.Activity
import android.content.Intent
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.chatgptweb.ChatGptWebOfficialFallbackIntent
import com.elon.app.grid.host.BinanceAccountActivity
import com.elon.app.grid.host.BinanceHostRuntime

/** Account metadata only; website credentials remain inside their existing WebViews. */
internal fun renderAccountProviders(activity: Activity, container: LinearLayout) {
    container.removeAllViews()
    fun row(title: String, summary: String, action: String, open: () -> Unit) {
        container.addView(TextView(activity).apply {
            text = "$title\n$summary"; textSize = 15f
            setTextColor(activity.getColor(R.color.elon_text_primary))
            setPadding(0, 20, 0, 8)
        })
        container.addView(Button(activity).apply { text = action; setOnClickListener { open() } })
    }
    row("OpenAI / ChatGPT", "登录账户请在本机官网中确认；网页登录状态与 Google 登录绑定分别管理。", "查看／切换 OpenAI 账户") {
        activity.startActivity(ChatGptWebOfficialFallbackIntent.create(activity, currentUrl = null))
    }
    val binance = runCatching { BinanceHostRuntime.onMain(activity) { it.accountSummary() } }
        .getOrDefault("币安账户状态暂不可用")
    row("币安", "$binance\n用于一龙量化 APK；切换后需要重新授权连接。", "管理／切换币安账户") {
        activity.startActivity(Intent(activity, BinanceAccountActivity::class.java))
    }
}
