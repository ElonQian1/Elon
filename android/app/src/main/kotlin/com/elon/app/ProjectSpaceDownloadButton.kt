package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

internal fun projectSpaceQuickActions(
    activity: AppCompatActivity,
    apkUrl: String?,
    dp: (Int) -> Int,
    selectableForeground: () -> android.graphics.drawable.Drawable?,
    onOpenDocuments: () -> Unit
): LinearLayout {
    return LinearLayout(activity).apply {
        gravity = Gravity.CENTER
        setPadding(0, dp(70), 0, dp(48))
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        )
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            background = GradientDrawable().apply {
                cornerRadius = dp(28).toFloat()
                setColor(Color.TRANSPARENT)
                setStroke(dp(1), Color.parseColor("#A8A8A8"))
            }
            addView(TextView(activity).apply {
                text = "下载APK"
                textSize = 18f
                gravity = Gravity.CENTER
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#D6D6D6"))
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener {
                    val url = apkUrl?.trim().orEmpty()
                    if (url.isBlank()) {
                        Toast.makeText(activity, "暂无可下载 APK", Toast.LENGTH_SHORT).show()
                    } else {
                        openProjectApkDownload(activity, url)
                    }
                }
            }, LinearLayout.LayoutParams(dp(122), LinearLayout.LayoutParams.MATCH_PARENT))
            addView(View(activity).apply {
                setBackgroundColor(Color.parseColor("#A8A8A8"))
            }, LinearLayout.LayoutParams(1, dp(30)))
            addView(FrameLayout(activity).apply {
                isClickable = true
                foreground = selectableForeground()
                contentDescription = "项目文档"
                setOnClickListener { onOpenDocuments() }
                addView(ImageView(activity).apply {
                    setImageResource(R.drawable.ic_project_documents_menu)
                    setColorFilter(Color.parseColor("#D6D6D6"))
                    scaleType = ImageView.ScaleType.CENTER
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }, LinearLayout.LayoutParams(dp(56), LinearLayout.LayoutParams.MATCH_PARENT))
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            dp(56)
        ))
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
