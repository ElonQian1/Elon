package com.elon.app

import android.content.Context
import android.view.ContextThemeWrapper
import android.view.LayoutInflater
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode

/** Production components with offline fixtures. This does not exercise account or build services. */
internal fun mobileDesignV2PreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.mobile.design_v2"
    override val supportedScenarios = setOf("projects", "empty", "chat_result", "login", "register", "account_security")

    override fun createView(context: Context, request: UiRuntimePreviewRequest): View {
        val themed = ContextThemeWrapper(context, R.style.Theme_ElonApp)
        if (request.scenario in setOf("login", "register", "account_security")) {
            return mobileAccountPreview(themed, request.scenario).uiNode("mobile.design_v2.${request.scenario}")
        }
        val content = LinearLayout(themed).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(themed.elonColor(R.color.mobile_surface))
        }
        val title = TextView(themed).apply {
            text = if (request.scenario == "chat_result") "项目对话" else "项目"
            textSize = 22f
            setTextColor(themed.elonColor(R.color.mobile_on_surface))
            setPadding(dp(themed, 16), dp(themed, 16), dp(themed, 16), dp(themed, 16))
        }
        content.addView(title)
        if (request.scenario == "chat_result") {
            addMessage(content, R.layout.item_message_user, "请整理项目需求，并检查这次构建结果。")
            addMessage(content, R.layout.item_message_ai,
                "已整理需求。\n\n构建结果：成功\n验证结果：请查看详细记录。\n\n这是离线布局示例，不代表实际任务已完成。")
        } else {
            val list = LinearLayout(themed).apply { orientation = LinearLayout.VERTICAL }
            var personal = true
            var joint = false
            val fixtures = if (request.scenario == "empty") emptyList() else listOf(
                AppProject("preview-work", "移动协作工具", "整理需求、跟踪任务与查看交付结果", 2L,
                    ownerAccount = "演示账户", memberCount = 3),
                AppProject("preview-long", "一个较长的项目名称，用于检查换行和大字体", "长说明应保持可读，项目操作与内容层级清楚。", 1L,
                    ownerAccount = "一个很长的演示账户名称", memberCount = 128),
                AppProject("preview-joint", "联合项目示例", "共同查看项目进度", 0L,
                    isJointProject = true, ownerAccount = "协作演示账户", memberCount = 8)
            )
            ProjectManagementHomeView(
                activity = themed, container = list, segmentContainer = null,
                projects = { fixtures }, plazaProjects = { emptyList() },
                personalProjectsExpanded = { personal }, jointProjectsExpanded = { joint },
                setPersonalProjectsExpanded = { personal = it }, setJointProjectsExpanded = { joint = it },
                formatTime = { "刚刚" }, openProject = { title.text = fixtures[it].title },
                openProjectConversations = {}, isProjectWorking = { false }, showProjectActions = { _, _ -> },
                showCreateProjectDialog = { title.text = "创建项目 · 离线示例" }, showProjectPlaza = {},
                dp = { dp(themed, it) }, selectableForeground = {
                    val value = android.util.TypedValue()
                    themed.theme.resolveAttribute(android.R.attr.selectableItemBackground, value, true)
                    themed.getDrawable(value.resourceId)
                }
            ).render()
            content.addView(list)
        }
        return ScrollView(themed).apply {
            isFillViewport = true
            setBackgroundColor(themed.elonColor(R.color.mobile_surface))
            addView(content)
        }.uiNode("mobile.design_v2.${request.scenario}")
    }

    private fun addMessage(parent: LinearLayout, layout: Int, message: String) {
        val row = LayoutInflater.from(parent.context).inflate(layout, parent, false)
        row.findViewById<TextView>(R.id.messageText).text = message
        parent.addView(row)
    }

    private fun dp(context: Context, value: Int) = (value * context.resources.displayMetrics.density).toInt()
}
