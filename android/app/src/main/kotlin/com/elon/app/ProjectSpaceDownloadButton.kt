package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

internal fun projectSpaceDownloadButton(
    activity: AppCompatActivity,
    apkUrl: String,
    dp: (Int) -> Int,
    selectableForeground: () -> android.graphics.drawable.Drawable?
): TextView {
    return TextView(activity).apply {
        text = "下载最新 APK"
        textSize = 15f
        gravity = Gravity.CENTER
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(Color.parseColor("#101010"))
        background = GradientDrawable().apply {
            cornerRadius = dp(6).toFloat()
            setColor(Color.parseColor("#C8C8C8"))
        }
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { openProjectApkDownload(activity, apkUrl) }
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(42)
        ).apply { topMargin = dp(10) }
    }
}

private fun openProjectApkDownload(activity: AppCompatActivity, apkUrl: String) {
    val token = AuthManager.token(activity)?.trim().orEmpty()
    if (token.isBlank()) {
        Toast.makeText(activity, "请先登录后下载 APK", Toast.LENGTH_SHORT).show()
        return
    }
    openProjectApkInstall(activity, apkUrl, token)
}
