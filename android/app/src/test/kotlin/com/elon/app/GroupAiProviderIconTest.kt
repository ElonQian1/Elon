package com.elon.app

import android.content.Context
import android.graphics.drawable.LayerDrawable
import android.view.View
import android.widget.TextView
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], qualifiers = "w360dp-h800dp-mdpi", application = android.app.Application::class)
class GroupAiProviderIconTest {
    @Test fun groupMarkIsLegibleAndFitsExistingSelector() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val button = TextView(context).apply { setPadding(14, 0, 24, 0); textSize = 14f }
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        WebChatComposerProviderPresentation.applyGroup(button, provider, "默认")
        button.measure(View.MeasureSpec.makeMeasureSpec(142, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(48, View.MeasureSpec.EXACTLY))
        button.layout(0, 0, 142, 48)
        val icon = button.compoundDrawablesRelative[0] as LayerDrawable
        assertEquals(32, icon.bounds.width())
        assertEquals(3, icon.getLayerInsetLeft(1))
        assertEquals(3, icon.getLayerInsetRight(1))
        assertTrue(button.paint.measureText("默认") <= button.width - button.compoundPaddingLeft - button.compoundPaddingRight)
        assertEquals("默认", button.text.toString())
        assertTrue(button.contentDescription.contains("ChatGPT"))
    }

    @Test fun groupSizeDoesNotLeakIntoPersonalOrWorkPresentation() {
        val button = TextView(ApplicationProvider.getApplicationContext<Context>())
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        WebChatComposerProviderPresentation.applyGroup(button, provider, "极高")
        WebChatComposerProviderPresentation.apply(button, provider, "极高")
        assertEquals(18, button.compoundDrawablesRelative[0].bounds.width())
        WebChatComposerProviderPresentation.clear(button)
        assertNull(button.compoundDrawablesRelative[0])
        assertEquals(0, button.compoundDrawablePadding)
    }
}
