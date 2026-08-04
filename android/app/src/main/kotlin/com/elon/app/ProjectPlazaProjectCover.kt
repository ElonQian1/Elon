package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.util.TypedValue
import android.view.Gravity
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal fun projectPlazaProjectCover(
    activity: AppCompatActivity,
    project: StoreProject,
    sizePx: Int,
    radiusPx: Float,
    fallbackTextSp: Float
): FrameLayout = FrameLayout(activity).apply {
    background = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        setColor(Color.WHITE)
        cornerRadius = radiusPx
    }
    clipToOutline = true
    contentDescription = "${project.displayTitle()}项目封面"
    addView(TextView(activity).apply {
        text = project.displayTitle().trim().firstOrNull()?.toString() ?: "项"
        gravity = Gravity.CENTER
        includeFontPadding = false
        setTextColor(Color.BLACK)
        setTextSize(TypedValue.COMPLEX_UNIT_SP, fallbackTextSp)
        typeface = Typeface.DEFAULT_BOLD
        contentDescription = null
    }, FrameLayout.LayoutParams(sizePx, sizePx, Gravity.CENTER))
    UserProfileStore.decodeAvatar(project.iconDataUrl)?.let { bitmap ->
        addView(ImageView(activity).apply {
            setImageBitmap(bitmap)
            scaleType = ImageView.ScaleType.CENTER_CROP
            contentDescription = null
        }, FrameLayout.LayoutParams(sizePx, sizePx, Gravity.CENTER))
    }
}
