package com.elon.app

import android.content.Context
import android.content.res.Configuration
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import org.robolectric.RuntimeEnvironment
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class MobileDesignV2RuntimeTest {
    @Test fun productionProjectRowsReflowAndTabsRemainUsableAtLargeFontSizes() {
        for (widthDp in listOf(320, 411, 600)) for (scale in listOf(1f, 2f)) {
            val context = context("light", scale)
            val root = preview(context, "projects", "light", scale)
            layout(root, widthDp)
            val views = descendants(root)
            val tabs = views.filterIsInstance<TextView>().filter { it.text.toString() in listOf("独立", "联合") }
            assertEquals(2, tabs.size)
            tabs.forEach { assertTrue(it.height >= (48 * context.resources.displayMetrics.density).toInt()) }
            val metadata = views.filterIsInstance<TextView>().filter { it.text.startsWith("成员：") }
            assertEquals(2, metadata.size)
            metadata.forEach { assertTrue("metadata must fit its parent", it.right <= (it.parent as View).width) }
            tabs.first { it.text == "联合" }.performClick()
            assertTrue(descendants(root).filterIsInstance<TextView>().any { it.text == "联合项目示例" })
        }
    }

    @Test fun realChatLayoutsUsePairedColorsInBothThemes() {
        for (mode in listOf("light", "dark")) {
            val context = context(mode, 2f)
            val root = preview(context, "chat_result", mode, 2f)
            layout(root, 320)
            val messages = descendants(root).filterIsInstance<TextView>().filter { it.id == R.id.messageText }
            assertEquals(2, messages.size)
            assertEquals(context.getColor(R.color.mobile_on_primary_container), messages[0].currentTextColor)
            assertEquals(context.getColor(R.color.mobile_on_surface), messages[1].currentTextColor)
            messages.forEach { assertTrue(it.height > 0); assertTrue(it.width > 0) }
        }
    }

    private fun context(theme: String, scale: Float): Context {
        val base = RuntimeEnvironment.getApplication()
        return base.createConfigurationContext(Configuration(base.resources.configuration).apply {
            fontScale = scale
            uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                if (theme == "dark") Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
        })
    }
    private fun preview(context: Context, scenario: String, theme: String, scale: Float) =
        mobileDesignV2PreviewScenario().createView(context,
            UiRuntimePreviewRequest("elon.mobile.design_v2", scenario, theme, scale, "zh-CN"))
    private fun layout(view: View, widthDp: Int) {
        val density = view.resources.displayMetrics.density
        view.measure(View.MeasureSpec.makeMeasureSpec((widthDp * density).toInt(), View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec((800 * density).toInt(), View.MeasureSpec.EXACTLY))
        view.layout(0, 0, view.measuredWidth, view.measuredHeight)
    }
    private fun descendants(view: View): List<View> = listOf(view) + if (view is ViewGroup)
        (0 until view.childCount).flatMap { descendants(view.getChildAt(it)) } else emptyList()
}
