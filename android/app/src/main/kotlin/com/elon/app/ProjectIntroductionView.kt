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
    val privacy: List<String>,
    val description: String? = null,
    val resources: List<ProjectIntroductionLink> = emptyList()
)

internal data class ProjectIntroductionLink(val label: String, val url: String)

internal fun parseProjectIntroduction(landing: JSONObject?): ProjectIntroductionContent? {
    if (landing == null) return null
    fun text(key: String) = landing.optString(key).trim().takeIf { it.isNotBlank() && it != "null" }
    fun items(key: String): List<String> {
        val values = landing.optJSONArray(key) ?: return emptyList()
        return (0 until minOf(values.length(), 12)).mapNotNull {
            values.optString(it).trim().takeIf { value -> value.isNotBlank() && value != "null" }
        }
    }
    val links = mutableListOf<ProjectIntroductionLink>()
    fun link(label: String, url: String) {
        if ((url.startsWith("https://") || url.startsWith("http://") || (url.startsWith("/") && !url.startsWith("//")))
            && links.none { it.url == url }) links.add(ProjectIntroductionLink(label, url))
    }
    text("custom_landing_url")?.let { link("完整项目介绍", it) }
    text("web_url")?.let { link("打开网页端", it) }
    landing.optJSONArray("resources")?.let { values ->
        for (index in 0 until minOf(values.length(), 12)) values.optJSONObject(index)?.let {
            link(it.optString("label", "项目资料"), it.optString("url"))
        }
    }
    landing.optJSONArray("downloads")?.let { values ->
        for (index in 0 until minOf(values.length(), 12)) values.optJSONObject(index)?.let {
            if (it.optString("status") in listOf("available", "external"))
                link(it.optString("label", "版本入口"), it.optString("url"))
        }
    }
    return ProjectIntroductionContent(text("tagline"), text("summary") ?: text("description"),
        items("highlights"), items("target_users"), items("recent_updates"),
        items("system_requirements"), items("privacy_notes"), text("description"), links)
}

internal class ProjectIntroductionView(
    private val activity: Context,
    private val dp: (Int) -> Int,
    private val openMembers: () -> Unit,
    private val openChannel: ((ProjectChannel) -> Unit)? = null,
    private val openDescription: (() -> Unit)? = null
) {
    private val colors = MobileColors(activity)

    fun render(space: ProjectSpace): LinearLayout = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(colors.surface)
        setPadding(dp(24), dp(20), dp(24), dp(20))
        val content = space.introduction
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
        addView(label(if (canManageProjectMembers(space.project.role)) "管理与邀请成员 ›" else "查看团队成员 ›").apply {
            setTextColor(colors.primary)
            minimumHeight = dp(48)
            isClickable = true
            isFocusable = true
            setOnClickListener { openMembers() }
        })
        openChannel?.let { select ->
            space.channels.firstOrNull { it.kind == "ai_development" }?.let { channel ->
                addView(action("进入 AI 开发") { select(channel) })
            }
            space.channels.firstOrNull { it.kind == "discussion" || it.kind == "chat" }?.let { channel ->
                addView(action("参与项目讨论") { select(channel) })
            }
            space.channels.firstOrNull { it.kind == "builds" }?.let { channel ->
                addView(action("查看构建与交付") { select(channel) })
            }
        }
        addView(label("加入同一个项目 → 围绕需求共同参与 → 查看进度与交付"))
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
            content.description?.takeIf { it != content.summary }?.let { addView(expandable("完整项目介绍", listOf(it))) }
        }
        openDescription?.let { open -> addView(action(if (canEditProjectDescription(space.project.role)) "编辑项目简介" else "查看项目简介", open)) }
    }

    private fun action(title: String, onClick: () -> Unit) = label("$title ›").apply {
        setTextColor(colors.primary)
        minimumHeight = dp(48)
        isClickable = true
        isFocusable = true
        setOnClickListener { onClick() }
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
