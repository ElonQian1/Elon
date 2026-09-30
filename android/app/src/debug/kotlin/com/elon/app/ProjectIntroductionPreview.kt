package com.elon.app

import android.content.Context
import android.view.ContextThemeWrapper
import android.view.View
import android.widget.ScrollView
import android.widget.LinearLayout
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode
import org.json.JSONObject

/** Offline production introduction; never connects to a project or changes membership. */
internal fun projectIntroductionPreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.project.introduction"
    override val supportedScenarios = setOf("main", "child", "empty", "readonly")

    override fun createView(context: Context, request: UiRuntimePreviewRequest): View {
        val themed = ContextThemeWrapper(context, R.style.Theme_ElonApp)
        val content = if (request.scenario == "empty") null else parseProjectIntroduction(JSONObject().apply {
            put("tagline", if (request.scenario == "main") "和团队一起，把想法做成应用" else "子项目介绍离线示例")
            put("summary", "离线布局示例：项目成员按角色权限共同讨论需求、参与功能开发、查看进度和交付结果。")
            put("highlights", org.json.JSONArray(listOf(
                "邀请成员共同开发：在同一项目空间按角色权限参与。",
                "手机与电脑接力：项目、频道和交付记录集中管理。",
                "接入已有项目与已授权开发环境，持续迭代功能。",
                "项目规则、文档和上下文帮助 AI 理解当前任务。",
                "追踪开发与交付，查看验证结果和实际产物。"
            )))
            put("target_users", org.json.JSONArray(listOf("个人开发者", "共同参与项目的团队成员")))
            put("privacy_notes", org.json.JSONArray(listOf("本页仅用于离线布局验收，不代表真实任务完成。")))
        })
        val space = ProjectSpace(
            project = ProjectSpaceSummary("offline-preview", "项目介绍离线验收", null, if (request.scenario == "readonly") "visitor" else "member",
                if (request.scenario == "readonly") PROJECT_JOIN_MODE_READONLY else PROJECT_JOIN_MODE_INVITE,
                3, null, ""),
            channels = if (request.scenario == "empty") emptyList() else listOf(
                ProjectChannel("preview-dev", "offline-preview", "AI 开发", "ai_development", 0, null, null, 0)
            ),
            members = emptyList(), latestApkUrl = null, latestApkIdentity = null,
            latestApkUpdatedAt = null, galleryImages = emptyList(), landingPreviewImages = emptyList(), introduction = content
        )
        val dp: (Int) -> Int = { (it * themed.resources.displayMetrics.density).toInt() }
        return ScrollView(themed).apply {
            setBackgroundColor(MobileColors(themed).surface)
            addView(LinearLayout(themed).apply {
                orientation = LinearLayout.VERTICAL
                addView(ProjectSpaceOverviewView(themed, dp, {}, {}, {}).render(space))
                addView(ProjectIntroductionView(themed, dp, {}, {}).render(space))
            })
        }.uiNode("project.introduction.root")
    }
}
