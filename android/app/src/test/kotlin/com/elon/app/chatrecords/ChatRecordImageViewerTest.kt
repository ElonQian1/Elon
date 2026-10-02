package com.elon.app.chatrecords

import android.app.Activity
import android.app.Application
import android.view.View
import android.view.ViewGroup
import com.davemorrissey.labs.subscaleview.SubsamplingScaleImageView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowDialog
import java.io.File

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordImageViewerTest {
    @Test fun importedImageOpensSharedTiledViewerAndClosesWithoutLeavingReader() {
        val controller = Robolectric.buildActivity(Activity::class.java).setup()
        val activity = controller.get()
        val file = File(activity.cacheDir, "record-image-viewer-test.png").apply { writeBytes(byteArrayOf(0)) }
        try {
            val row = RecordRow("image", null, "fixture", "", "image", "", "long-image.png", "asset")
            ChatRecordMedia.open(activity, row, file) { fail("image must not launch video") }
            val dialog = ShadowDialog.getLatestDialog()
            assertTrue(dialog.isShowing)
            val views = descendants(dialog.window!!.decorView)
            assertEquals(1, views.filterIsInstance<SubsamplingScaleImageView>().size)
            assertTrue(views.any { it.contentDescription == "高清图片：long-image.png" })
            assertTrue(views.any { it.contentDescription == "原始比例" })
            views.single { it.contentDescription == "关闭图片" }.performClick()
            assertFalse(dialog.isShowing)
            assertFalse(activity.isFinishing)
        } finally { controller.pause().stop().destroy(); file.delete() }
    }

    private fun descendants(view: View): List<View> = listOf(view) +
        if (view is ViewGroup) (0 until view.childCount).flatMap { descendants(view.getChildAt(it)) } else emptyList()
}
