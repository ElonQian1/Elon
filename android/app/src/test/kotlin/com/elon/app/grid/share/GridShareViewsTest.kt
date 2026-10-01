package com.elon.app.grid.share

import android.content.res.Configuration
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import com.elon.app.R
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class GridShareViewsTest {
    @Test fun publicCardAndDetailRemainReadableAcrossThemesAndFontScales() {
        val data = GridShareModel.project(mapOf("symbol" to "QNTUSDT", "direction" to "LONG", "leverage" to "10", "count" to "30", "spacing" to "ARITH", "profit" to "517.950000", "unrealizedPnl" to "-162.88", "investment" to "1000.000", "positionQty" to "12.34", "positionNotional" to "3000", "lower" to "275.590000", "upper" to "326.290000", "markPrice" to "280"), 1000)
        for (dark in listOf(false, true)) for (scale in listOf(1f, 1.5f, 2f)) {
            val base = RuntimeEnvironment.getApplication()
            val config = Configuration(base.resources.configuration).apply {
                fontScale = scale
                uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or if (dark) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
            }
            val context = android.view.ContextThemeWrapper(base.createConfigurationContext(config), R.style.Theme_ElonApp)
            val ui = GridShareViews(context)
            val card = ui.summary(data); layout(card, ui.dp(272))
            val texts = descendants(card).filterIsInstance<TextView>()
            assertEquals(context.getColor(R.color.mobile_financial_positive), texts.single { it.text == "+517.95" }.currentTextColor)
            assertEquals(context.getColor(R.color.mobile_financial_negative), texts.single { it.text == "-162.88" }.currentTextColor)
            assertTrue(texts.any { it.text == "1000" }); assertTrue(texts.any { it.text == "12.34" })
            assertFalse(texts.any { it.text == "—" || it.text == "金额与数量未公开" })
            texts.forEach { label ->
                assertTrue("overflow: ${label.text}, dark=$dark scale=$scale right=${label.right} parent=${(label.parent as View).width}", label.right <= (label.parent as View).width)
                assertTrue("empty layout: ${label.text}, dark=$dark scale=$scale", label.layout.lineCount > 0)
            }
            val details = ui.details(data); layout(details, ui.dp(272))
            val tab = descendants(details).filterIsInstance<TextView>().single { it.text == "收益" }
            assertTrue("tab target too small: ${tab.height}, expected=${ui.dp(48)}, dark=$dark scale=$scale", tab.height >= ui.dp(48)); tab.performClick(); layout(details, ui.dp(272))
            assertTrue(tab.isSelected)
            assertTrue(descendants(details).filterIsInstance<TextView>().any { it.text.toString() == "517.950000" })
            assertTrue(descendants(details).filterIsInstance<TextView>().any { it.text.toString() == "未读取" })
        }
    }
    private fun layout(view: View, width: Int) {
        view.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
        view.layout(0, 0, view.measuredWidth, view.measuredHeight)
    }
    private fun descendants(view: View): List<View> = listOf(view) + if (view is ViewGroup) (0 until view.childCount).flatMap { descendants(view.getChildAt(it)) } else emptyList()
}
