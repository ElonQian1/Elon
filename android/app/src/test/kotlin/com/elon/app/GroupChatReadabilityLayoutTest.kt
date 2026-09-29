package com.elon.app

import android.content.res.Configuration
import android.graphics.Color
import android.graphics.Rect
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.view.ContextThemeWrapper
import android.view.View
import android.widget.TextView
import androidx.core.graphics.ColorUtils
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Robolectric
import org.robolectric.Shadows.shadowOf
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class GroupChatReadabilityLayoutTest {
    @Test fun actualComposerOccludesHistoryAndReservesItsMeasuredHeight() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        try {
        for (width in listOf(320, 411)) for (scale in listOf(1f, 1.5f, 2f)) {
            for (night in listOf(Configuration.UI_MODE_NIGHT_NO, Configuration.UI_MODE_NIGHT_YES)) {
                for (scenario in listOf("answer", "input")) {
                    val base = RuntimeEnvironment.getApplication()
                    val context = ContextThemeWrapper(base.createConfigurationContext(Configuration(base.resources.configuration).apply {
                        fontScale = scale
                        uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or night
                    }), R.style.Theme_ElonApp)
                    val fixture = GroupChatReadabilityPreview(context, scenario)
                    val root = fixture.binding.root
                    host.get().setContentView(root)
                    val density = context.resources.displayMetrics.density
                    repeat(3) {
                        root.measure(View.MeasureSpec.makeMeasureSpec((width * density).toInt(), View.MeasureSpec.EXACTLY),
                            View.MeasureSpec.makeMeasureSpec((800 * density).toInt(), View.MeasureSpec.EXACTLY))
                        root.layout(0, 0, root.measuredWidth, root.measuredHeight)
                        shadowOf(android.os.Looper.getMainLooper()).idle()
                    }
                    val dock = fixture.binding.inputLayout
                    val background = (dock.background as ColorDrawable).color
                    assertEquals("History must not bleed through the account entry", 255, Color.alpha(background))
                    assertEquals(context.getColor(R.color.mobile_surface), background)
                    assertTrue("The list must reserve the full measured dock: ${fixture.binding.chatList.paddingBottom} < ${dock.height}",
                        fixture.binding.chatList.paddingBottom >= dock.height)
                    val account = fixture.account
                    assertTrue(account.width > 0)
                    assertTrue(account.height >= (48 * density).toInt())
                    for (index in 0 until account.childCount) {
                        val label = account.getChildAt(index) as TextView
                        assertTrue(label.right <= account.width)
                        assertTrue(label.bottom <= account.height)
                        assertTrue(label.layout.height <= label.height - label.compoundPaddingTop - label.compoundPaddingBottom)
                        for (line in 0 until label.lineCount) assertEquals(0, label.layout.getEllipsisCount(line))
                    }
                    val accountBounds = Rect(0, 0, account.width, account.height)
                    val inputBounds = Rect(0, 0, fixture.composer.inputBarContainer.width, fixture.composer.inputBarContainer.height)
                    root.offsetDescendantRectToMyCoords(account, accountBounds)
                    root.offsetDescendantRectToMyCoords(fixture.composer.inputBarContainer, inputBounds)
                    assertTrue("Account and input must not overlap", accountBounds.bottom <= inputBounds.top)
                    assertTrue(inputBounds.bottom <= root.height)
                    val panel = fixture.composer.inputBarContainer.parent as View
                    val panelColor = (panel.background as GradientDrawable).color!!.defaultColor
                    assertTrue(ColorUtils.calculateContrast(fixture.binding.inputEdit.currentTextColor, panelColor) >= 4.5)
                    assertTrue(ColorUtils.calculateContrast(fixture.composer.collapsedInputPreview.currentTextColor, panelColor) >= 4.5)
                    for (icon in listOf(fixture.composer.attachmentButton, fixture.composer.emojiButton,
                        fixture.composer.inputModeButton, fixture.composer.expandEditorButton)) {
                        assertTrue(ColorUtils.calculateContrast(icon.imageTintList!!.defaultColor, panelColor) >= 3.0)
                    }
                    if (scenario == "input") {
                        assertTrue(fixture.binding.inputEdit.text.startsWith("尚未发送"))
                        assertTrue("Expanded editor must have a visible area", fixture.binding.inputEdit.height >= (42 * density).toInt())
                        val editor = fixture.binding.inputEdit
                        val editorBounds = Rect(0, 0, editor.width, editor.height)
                        root.offsetDescendantRectToMyCoords(editor, editorBounds)
                        assertTrue("Draft text must end before the action row", editorBounds.bottom <= inputBounds.top)
                        fixture.composer.inputComposerMotion.setExpanded(false, animate = false)
                        MainCollapsedInputPreviewActions(fixture.binding, { emptyList() }, { fixture.composer.collapsedInputPreview })
                            .updateCollapsedInputPreview()
                        assertTrue(ColorUtils.calculateContrast(fixture.composer.collapsedInputPreview.currentTextColor, panelColor) >= 4.5)
                    }
                    fixture.binding.chatList.adapter = null
                }
            }
        }
        } finally { host.pause().stop().destroy() }
    }
}
