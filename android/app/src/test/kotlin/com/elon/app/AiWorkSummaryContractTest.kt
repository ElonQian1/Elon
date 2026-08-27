package com.elon.app

import java.io.File
import java.util.Calendar
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AiWorkSummaryContractTest {
    private val root = generateSequence(File(System.getProperty("user.dir")).canonicalFile) { it.parentFile }
        .first { File(it, "android/app/src/main/AndroidManifest.xml").isFile }

    @Test
    fun androidSummaryKeepsRequiredInformationAndActions() {
        val source = root.resolve("android/app/src/main/kotlin/com/elon/app/AiWorkSummaryActivity.kt").readText()

        listOf("AI 建议", "交给 AI 处理", "有新进展", "待确认", "generateWorkSummary").forEach {
            assertTrue("Android 工作摘要缺少：$it", source.contains(it))
        }
        assertTrue(source.contains("ic_work_summary_calendar"))
        assertTrue(source.contains("contentDescription = \"选择摘要日期\""))
        assertTrue("次要摘要区域必须支持折叠", source.contains("content.visibility"))
        assertTrue("卡片必须使用设计稿的 30dp 内容对齐线", source.contains("setPadding(dp(30), dp(21), dp(30), dp(21))"))
        assertTrue("指标卡间距必须使用设计稿的 16dp 节奏", source.contains("weighted(dp(16))"))
        assertTrue("分区数量胶囊必须使用设计稿 20dp 宽度", source.contains("LinearLayout.LayoutParams(dp(20), dp(21))"))
        assertTrue("日期选择组必须按整屏居中，日历独立贴右", source.contains("FrameLayout.LayoutParams(dp(96), dp(48), Gravity.CENTER)"))
        assertTrue("问候文字必须与头像保留设计稿横向留白，禁止负边距", source.contains("marginStart = dp(106)") && !source.contains("marginStart = dp(-7)"))
        assertTrue("AI 头像必须放大到设计容器而不是保留 nodpi 原始像素", source.contains("scaleType = ImageView.ScaleType.FIT_CENTER"))
        assertTrue("指标卡数字和标题必须真正居中", source.contains("label(number, 16f, color, regular).apply { gravity = Gravity.CENTER }") && source.contains("label(caption, 13f, \"#E5E8E7\", regular).apply"))
        assertTrue("进展行项目图标必须使用设计稿 29dp 起点", source.contains("setPadding(dp(29), dp(14), dp(4), dp(14))"))
        assertTrue("折叠标题与查看全部必须使用设计稿边界", source.contains("setPadding(dp(13), 0, dp(9), 0)") && source.contains("setPadding(0, 0, dp(22), 0)"))
        assertTrue("建议卡片必须按内容自然增长并保留设计稿最小高度", source.contains("LinearLayout.LayoutParams(MATCH, WRAP)") && source.contains("minimumHeight = dp(if (item.highPriority) 268 else 258)") && !source.contains("cardHeightDp"))
        assertTrue("项目标题和进展说明不得设置最大行数", !source.contains("maxLines ="))
        assertTrue("页面箭头必须复用 APP 图标资源", source.contains("R.drawable.ic_input_model_chevron") && source.contains("R.drawable.ic_project_space_chevron_right") && !source.contains("label(\"›\"") && !source.contains("\"⌃\"") && !source.contains("\"⌄\""))
        assertTrue("折叠箭头必须保持 14dp 原始图形并居中，不能铺满点击区", source.contains("scaleType = ImageView.ScaleType.CENTER") && source.contains("setColorFilter(Color.parseColor(\"#E1E5E4\"))"))
        assertTrue("进展项必须显示真实更新时间", source.contains("formatDate(item.updatedAt)"))
        assertTrue("日期按钮必须打开选择器", source.contains("DatePickerDialog"))
        assertTrue("项目操作必须进入真实项目入口", source.contains("EXTRA_OPEN_WORK_SUMMARY_PROJECT_TITLE"))
        assertTrue("AI 操作必须进入真实发送链路", source.contains("EXTRA_WORK_SUMMARY_AI_PROMPT"))
    }

    @Test
    fun homeEntryAndWebMirrorStayConnected() {
        val header = root.resolve("android/app/src/main/kotlin/com/elon/app/HomeConversationHeaderView.kt").readText()
        val manifest = root.resolve("android/app/src/main/AndroidManifest.xml").readText()
        val web = root.resolve("server/src/assets/web_page.html").readText()
        val createActions = root.resolve("android/app/src/main/kotlin/com/elon/app/MainCreateActions.kt").readText()

        assertTrue(header.contains("onOpenSummary()"))
        assertTrue(manifest.contains(".AiWorkSummaryActivity"))
        listOf("workSummaryPage", "workSummaryForDate", "AI 建议", "data-summary-fold").forEach {
            assertTrue("Web 工作摘要镜像缺少：$it", web.contains(it))
        }
        assertTrue(web.contains("data-summary-prompt"))
        assertTrue(web.contains("data-summary-project-id"))
        assertFalse("Web 不得保留演示项目内容", web.contains("大卫提出了2个兼容性问题"))
        assertTrue("Web 建议卡片必须随内容增长", web.contains("min-height: 258px; height: auto"))
        assertTrue("Web 进展说明不得强制单行截断", web.contains("overflow-wrap:anywhere") && !web.contains(".work-summary-update-copy span { display:block; overflow:hidden; white-space:nowrap"))
        assertTrue("Web 折叠状态必须通过图标旋转表达", web.contains("work-summary-fold-chevron") && web.contains("aria-expanded"))
        assertTrue(createActions.contains("openProjectByTitle"))
        assertTrue(createActions.contains("if (autoSend) sendMessage()"))
    }

    @Test
    fun summaryUsesRealProjectStatusAndSelectedDay() {
        val selected = Calendar.getInstance().apply { set(2026, Calendar.AUGUST, 27, 12, 0, 0) }.timeInMillis
        fun project(id: String, status: String, day: Int, tone: String? = null) = AppProject(
            id = id, title = id, subtitle = "", updatedAt = Calendar.getInstance().apply {
                set(2026, Calendar.AUGUST, day, 9, 0, 0)
            }.timeInMillis, stage = status, workspaceHealthTone = tone
        )
        val result = generateWorkSummary(listOf(
            project("失败项目", "构建失败", 27, "bad"),
            project("完成项目", "部署完成", 27),
            project("确认项目", "待发布", 27),
            project("昨天项目", "构建失败", 26),
        ), selected)

        assertEquals(listOf("失败项目"), result.attention.map { it.project })
        assertEquals(listOf("完成项目"), result.progress.map { it.project })
        assertEquals(listOf("确认项目"), result.confirm.map { it.project })
        assertTrue(result.attention.single().highPriority)
    }
}
