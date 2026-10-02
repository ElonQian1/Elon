package com.elon.app

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.PointF
import android.view.View
import android.widget.TextView
import com.davemorrissey.labs.subscaleview.ImageSource
import com.davemorrissey.labs.subscaleview.SubsamplingScaleImageView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ChatImageReadingControllerTest {
    @Test fun realImageViewStartsAtReadableTopAndCanToggleOverview() {
        val context = RuntimeEnvironment.getApplication()
        val image = SubsamplingScaleImageView(context)
        val button = TextView(context)
        val controller = ChatImageReadingController(image, button)
        image.setOnStateChangedListener(object : SubsamplingScaleImageView.DefaultOnStateChangedListener() {
            override fun onScaleChanged(newScale: Float, origin: Int) { controller.stateChanged() }
            override fun onCenterChanged(newCenter: PointF?, origin: Int) { controller.stateChanged() }
        })
        val source = Bitmap.createBitmap(300, 3000, Bitmap.Config.ARGB_8888)
        val target = Bitmap.createBitmap(720, 1200, Bitmap.Config.ARGB_8888)
        fun draw() { image.draw(Canvas(target)) }
        try {
            image.measure(View.MeasureSpec.makeMeasureSpec(720, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(1200, View.MeasureSpec.EXACTLY))
            image.layout(0, 0, 720, 1200)
            image.setImage(ImageSource.bitmap(source))
            draw(); assertTrue(image.isReady)
            controller.ready(); draw()
            assertEquals(2.4f, image.scale, .01f)
            assertEquals(0f, image.sourceToViewCoord(0f, 0f)!!.y, 1f)
            assertEquals("查看整图", button.contentDescription)
            button.performClick(); draw()
            assertEquals(.4f, image.scale, .01f)
            assertEquals("长图阅读", button.contentDescription)
            button.performClick(); draw()
            assertEquals(2.4f, image.scale, .01f)
            assertEquals(0f, image.sourceToViewCoord(0f, 0f)!!.y, 1f)
            controller.reset(); assertFalse(button.isEnabled)
        } finally { controller.release(); image.recycle(); target.recycle() }
    }
}
