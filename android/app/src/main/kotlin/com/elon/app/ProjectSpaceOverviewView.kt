package com.elon.app

import android.content.Context
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.ImageView
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory

/** Adaptive overview inside the existing View host; navigation uses its real controller. */
internal class ProjectSpaceOverviewView(
    private val context: Context,
    private val dp: (Int) -> Int,
    private val openChannel: (ProjectChannel) -> Unit,
    private val openMembers: () -> Unit,
    private val join: () -> Unit
) {
    private val colors = MobileColors(context)

    fun render(space: ProjectSpace): LinearLayout = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(colors.surface)
        setPadding(dp(24), dp(20), dp(24), dp(8))
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            val bitmap = UserProfileStore.decodeAvatar(space.project.iconDataUrl)
            val icon = if (bitmap != null) ImageView(context).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply { cornerRadius = dp(12).toFloat() })
            } else label(space.project.name.firstOrNull()?.toString() ?: "项", 24, bold = true).apply {
                gravity = Gravity.CENTER
                background = GradientDrawable().apply { setColor(this@ProjectSpaceOverviewView.colors.container); cornerRadius = dp(12).toFloat() }
            }
            icon.importantForAccessibility = android.view.View.IMPORTANT_FOR_ACCESSIBILITY_NO
            addView(icon, LinearLayout.LayoutParams(dp(56), dp(56)).apply { marginEnd = dp(16) })
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                addView(label("项目空间 · ${roleLabel(space.project.role)}", 14, muted = true))
                addView(label(space.project.name.ifBlank { "项目空间" }, 26, bold = true))
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        })
        space.introduction?.tagline?.let { addView(label(it, 20, bold = true)) }
        (space.introduction?.summary ?: space.project.description)?.takeIf { it.isNotBlank() }
            ?.let { addView(label(it, 16, muted = true)) }
        addView(label("${space.project.memberCount} 位成员 · ${space.channels.size} 个频道", 14, muted = true))
        val development = space.channels.firstOrNull { it.kind == "ai_development" }
        val discussion = space.channels.firstOrNull { it.kind == "discussion" || it.kind == "chat" }
        val builds = space.channels.firstOrNull { it.kind == "builds" }
        when {
            isProjectSpaceVisitor(space.project.role) -> {
                val title = when (space.project.joinMode) {
                    PROJECT_JOIN_MODE_OPEN -> "加入项目"
                    PROJECT_JOIN_MODE_APPROVAL -> "申请加入"
                    PROJECT_JOIN_MODE_READONLY -> "查看团队成员"
                    else -> "查看加入方式"
                }
                addView(action(title, primary = true) {
                    if (space.project.joinMode == PROJECT_JOIN_MODE_READONLY) openMembers() else join()
                })
            }
            development != null -> addView(action(if (space.project.role in listOf("viewer", "observer")) "查看开发进度" else "继续开发", primary = true) { openChannel(development) })
            discussion != null -> addView(action("参与项目讨论", primary = true) { openChannel(discussion) })
            builds != null -> addView(action("查看构建与交付", primary = true) { openChannel(builds) })
            else -> addView(action("查看团队成员", primary = true, onClick = openMembers))
        }
        addView(label("手机与 Windows 使用同一个项目空间，按权限参与讨论、开发和查看交付。", 14, muted = true))
    }

    private fun label(value: String, size: Int, bold: Boolean = false, muted: Boolean = false) = TextView(context).apply {
        text = value
        textSize = size.toFloat()
        setTextColor(if (muted) colors.muted else colors.text)
        if (bold) setTypeface(typeface, Typeface.BOLD)
        setLineSpacing(dp(3).toFloat(), 1f)
        setPadding(0, dp(6), 0, dp(6))
    }

    private fun action(title: String, primary: Boolean, onClick: () -> Unit) = TextView(context).apply {
        text = title
        textSize = 16f
        gravity = Gravity.CENTER
        minimumHeight = dp(48)
        setPadding(dp(16), dp(12), dp(16), dp(12))
        setTextColor(if (primary) colors.onPrimary else colors.primary)
        background = GradientDrawable().apply {
            setColor(if (primary) this@ProjectSpaceOverviewView.colors.primary else this@ProjectSpaceOverviewView.colors.container)
            cornerRadius = dp(12).toFloat()
        }
        isClickable = true
        isFocusable = true
        setOnClickListener { onClick() }
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply {
            topMargin = dp(12)
        }
    }

    private fun roleLabel(role: String): String = when (role.lowercase()) {
        "owner" -> "项目所有者"
        "admin" -> "管理员"
        "editor" -> "开发成员"
        "viewer", "observer" -> "只读成员"
        "visitor", "guest", "" -> "访客"
        else -> "项目成员"
    }
}
