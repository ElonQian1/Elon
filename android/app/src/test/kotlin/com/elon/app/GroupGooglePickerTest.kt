package com.elon.app

import android.app.Application
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import androidx.appcompat.app.AppCompatActivity
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class, qualifiers = "w320dp-h640dp")
class GroupGooglePickerTest {
    @Test fun googleRowIsVisibleAndClickDispatchesItsOwnIdentity() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        lifecycle.setup()
        var selected = ""
        val sheet = requireNotNull(ChatAiChoiceSheet.show(activity, "切换 AI",
            GroupAiProviderChoices.options(GroupAiConfiguration()), { selected = it }))
        try {
            shadowOf(Looper.getMainLooper()).idle()
            val root = requireNotNull(sheet.dialog.window).decorView
            val density = activity.resources.displayMetrics.density
            val width = (320 * density).toInt()
            val height = (640 * density).toInt()
            root.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(height, View.MeasureSpec.EXACTLY))
            root.layout(0, 0, width, height)
            val rows = descendants(root).filter { it.contentDescription?.startsWith("group-ai-provider:") == true }.toList()
            assertEquals(3, rows.size)
            val google = rows.single { it.contentDescription == "group-ai-provider:GOOGLE" }
            assertTrue(google.isShown)
            assertTrue(google.isEnabled)
            assertTrue(google.width > 0 && google.height >= 48 * density)
            assertTrue(google.performClick())
            assertEquals("GOOGLE", selected)
            assertFalse(sheet.dialog.isShowing)
        } finally {
            sheet.dialog.dismiss()
            lifecycle.pause().stop().destroy()
        }
    }

    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if (view is ViewGroup) for (index in 0 until view.childCount) yieldAll(descendants(view.getChildAt(index)))
    }
}
