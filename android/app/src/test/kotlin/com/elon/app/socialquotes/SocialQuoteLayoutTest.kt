package com.elon.app.socialquotes

import android.content.res.Configuration
import android.view.ContextThemeWrapper
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.TextView
import androidx.core.view.children
import com.elon.app.ChatAdapter
import com.elon.app.ChatMessage
import com.elon.app.R
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class SocialQuoteLayoutTest {
    @Test fun quoteIsOutsideBubbleBoundedAndClearedOnRecycle() {
        for (night in listOf(Configuration.UI_MODE_NIGHT_NO, Configuration.UI_MODE_NIGHT_YES)) {
            for (scale in listOf(1f, 2f)) for (role in listOf("user", "friend")) {
                val app = RuntimeEnvironment.getApplication()
                val config = Configuration(app.resources.configuration).apply { uiMode = night; fontScale = scale }
                val context = ContextThemeWrapper(app.createConfigurationContext(config), R.style.Theme_ElonApp)
                val quote = SocialQuote("original", "示例群友", "这是一条很长的引用消息，需要截断但不能遮挡正文或取消按钮。".repeat(8))
                val message = ChatMessage(role, "新的回复", quote = quote)
                val adapter = ChatAdapter(mutableListOf(message))
                val holder = adapter.onCreateViewHolder(FrameLayout(context), adapter.getItemViewType(0))
                adapter.onBindViewHolder(holder, 0)
                val width = (320 * context.resources.displayMetrics.density).toInt()
                holder.itemView.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY), View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
                holder.itemView.layout(0, 0, width, holder.itemView.measuredHeight)
                val host = holder.itemView.findViewById<ViewGroup>(R.id.messageQuote)
                assertEquals(View.VISIBLE, host.visibility)
                assertNotSame(holder.bubble, host.parent)
                assertTrue(host.measuredWidth in 1..width)
                val labels = descendants(host).filterIsInstance<TextView>().toList()
                assertEquals(1, labels.size)
                assertEquals(2, labels.single().maxLines)
                assertEquals("新的回复", holder.text.text.toString())
                message.quote = null
                adapter.onBindViewHolder(holder, 0)
                assertEquals(View.GONE, host.visibility)
                assertEquals(0, host.childCount)
            }
        }
    }

    @Test fun draftCloseIsIndependentOfTextAndHasTouchTarget() {
        val context = ContextThemeWrapper(RuntimeEnvironment.getApplication(), R.style.Theme_ElonApp)
        val view = SocialQuotePreview(context)
        var cancelled = false
        view.bind(SocialQuote("id", "我", "原消息"), cancel = { cancelled = true })
        val close = view.children.first { it.contentDescription == "取消引用" }
        assertTrue(close.layoutParams.width >= (48 * context.resources.displayMetrics.density).toInt())
        close.performClick()
        assertTrue(cancelled)
    }
    private fun descendants(view: ViewGroup): Sequence<View> = view.children.flatMap {
        sequenceOf(it) + if (it is ViewGroup) descendants(it) else emptySequence()
    }
}
