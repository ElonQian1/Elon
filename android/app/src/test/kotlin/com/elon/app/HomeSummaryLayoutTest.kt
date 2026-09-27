package com.elon.app

import android.content.res.Configuration
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class, qualifiers = "mdpi")
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class HomeSummaryLayoutTest {
    @Test fun allSevenDatesHaveReadableLabelsAndReachableTargets() {
        for (night in listOf(false, true)) for (scale in listOf(1f, 2f)) {
            Robolectric.buildActivity(AppCompatActivity::class.java).use { controller ->
                val activity = controller.get(); activity.setTheme(R.style.Theme_ElonApp)
                controller.setup(); configure(activity, night, scale)
                val selected = java.time.LocalDate.of(2026, 9, 28)
                val clicked = mutableListOf<java.time.LocalDate>()
                val view = createSocialSidebarDateStrip(activity, selected, clicked::add, {it}, {null})
                activity.setContentView(view)
                view.measure(View.MeasureSpec.makeMeasureSpec(240, View.MeasureSpec.EXACTLY),
                    View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
                view.layout(0, 0, 240, view.measuredHeight)
                val dates = nodes(view).filterIsInstance<TextView>()
                assertEquals(7, dates.size)
                assertEquals(1, dates.count { it.isSelected })
                dates.forEach {
                    assertTrue(it.width >= 48); assertTrue(it.height >= 48)
                    assertTrue(it.contentDescription.isNotBlank()); it.performClick()
                }
                assertEquals((-3L..3L).map(selected::plusDays), clicked)
                assertTrue(view.getChildAt(0).width > view.width)
                readable(view)
            }
        }
    }

    @Test fun filtersAndSummaryGrowAtLargeFontSizesWithoutLosingTheirActions() {
        for (night in listOf(false, true)) for (scale in listOf(1f, 2f)) {
            Robolectric.buildActivity(AppCompatActivity::class.java).use { controller ->
                val activity = controller.get(); activity.setTheme(R.style.Theme_ElonApp)
                controller.setup()
                configure(activity, night, scale)
                val selections = mutableListOf<HomeListFilterMode>(); var summaries = 0
                val header = HomeConversationHeaderView(activity, {it}, {null}).create(HomeListFilterMode.All,
                    HomeConversationCounts(10, 2, 3, 5, 8), selections::add) { summaries++ }
                activity.setContentView(header); layout(header)
                val controls = nodes(header).filter { it.isClickable }
                assertEquals(5, controls.size)
                controls.forEach { assertTrue(it.height >= 48); it.performClick() }
                assertEquals(listOf(HomeListFilterMode.All, HomeListFilterMode.Friends, HomeListFilterMode.Projects, HomeListFilterMode.Conversations), selections)
                assertEquals(1, summaries)
                assertTrue(nodes(header).filterIsInstance<TextView>().any { it.text == "3个项目 · 8条未读消息" })
                readable(header)
            }
        }
    }

    @Test fun summaryRendersRealStatesAndDispatchesReviewedActions() {
        for (night in listOf(false, true)) for (scale in listOf(1f, 2f)) {
            Robolectric.buildActivity(AppCompatActivity::class.java).use { controller ->
                val activity = controller.get(); activity.setTheme(R.style.Theme_ElonApp)
                controller.setup(); configure(activity, night, scale)
                var dates = 0; val opened = mutableListOf<String>(); val actions = mutableListOf<String>()
                val item = GeneratedWorkSummaryItem("a", "长项目名称用于窄屏检查", "需要核对构建结果", "构建结果待确认，请阅读完整日志。",
                    "查看结果后再决定下一步。", "交给 AI 处理", "查看项目", WorkSummarySection.ATTENTION, 1, true)
                val view = AiWorkSummaryContent(activity, {}, {dates++}, opened::add, { _, value -> actions.add(value) })
                    .create(GeneratedWorkSummary(1, listOf(item), listOf(item), emptyList()), "今天")
                activity.setContentView(view); layout(view)
                readable(view)
                val labels = nodes(view).filterIsInstance<TextView>()
                labels.single { it.contentDescription == "选择摘要日期" }.performClick()
                labels.first { it.text == item.project }.performClick()
                labels.single { it.text == "交给 AI 处理" }.performClick()
                labels.single { it.text == "查看项目" }.performClick()
                val fold = labels.single { it.contentDescription == "收起有新进展" }
                fold.performClick(); assertEquals("展开有新进展", fold.contentDescription)
                assertEquals(1, dates); assertEquals(listOf(item.project), opened)
                assertEquals(listOf("交给 AI 处理", "查看项目"), actions)
            }
        }
    }

    private fun configure(activity: AppCompatActivity, night: Boolean, scale: Float) {
        @Suppress("DEPRECATION")
        activity.resources.updateConfiguration(Configuration(activity.resources.configuration).apply {
            fontScale = scale
            uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                if (night) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
        }, activity.resources.displayMetrics)
    }
    private fun layout(view: View) {
        view.measure(View.MeasureSpec.makeMeasureSpec(320, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(800, View.MeasureSpec.EXACTLY))
        view.layout(0, 0, 320, 800)
    }
    private fun readable(root: View) {
        nodes(root).filterIsInstance<TextView>().filter { it.visibility == View.VISIBLE && it.text.isNotEmpty() }.forEach {
            assertTrue("Clipped ${it.text}", it.layout.height <= it.height - it.compoundPaddingTop - it.compoundPaddingBottom)
            if (it.isClickable) assertTrue("Small target ${it.text}", it.height >= 48)
        }
    }
    private fun nodes(view: View): List<View> = listOf(view) + if (view is ViewGroup)
        (0 until view.childCount).flatMap { nodes(view.getChildAt(it)) } else emptyList()
}
