package com.elon.app.sociallinks

import android.app.Activity
import android.app.Application
import android.content.res.Configuration
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.core.graphics.ColorUtils
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class, qualifiers = "mdpi")
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class SocialLinkCardThemeTest {
    @Test fun cardTextHasContrastAndFooterGrowsInBothThemesAtLargeFont() {
        val surfaces = mutableSetOf<Int>()
        for (night in listOf(false, true)) for (scale in listOf(1f, 2f)) {
            Robolectric.buildActivity(Activity::class.java).use { controller ->
                val activity = controller.setup().get()
                @Suppress("DEPRECATION")
                activity.resources.updateConfiguration(Configuration(activity.resources.configuration).apply {
                    fontScale = scale
                    uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                        if (night) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
                }, activity.resources.displayMetrics)
                for (poster in listOf(false, true)) {
                    val card = SocialLinkCardView(activity, poster) {}
                    card.bind(SocialLinkPolicy.link("https://www.bilibili.com/video/BV19eYH6NEsC/?t=80")!!
                        .copy(title = "媒体标题", author = "作者", summary = "内容摘要"))
                    card.measure(View.MeasureSpec.makeMeasureSpec(220, View.MeasureSpec.EXACTLY),
                        View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
                    card.layout(0, 0, 220, card.measuredHeight)
                    val surface = (card.background as GradientDrawable).color!!.defaultColor
                    surfaces += surface
                    listOf(card.title, card.time).forEach {
                        assertTrue("Text contrast at night=$night", ColorUtils.calculateContrast(it.currentTextColor, surface) >= 4.5)
                    }
                    val footer = card.getChildAt(card.childCount - 1)
                    if (poster) {
                        assertEquals(card.height, footer.bottom)
                        assertTrue(footer.height >= 58)
                        val footerColor = (footer.background as android.graphics.drawable.ColorDrawable).color
                        assertTrue(ColorUtils.calculateContrast(card.source.currentTextColor, footerColor) >= 4.5)
                        labels(footer).filter { it.text.isNotEmpty() && it.visibility == View.VISIBLE }.forEach {
                            assertTrue("Clipped ${it.text} at scale=$scale", it.layout.height <= it.height - it.compoundPaddingTop - it.compoundPaddingBottom)
                        }
                    }
                }
            }
        }
        assertEquals("Light and dark cards have distinct surfaces", 2, surfaces.size)
    }

    private fun labels(view: View): List<TextView> = if (view is TextView) listOf(view)
        else if (view is ViewGroup) (0 until view.childCount).flatMap { labels(view.getChildAt(it)) }
        else emptyList()
}
