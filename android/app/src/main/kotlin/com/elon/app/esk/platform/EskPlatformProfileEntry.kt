package com.elon.app.esk.platform

import android.content.Intent
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R
import com.elon.app.databinding.ActivityMainBinding

/** Static entry: no token read, balance copy, or asynchronous callback on the personal page. */
internal object EskPlatformProfileEntry {
    private const val ENTRY_TAG = "esk-platform-profile-entry"

    fun attach(activity: AppCompatActivity, binding: ActivityMainBinding) {
        val host = binding.profileEskAssetContainer
        if (host.findViewWithTag<View>(ENTRY_TAG) != null) return
        val density = activity.resources.displayMetrics.density
        fun dp(value: Int) = (value * density).toInt()
        host.addView(TextView(activity).apply {
            tag = ENTRY_TAG
            text = "正式 ESK 平台登记  ›\n查看经审核数量与流水 · 尚未上链"
            textSize = 16f
            setTextColor(activity.getColor(R.color.elon_text_primary))
            setPadding(dp(16), dp(16), dp(16), dp(16))
            setLineSpacing(dp(6).toFloat(), 1f)
            minHeight = dp(56)
            isSaveEnabled = false
            isSaveFromParentEnabled = false
            layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(12) }
            contentDescription = "查看正式 ESK 平台登记，与 Paper 模拟余额分开"
            isFocusable = true
            setOnClickListener { activity.startActivity(Intent(activity, EskPlatformAssetsActivity::class.java)) }
        })
    }
}
