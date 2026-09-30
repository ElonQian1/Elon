package com.elon.app

import android.graphics.Typeface
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import android.content.Context
import org.json.JSONObject

/** The same manifest sections used by the desktop project introduction. */
internal data class ProjectIntroductionContent(
    val tagline: String?,
    val summary: String?,
    val highlights: List<String>,
    val targetUsers: List<String>,
    val updates: List<String>,
    val requirements: List<String>,
    val privacy: List<String>
)

internal fun parseProjectIntroduction(landing: JSONObject?): ProjectIntroductionContent? {
    if (landing == null) return null
    fun text(key: String) = landing.optString(key).trim().takeIf { it.isNotBlank() && it != "null" }
    fun items(key: String): List<String> {
        val values = landing.optJSONArray(key) ?: return emptyList()
        return (0 until minOf(values.length(), 12)).mapNotNull {
            values.optString(it).trim().takeIf { value -> value.isNotBlank() && value != "null" }
        }
    }
    return ProjectIntroductionContent(text("tagline"), text("summary") ?: text("description"),
        items("highlights"), items("target_users"), items("recent_updates"),
        items("system_requirements"), items("privacy_notes"))
}

internal class ProjectIntroductionView(
    private val activity: Context,
    private val dp: (Int) -> Int,
    private val openMembers: () -> Unit
) {
    private val colors = MobileColors(activity)

    fun render(space: ProjectSpace): LinearLayout = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(colors.surface)
        setPadding(dp(24), dp(20), dp(24), dp(20))
        val content = space.introduction
        content?.tagline?.let { addView(label(it, heading = true)) }
        content?.summary?.let { addView(label(it)) }
        addView(label("团队协作", heading = true))
        val joining = when (space.project.joinMode) {
            PROJECT_JOIN_MODE_OPEN -> "加入同一个项目，按角色权限共同参与。"
            PROJECT_JOIN_MODE_APPROVAL -> "申请通过后加入项目，按角色权限共同参与。"
            PROJECT_JOIN_MODE_READONLY -> "当前项目提供只读体验；开发操作以授权为准。"
            else -> "接受项目邀请后，在同一个项目空间按角色权限共同参与。"
        }
        addView(label(joining))
        val development = space.channels.any { it.kind == "ai_development" }
        addView(label(if (development)
            "围绕需求共同讨论、参与 AI 功能开发，查看项目资料和交付记录。管理、发布和节点执行分别受权限与环境约束。"
            else "共同交流需求，查看项目资料与交付记录。可操作范围以当前成员权限为准。"))
        addView(label("查看项目成员 ›").apply {
            setTextColor(colors.primary)
            minimumHeight = dp(48)
            isClickable = true
            isFocusable = true
            setOnClickListener { openMembers() }
        })
        if (content != null) {
            if (content.highlights.isNotEmpty()) {
                addView(label("核心能力", heading = true))
                content.highlights.take(3).forEachIndexed { index, value -> addView(label("${index + 1}. $value")) }
                if (content.highlights.size > 3) addView(expandable("查看全部 ${content.highlights.size} 项能力", content.highlights.drop(3)))
            }
            if (content.targetUsers.isNotEmpty()) addView(expandable("适合谁使用", content.targetUsers))
            if (content.updates.isNotEmpty()) addView(expandable("最近更新", content.updates))
            if (content.requirements.isNotEmpty()) addView(expandable("使用条件与环境", content.requirements))
            if (content.privacy.isNotEmpty()) addView(expandable("隐私、权限与使用边界", content.privacy))
        }
    }

    private fun expandable(title: String, items: List<String>): LinearLayout = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            visibility = View.GONE
            items.forEach { addView(label("• $it")) }
        }
        val toggle = label("$title ＋").apply {
            minimumHeight = dp(48)
            setTextColor(colors.primary)
            isClickable = true
            isFocusable = true
            contentDescription = "$title，已折叠"
            setOnClickListener {
                val expanded = body.visibility != View.VISIBLE
                body.visibility = if (expanded) View.VISIBLE else View.GONE
                text = "$title ${if (expanded) "－" else "＋"}"
                contentDescription = "$title，${if (expanded) "已展开" else "已折叠"}"
            }
        }
        addView(toggle)
        addView(body)
    }

    private fun label(value: String, heading: Boolean = false) = TextView(activity).apply {
        text = value
        textSize = if (heading) 20f else 16f
        setTextColor(if (heading) colors.text else colors.muted)
        if (heading) setTypeface(typeface, Typeface.BOLD)
        setLineSpacing(dp(3).toFloat(), 1f)
        setPadding(0, dp(if (heading) 16 else 8), 0, dp(8))
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT)
    }
}
