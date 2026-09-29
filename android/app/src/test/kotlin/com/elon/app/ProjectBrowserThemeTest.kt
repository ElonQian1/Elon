package com.elon.app

import android.content.Context
import android.content.res.Configuration
import android.graphics.drawable.GradientDrawable
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import android.widget.ImageView
import android.widget.EditText
import androidx.appcompat.app.AppCompatDelegate
import androidx.core.graphics.ColorUtils
import com.elon.app.databinding.ActivityMainBinding
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ProjectBrowserThemeTest {
    @Test fun defaultIsDarkButEveryExplicitChoiceSurvives() {
        val context = RuntimeEnvironment.getApplication()
        val prefs = context.getSharedPreferences("mobile_appearance", Context.MODE_PRIVATE)
        prefs.edit().clear().commit()
        try {
            assertEquals(Configuration.UI_MODE_NIGHT_YES, MobileThemePreference.night(context))
            prefs.edit().putInt("mode", AppCompatDelegate.MODE_NIGHT_NO).commit()
            assertEquals(Configuration.UI_MODE_NIGHT_NO, MobileThemePreference.night(context))
            prefs.edit().putInt("mode", AppCompatDelegate.MODE_NIGHT_YES).commit()
            assertEquals(Configuration.UI_MODE_NIGHT_YES, MobileThemePreference.night(context))
            prefs.edit().putInt("mode", AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM).commit()
            assertEquals(context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK,
                MobileThemePreference.night(context))
            prefs.edit().putInt("mode", 999).commit()
            assertEquals(Configuration.UI_MODE_NIGHT_YES, MobileThemePreference.night(context))
        } finally { prefs.edit().clear().commit() }
    }

    @Test fun sheetScalesAndKeepsSearchExpandAndProjectRouting() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        try {
            for (width in listOf(320, 411)) for (scale in listOf(1f, 1.5f, 2f)) {
                for (night in listOf(Configuration.UI_MODE_NIGHT_NO, Configuration.UI_MODE_NIGHT_YES)) {
                    val base = RuntimeEnvironment.getApplication()
                    val context = android.view.ContextThemeWrapper(base.createConfigurationContext(Configuration(base.resources.configuration).apply {
                        fontScale = scale
                        uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or night
                    }), R.style.Theme_ElonApp)
                    val fixture = ProjectBrowserThemePreview(context, "normal")
                    val root = fixture.binding.root
                    host.get().setContentView(root)
                    fun settle() {
                        repeat(3) {
                            val density = context.resources.displayMetrics.density
                            root.measure(View.MeasureSpec.makeMeasureSpec((width * density).toInt(), View.MeasureSpec.EXACTLY),
                                View.MeasureSpec.makeMeasureSpec((842 * density).toInt(), View.MeasureSpec.EXACTLY))
                            root.layout(0, 0, root.measuredWidth, root.measuredHeight)
                            shadowOf(android.os.Looper.getMainLooper()).idleFor(java.time.Duration.ofMillis(400))
                        }
                    }
                    settle()
                    val sheet = fixture.binding.projectBrowserSheetContent
                    fun entries() = descendants(sheet).filter { it.contentDescription?.startsWith("打开项目 ") == true }.toList()
                    assertEquals(16, entries().size)
                    descendants(sheet).filterIsInstance<TextView>().filter { it !is EditText }.forEach {
                        assertTrue("Clipped label at $width/$scale: ${it.text}",
                            it.layout.height <= it.height - it.compoundPaddingTop - it.compoundPaddingBottom)
                    }
                    val search = root.findViewById<EditText>(R.id.projectBrowserSearchInput)
                    val panel = (root.findViewById<ImageView>(R.id.projectBrowserSearchBackground).drawable as GradientDrawable).color!!.defaultColor
                    assertTrue(ColorUtils.calculateContrast(search.currentHintTextColor, panel) >= 4.5)
                    descendants(sheet).filterIsInstance<ImageView>().filter {
                        it.id == R.id.projectBrowserSearchIcon || it.contentDescription?.startsWith("展开") == true
                    }.forEach { assertTrue(ColorUtils.calculateContrast(it.imageTintList!!.defaultColor,
                        context.getColor(R.color.mobile_surface)) >= 3.0) }
                    descendants(sheet).first { it.contentDescription == "展开个人项目" }.performClick()
                    settle()
                    assertEquals(18, entries().size)
                    search.setText("没有匹配的查询")
                    assertEquals(0, entries().size)
                    search.setText("手机")
                    settle()
                    assertEquals(5, entries().size)
                    entries().first().performClick()
                    settle()
                    assertEquals(8, fixture.openedProjectIndex)
                    assertFalse(fixture.controller.isOpen)
                }
            }
        } finally { host.pause().stop().destroy() }
    }

    @Test fun actualProjectSheetHasReadableLabelsInBothThemes() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        try {
            for (night in listOf(Configuration.UI_MODE_NIGHT_NO, Configuration.UI_MODE_NIGHT_YES)) {
                val base = host.get()
                val context = android.view.ContextThemeWrapper(base.createConfigurationContext(
                    Configuration(base.resources.configuration).apply {
                        uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or night
                    }), R.style.Theme_ElonApp)
                val binding = ActivityMainBinding.inflate(LayoutInflater.from(context))
                val controller = ProjectBrowserSheetController(context, binding, { it },
                    ProjectBrowserSheetDependencies({ listOf(AppProject("offline", "示例项目", "", 0)) }, {}, { null }), {})
                controller.setup()
                val search = binding.root.findViewById<android.widget.EditText>(R.id.projectBrowserSearchInput)
                search.setText("示例") // Uses the production filtering/rendering path, without network.
                val background = binding.projectBrowserSheetBackground.drawable
                assertTrue("The sheet must resolve a themed surface instead of a fixed black bitmap", background is GradientDrawable)
                val color = (background as GradientDrawable).color!!.defaultColor
                assertEquals(context.getColor(R.color.mobile_surface), color)
                val labels = descendants(binding.projectBrowserSheetContent).filterIsInstance<TextView>().toList()
                assertTrue(labels.any { it.text.toString() == "示例项目" })
                labels.forEach { assertTrue("Unreadable ${it.text}", ColorUtils.calculateContrast(it.currentTextColor, color) >= 4.5) }
            }
        } finally { host.pause().stop().destroy() }
    }

    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if (view is ViewGroup) for (index in 0 until view.childCount) yieldAll(descendants(view.getChildAt(index)))
    }
}
