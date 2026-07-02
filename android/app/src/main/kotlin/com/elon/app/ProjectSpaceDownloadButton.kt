package com.elon.app

import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.PopupWindow
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
                text = "安装应用"
                textSize = 16f
                gravity = Gravity.CENTER
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#D6D6D6"))
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener {
                    val url = cleanProjectApkUrl(apkUrl)
                    if (url == null) {
                        Toast.makeText(activity, "暂无可安装 APK", Toast.LENGTH_SHORT).show()
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

internal fun projectSpaceAnnouncementMenuButton(
    activity: AppCompatActivity,
    dp: (Int) -> Int,
    selectableForeground: () -> android.graphics.drawable.Drawable?,
    onOpenDocuments: () -> Unit,
    apkActionLabel: () -> String = { "安装应用" },
    onDownloadApk: () -> Unit
): FrameLayout {
    var popup: PopupWindow? = null
    return FrameLayout(activity).apply {
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "项目空间快捷菜单"
        setOnClickListener { anchor ->
            popup = showProjectSpaceActionPopup(
                anchor = anchor,
                previousPopup = popup,
                activity = activity,
                dp = dp,
                selectableForeground = selectableForeground,
                onOpenDocuments = onOpenDocuments,
                apkActionLabel = apkActionLabel,
                onDownloadApk = onDownloadApk
            )
        }
        addView(ImageView(activity).apply {
            setImageResource(R.drawable.ic_project_documents_menu)
            setColorFilter(Color.parseColor("#D6D6D6"))
            scaleType = ImageView.ScaleType.CENTER
        }, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT
        ))
    }
}

private fun showProjectSpaceActionPopup(
    anchor: View,
    previousPopup: PopupWindow?,
    activity: AppCompatActivity,
    dp: (Int) -> Int,
    selectableForeground: () -> android.graphics.drawable.Drawable?,
    onOpenDocuments: () -> Unit,
    apkActionLabel: () -> String,
    onDownloadApk: () -> Unit
): PopupWindow {
    previousPopup?.dismiss()
    val popupWidth = dp(128)
    val arrowHeight = dp(12)
    val panelColor = Color.parseColor("#BDBDBD")
    val root = FrameLayout(activity).apply {
        layoutParams = ViewGroup.LayoutParams(popupWidth, ViewGroup.LayoutParams.WRAP_CONTENT)
        alpha = 0f
        scaleX = 0.98f
        scaleY = 0.98f
    }
    lateinit var popup: PopupWindow
    val panel = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(0, dp(8), 0, dp(8))
        background = GradientDrawable().apply {
            cornerRadius = dp(16).toFloat()
            setColor(panelColor)
        }
        addView(projectSpaceMenuRow(activity, dp, selectableForeground, "项目文档", { popup.dismiss() }, onOpenDocuments))
        addView(projectSpaceMenuRow(activity, dp, selectableForeground, apkActionLabel(), { popup.dismiss() }, onDownloadApk))
    }
    root.addView(panel, FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.MATCH_PARENT,
        FrameLayout.LayoutParams.WRAP_CONTENT
    ).apply {
        topMargin = arrowHeight
    })
    root.addView(projectSpaceMenuArrow(activity, panelColor), FrameLayout.LayoutParams(dp(24), arrowHeight).apply {
        gravity = Gravity.TOP or Gravity.END
        rightMargin = dp(26)
    })
    popup = PopupWindow(root, popupWidth, ViewGroup.LayoutParams.WRAP_CONTENT, true).apply {
        isOutsideTouchable = true
        elevation = dp(8).toFloat()
        setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        showAsDropDown(anchor, anchor.width - popupWidth + dp(14), -dp(4))
    }
    root.pivotX = (popupWidth - dp(34)).toFloat()
    root.pivotY = 0f
    root.animate()
        .alpha(1f)
        .scaleX(1f)
        .scaleY(1f)
        .setDuration(120L)
        .start()
    return popup
}

private fun projectSpaceMenuRow(
    activity: AppCompatActivity,
    dp: (Int) -> Int,
    selectableForeground: () -> android.graphics.drawable.Drawable?,
    label: String,
    dismissPopup: () -> Unit,
    onClick: () -> Unit
): TextView {
    return TextView(activity).apply {
        text = label
        textSize = 17f
        gravity = Gravity.CENTER
        includeFontPadding = false
        setTextColor(Color.parseColor("#3F4146"))
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener {
            dismissPopup()
            onClick()
        }
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(46)
        )
    }
}

private fun projectSpaceMenuArrow(activity: AppCompatActivity, color: Int): View {
    val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        this.color = color
        style = Paint.Style.FILL
    }
    return object : View(activity) {
        override fun onDraw(canvas: Canvas) {
            super.onDraw(canvas)
            val path = Path().apply {
                moveTo(width / 2f, 0f)
                lineTo(width.toFloat(), height.toFloat())
                lineTo(0f, height.toFloat())
                close()
            }
            canvas.drawPath(path, paint)
        }
    }
}

internal fun openProjectApkDownload(
    activity: AppCompatActivity,
    apkUrl: String?,
    projectId: String? = null,
    projectName: String? = null,
    apkIdentity: String? = null,
    apkUpdatedAt: String? = null
) {
    if (!projectId.isNullOrBlank() && !projectName.isNullOrBlank() &&
        !hasProjectApkUpdate(activity, projectId, projectName, apkIdentity, apkUpdatedAt) &&
        openInstalledProjectApp(activity, projectId, projectName)
    ) {
        return
    }
    val url = cleanProjectApkUrl(apkUrl)
    if (url == null) {
        Toast.makeText(activity, "暂无可安装 APK", Toast.LENGTH_SHORT).show()
        return
    }
    val token = AuthManager.token(activity)?.trim().orEmpty()
    if (token.isBlank()) {
        Toast.makeText(activity, "请先登录后安装 APK", Toast.LENGTH_SHORT).show()
        return
    }
    openProjectApkInstall(activity, url, token, projectId, projectName, apkIdentity, apkUpdatedAt)
}
