package com.elon.app

import java.io.File
import org.junit.Assert.assertTrue
import org.junit.Test

class AiWorkSummaryContractTest {
    private val root = generateSequence(File(System.getProperty("user.dir")).canonicalFile) { it.parentFile }
        .first { File(it, "android/app/src/main/AndroidManifest.xml").isFile }

    @Test
    fun androidSummaryKeepsRequiredInformationAndActions() {
        val source = root.resolve("android/app/src/main/kotlin/com/elon/app/AiWorkSummaryActivity.kt").readText()

        listOf("大卫提出了2个兼容性问题", "AI 建议", "交给 AI 处理", "有新进展", "待确认").forEach {
            assertTrue("Android 工作摘要缺少：$it", source.contains(it))
        }
        assertTrue(source.contains("ic_work_summary_calendar"))
        assertTrue(source.contains("contentDescription = \"选择摘要日期\""))
        assertTrue("次要摘要区域必须支持折叠", source.contains("content.visibility"))
        assertTrue("卡片必须使用设计稿的 30dp 内容对齐线", source.contains("setPadding(dp(30), dp(21), dp(30), dp(21))"))
        assertTrue("指标卡间距必须使用设计稿的 16dp 节奏", source.contains("weighted(dp(16))"))
        assertTrue("重点卡片必须维持设计稿高度", source.contains("true, true, 268"))
        assertTrue("进展项必须显示日期", source.contains("WorkSummaryUpdate(\"杀蟑螂\"") && source.contains("8月17号"))
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
        listOf("workSummaryPage", "大卫提出了2个兼容性问题", "AI 建议", "data-summary-fold").forEach {
            assertTrue("Web 工作摘要镜像缺少：$it", web.contains(it))
        }
        assertTrue(web.contains("data-summary-prompt"))
        assertTrue(web.contains("data-summary-project"))
        assertTrue(createActions.contains("openProjectByTitle"))
        assertTrue(createActions.contains("if (autoSend) sendMessage()"))
    }
}
