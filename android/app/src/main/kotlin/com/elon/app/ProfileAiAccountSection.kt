package com.elon.app

import android.content.Intent
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.esk.compute.EskComputeActivity

/** Navigation only: never displays cached or simulated money as a real balance. */
internal object ProfileAiAccountSection {
    private const val TAG = "profile-ai-account-section"

    fun attach(activity: AppCompatActivity, binding: ActivityMainBinding) {
        val host = binding.profileEskAssetContainer
        if (host.findViewWithTag<View>(TAG) != null) return
        val density = activity.resources.displayMetrics.density
        fun dp(value: Int) = (value * density + .5f).toInt()
        val section = LinearLayout(activity).apply {
            tag = TAG
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(12), dp(16), dp(12))
            setBackgroundResource(R.color.elon_surface_card)
            layoutParams = LinearLayout.LayoutParams(-1, ViewGroup.LayoutParams.WRAP_CONTENT)
        }
        fun label(value: String, size: Float) = TextView(activity).apply {
            text = value; textSize = size
            setTextColor(activity.getColor(R.color.elon_text_primary))
            setPadding(0, dp(8), 0, dp(8))
        }
        section.addView(label("用 ESK 支付 AI 服务", 18f))
        section.addView(label("充值获得 ESK，使用 AI 时按模型和实际用量计费。每笔消费都可以查看明细。", 14f))
        section.addView(label("目前可查看余额与记录，ESK 充值和 AI 支付暂未开放。", 14f))
        section.addView(label("查看我的 AI 账户  ›\nESK 余额 · 到账记录 · AI 用量 · 消费账单", 16f).apply {
            minimumHeight = dp(64); isFocusable = true
            setOnClickListener { activity.startActivity(Intent(activity, EskComputeActivity::class.java)) }
        })
        host.addView(section, 0)
    }
}
