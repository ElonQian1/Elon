package com.elon.app

import android.app.Application
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.recyclerview.widget.RecyclerView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowDialog

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class GroupMentionPickerTest {
    @Test fun sheetListHasHeightAndMultiSelectionSurvivesSearch() {
        val lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get()
        activity.setTheme(R.style.Theme_ElonApp)
        lifecycle.setup()
        var chosen = emptyList<GroupMentionTarget>()
        val picker = GroupMentionPicker(activity, { chosen = it }, {})
        picker.show()
        picker.showMembers(listOf(GroupMentionTarget("ai", "EL", isAi = true)) +
            (0..114).map { GroupMentionTarget("u$it", "群友$it") })
        val root = ShadowDialog.getLatestDialog().window!!.decorView
        fun layout() {
            shadowOf(Looper.getMainLooper()).idle()
            root.measure(View.MeasureSpec.makeMeasureSpec(1080, View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(1800, View.MeasureSpec.EXACTLY))
            root.layout(0, 0, 1080, 1800)
        }
        fun text(value: String) = descendants(root).filterIsInstance<TextView>().first { it.text.toString() == value }
        layout()
        val list = descendants(root).filterIsInstance<RecyclerView>().single()
        assertTrue("Member list must fill the sheet, not collapse to zero", list.height > 100)
        assertEquals(116, list.adapter!!.itemCount)
        text("多选").performClick()
        val search = descendants(root).filterIsInstance<EditText>().single()
        search.setText("群友114")
        layout()
        assertEquals(1, list.adapter!!.itemCount)
        list.getChildAt(0).performClick()
        search.setText("群AI")
        layout()
        list.getChildAt(0).performClick()
        text("完成（2）").performClick()
        assertEquals(listOf("u114", "ai"), chosen.map { it.id })
        lifecycle.pause().stop().destroy()
    }

    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if (view is ViewGroup) for (index in 0 until view.childCount) yieldAll(descendants(view.getChildAt(index)))
    }
}
