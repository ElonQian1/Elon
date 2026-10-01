package com.elon.app

import android.app.Application
import android.graphics.Rect
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ProfileQrNavigationTest {
    @Test fun qrTouchTargetOpensQrWithoutOpeningParentProfile() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        lifecycle.setup()
        try {
            val binding = ActivityMainBinding.inflate(activity.layoutInflater)
            activity.setContentView(binding.root)
            var profileClicks = 0
            UserProfileViews.renderSummary(activity, binding, OkHttpClient(), "http://localhost") {
                profileClicks++
            }
            val card = descendants(binding.root).first { it.tag == "profile_summary_card" } as ViewGroup
            val qr = descendants(card).first { it.contentDescription == "我的二维码" }
            val density = activity.resources.displayMetrics.density
            assertTrue(qr.isClickable)
            assertTrue(qr.isFocusable)
            assertTrue(qr.layoutParams.width >= (48 * density).toInt())
            assertTrue(qr.layoutParams.height >= (48 * density).toInt())
            card.measure(
                View.MeasureSpec.makeMeasureSpec((360 * density).toInt(), View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(card.layoutParams.height, View.MeasureSpec.EXACTLY)
            )
            card.layout(0, 0, card.measuredWidth, card.measuredHeight)
            val bounds = Rect().also(qr::getHitRect)
            // The image center and the padding around it both belong to the QR action.
            for (x in listOf(bounds.centerX(), bounds.left + 1)) {
                val y = bounds.centerY().toFloat()
                for (action in listOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
                    val event = MotionEvent.obtain(0, 10, action, x.toFloat(), y, 0)
                    try { assertTrue(card.dispatchTouchEvent(event)) } finally { event.recycle() }
                }
                shadowOf(android.os.Looper.getMainLooper()).idle()
                assertEquals(PersonalQrCodeActivity::class.java.name,
                    shadowOf(activity).nextStartedActivity?.component?.className)
                assertEquals(0, profileClicks)
                assertNull(shadowOf(activity).nextStartedActivity)
            }
            qr.performClick() // Keyboard/accessibility activation uses the same route.
            assertEquals(PersonalQrCodeActivity::class.java.name,
                shadowOf(activity).nextStartedActivity?.component?.className)
            card.performClick()
            assertEquals(1, profileClicks)
            assertNull(shadowOf(activity).nextStartedActivity)
        } finally {
            lifecycle.pause().stop().destroy()
        }
    }

    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if (view is ViewGroup) for (index in 0 until view.childCount) yieldAll(descendants(view.getChildAt(index)))
    }
}
