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
            text = "ESK 余额与记录  ›\n查看已审核到账的 ESK 和余额变动"
            textSize = 16f
            setTextColor(activity.getColor(R.color.elon_text_primary))
            setPadding(dp(16), dp(14), dp(16), dp(14))
            minimumHeight = dp(48)
            isFocusable = true
            setOnClickListener { activity.startActivity(Intent(activity, EskPlatformAssetsActivity::class.java)) }
        })
        host.addView(TextView(activity).apply {
            tag = ENTRY_TAG
            text = "到账与审核记录  ›\n查看每笔 ESK 的审核结果"
            textSize = 16f
            setTextColor(activity.getColor(R.color.elon_text_primary))
            setPadding(dp(16), dp(16), dp(16), dp(16))
            setLineSpacing(dp(6).toFloat(), 1f)
            minHeight = dp(56)
            isSaveEnabled = false
            isSaveFromParentEnabled = false
            layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(12) }
            contentDescription = "查看 ESK 到账与审核记录"
            isFocusable = true
            setOnClickListener { activity.startActivity(Intent(activity, EskPlatformHistoryActivity::class.java)) }
        })
    }
}
