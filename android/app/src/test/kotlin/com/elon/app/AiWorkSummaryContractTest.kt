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
        assertTrue("卡片内容必须使用设计稿的 32dp 对齐线", source.contains("setPadding(dp(32), dp(22), dp(32), dp(22))"))
        assertTrue("指标卡间距必须使用设计稿的 17dp 节奏", source.contains("weighted(dp(17))"))
    }

    @Test
    fun homeEntryAndWebMirrorStayConnected() {
        val header = root.resolve("android/app/src/main/kotlin/com/elon/app/HomeConversationHeaderView.kt").readText()
        val manifest = root.resolve("android/app/src/main/AndroidManifest.xml").readText()
        val web = root.resolve("server/src/assets/web_page.html").readText()

        assertTrue(header.contains("onOpenSummary()"))
        assertTrue(manifest.contains(".AiWorkSummaryActivity"))
        listOf("workSummaryPage", "大卫提出了2个兼容性问题", "AI 建议", "data-summary-fold").forEach {
            assertTrue("Web 工作摘要镜像缺少：$it", web.contains(it))
        }
    }
}
