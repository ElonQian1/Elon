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
    @Test fun featuredCardKeepsActionsReachableAndDoesNotInventRuntimeStatus() {
        for (mode in listOf("light", "dark")) for (scale in listOf(1f, 2f)) {
            val context = android.view.ContextThemeWrapper(context(mode, scale), R.style.Theme_ElonApp)
            val density = context.resources.displayMetrics.density
            val project = StoreProject(id = "v2-preview", name = "很长的中文项目名称用于布局检查",
                description = "用于验证大字体的项目简介，加入项目不代表项目正在运行。", template = "android",
                ownerAccount = "preview", memberCount = 12, isPublic = true, joinMode = "open", lastTaskStatus = null)
            val prefs = context.getSharedPreferences("mobile_v2_plaza", Context.MODE_PRIVATE)
            prefs.edit().clear().commit()
            var opened = false
            val root = ProjectPlazaFeaturedSection(context, { (it * density).toInt() }, { null },
                prefs, {}, { true }, { projectPlazaPrimaryAction(it, joined = true) }, { opened = true })
                .build(listOf(project))
            root.measure(View.MeasureSpec.makeMeasureSpec((320 * density).toInt(), View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
            root.layout(0, 0, root.measuredWidth, root.measuredHeight)
            val views = descendants(root)
            val action = views.filterIsInstance<TextView>().single { it.text == "进入空间" }
            assertTrue(action.height >= (48 * density).toInt())
            assertTrue(action.right <= (action.parent as View).width)
            assertTrue(action.bottom <= (action.parent as View).height)
            action.performClick()
            assertTrue(opened)
            for (label in listOf("收藏", "点赞")) {
                val button = views.single { it.contentDescription == label }
                assertTrue(button.width >= (48 * density).toInt())
                assertTrue(button.height >= (48 * density).toInt())
                assertTrue(button.right <= (button.parent as View).width)
                button.performClick()
                assertEquals("取消$label", button.contentDescription)
            }
            assertTrue(views.filterIsInstance<TextView>().any { it.text == "已加入" })
            assertFalse(views.filterIsInstance<TextView>().any { it.text == "运行中" })
        }
    }

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
