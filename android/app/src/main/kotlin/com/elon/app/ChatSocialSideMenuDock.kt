package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView

internal fun createSocialSidebarFilterDock(
    context: Context,
    selectedFilter: SocialSidebarContentType,
    onFilterSelected: (SocialSidebarContentType) -> Unit,
    openSettings: () -> Unit,
    dp: (Int) -> Int,
    selectableForeground: () -> Drawable?
): LinearLayout = LinearLayout(context).apply {
    orientation = LinearLayout.VERTICAL
    setPadding(0, dp(8), 0, 0)
    addView(LinearLayout(context).apply {
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        socialSidebarFilterDefinitions().forEach { (label, type) ->
            addView(
                socialSidebarFilterText(
                    context = context,
                    label = label,
                    type = type,
                    selectedFilter = selectedFilter,
                    onFilterSelected = onFilterSelected,
                    selectableForeground = selectableForeground
                ),
                LinearLayout.LayoutParams(
                    0,
                    dp(48),
                    if (type == SocialSidebarContentType.MEDIA) 1.65f else 1f
                )
            )
        }
    }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(48)))
    addView(TextView(context).apply {
        gravity = Gravity.CENTER_VERTICAL or Gravity.START
        includeFontPadding = false
        text = "设置"
        textSize = 15f
        setSingleLine(true)
        setTextColor(Color.parseColor("#D9D9D9"))
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "侧栏设置"
        setOnClickListener { openSettings() }
    }, LinearLayout.LayoutParams(dp(96), dp(48)))
}

private fun socialSidebarFilterText(
    context: Context,
    label: String,
    type: SocialSidebarContentType,
    selectedFilter: SocialSidebarContentType,
    onFilterSelected: (SocialSidebarContentType) -> Unit,
    selectableForeground: () -> Drawable?
) = TextView(context).apply {
    gravity = Gravity.CENTER
    includeFontPadding = false
    text = label
    textSize = 13f
    setSingleLine(true)
    setTextColor(Color.parseColor(if (selectedFilter == type) "#4F9DFF" else "#D9D9D9"))
    isClickable = true
    foreground = selectableForeground()
    contentDescription = "$label${if (selectedFilter == type) "，已选中" else ""}"
    setOnClickListener {
        onFilterSelected(
            if (selectedFilter == type) SocialSidebarContentType.ALL else type
        )
    }
}

private fun socialSidebarFilterDefinitions() = listOf(
    "图片与视频" to SocialSidebarContentType.MEDIA,
    "文本" to SocialSidebarContentType.TEXT,
    "链接" to SocialSidebarContentType.LINK,
    "笔记" to SocialSidebarContentType.NOTE,
    "文件" to SocialSidebarContentType.FILE
)
