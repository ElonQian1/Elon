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

        val content = root.resolve("android/app/src/main/kotlin/com/elon/app/AiWorkSummaryContent.kt").readText()
        assertTrue(source.contains("generateWorkSummary"))
        assertTrue(content.contains("content.visibility"))
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
