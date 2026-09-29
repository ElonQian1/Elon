package com.elon.app

import android.content.res.Configuration
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.core.graphics.ColorUtils
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class HomeConversationPolishTest {
    @Test fun headerPreservesFiltersAndSummaryAtLargeFont() {
        for (width in listOf(320, 411)) for (scale in listOf(1f, 1.5f, 2f)) {
            val context = context(Configuration.UI_MODE_NIGHT_YES, scale)
            val dp: (Int) -> Int = { (it * context.resources.displayMetrics.density).toInt() }
            var selected = HomeListFilterMode.All
            var summary = false
            val header = HomeConversationHeaderView(context, dp) { null }.create(selected,
                HomeConversationCounts(31, 12, 18, 1, 2), { selected = it }, { summary = true })
            header.measure(View.MeasureSpec.makeMeasureSpec(dp(width), View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
            header.layout(0, 0, header.measuredWidth, header.measuredHeight)
            val all = descendants(header).toList()
            val filters = all.filterIsInstance<TextView>().filter { it.isClickable }
            assertEquals(4, filters.size)
            filters.forEach { assertTrue(it.measuredHeight >= dp(48)) }
            filters.first { it.text.startsWith("项目") }.performClick()
            assertEquals(HomeListFilterMode.Projects, selected)
            all.first { it.contentDescription?.startsWith("AI 工作摘要") == true }.performClick()
            assertTrue(summary)
            all.filterIsInstance<TextView>().forEach { view ->
                assertTrue("Text must fit at $scale: ${view.text}", view.measuredHeight >= view.layout.height + view.compoundPaddingTop + view.compoundPaddingBottom)
            }
            if (scale == 1f) assertTrue("Header ${header.measuredHeight}px should fit ${dp(200)}px at width=$width; " +
                all.filterIsInstance<TextView>().joinToString { "${it.text}: ${it.measuredWidth}x${it.measuredHeight}, ${it.textSize}px" },
                header.measuredHeight <= dp(200))
        }
    }

    @Test fun bothThemeRolePairsMeetContrastAndNeutralBadgesStaySecondary() {
        for (night in listOf(Configuration.UI_MODE_NIGHT_NO, Configuration.UI_MODE_NIGHT_YES)) {
            val context = context(night, 1f)
            val c = MobileColors(context)
            for (surface in listOf(c.surface, c.container, c.elevated)) {
                assertTrue(ColorUtils.calculateContrast(c.text, surface) >= 4.5)
                assertTrue(ColorUtils.calculateContrast(c.muted, surface) >= 4.5)
            }
            assertTrue(ColorUtils.calculateContrast(c.onPrimary, c.primary) >= 4.5)
            assertTrue(ColorUtils.calculateContrast(c.onPrimaryContainer, c.primaryContainer) >= 4.5)
            val title = HomeRowStatusDecorations(context) { it }.createTitle("移动端项目", HomeRowBadge.PROJECT)
            val badge = descendants(title).filterIsInstance<TextView>().first { it.text == "项目" }
            assertEquals(c.muted, badge.currentTextColor)
            val rows = MainHomeRows(context, java.text.DateFormat.getTimeInstance(), { -1 }, {}, { _, _ -> }, {}, {}, { it }, { null })
            val group = AppGroup("unread-fixture", "未读消息", 0, emptyList(), 0, "离线样例", null, 8)
            val unread = descendants(rows.createGroupRow(group, false) {}).filterIsInstance<TextView>().first { it.text == "8" }
            val fill = (unread.background as android.graphics.drawable.GradientDrawable).color!!.defaultColor
            assertTrue("Unread count must be legible", ColorUtils.calculateContrast(unread.currentTextColor, fill) >= 4.5)
        }
    }

    private fun context(night: Int, scale: Float): android.content.Context {
        val base = RuntimeEnvironment.getApplication()
        return android.view.ContextThemeWrapper(base.createConfigurationContext(Configuration(base.resources.configuration).apply {
            uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or night
            fontScale = scale
        }), R.style.Theme_ElonApp)
    }
    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if (view is ViewGroup) for (index in 0 until view.childCount) yieldAll(descendants(view.getChildAt(index)))
    }
}
