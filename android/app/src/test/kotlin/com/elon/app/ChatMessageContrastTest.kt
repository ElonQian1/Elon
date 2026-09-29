package com.elon.app

import android.content.res.Configuration
import android.graphics.drawable.GradientDrawable
import android.view.ContextThemeWrapper
import android.widget.FrameLayout
import androidx.core.graphics.ColorUtils
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class ChatMessageContrastTest {
    @Test fun boundMessagesRemainReadableAcrossRolesThemesAndRecycling() {
        for (night in listOf(Configuration.UI_MODE_NIGHT_NO, Configuration.UI_MODE_NIGHT_YES)) {
            val base = RuntimeEnvironment.getApplication()
            val context = ContextThemeWrapper(base.createConfigurationContext(Configuration(base.resources.configuration).apply {
                uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or night
            }), R.style.Theme_ElonApp)
            val roles = listOf("ai", "friend", "user", "ai-intent", "ai-working", "ai-progress",
                "ai-cli-log", "ai-tool", "ai-complete", "ai-stopped", "error")
            for (role in roles) {
                val message = ChatMessage(role, "中文消息与 English 123", id = "contrast-$role")
                val adapter = ChatAdapter(mutableListOf(message))
                val holder = adapter.onCreateViewHolder(FrameLayout(context), adapter.getItemViewType(0))
                adapter.onBindViewHolder(holder, 0)
                holder.friendAvatar?.let { avatar ->
                    val avatarBackground = (avatar.background as GradientDrawable).color!!.defaultColor
                    assertTrue("Fallback sender avatar must remain readable in night=$night",
                        ColorUtils.calculateContrast(avatar.currentTextColor, avatarBackground) >= 4.5)
                }
                val background = (holder.bubble?.background as? GradientDrawable)?.color?.defaultColor
                    ?: (holder.text.background as? GradientDrawable)?.color?.defaultColor
                    ?: context.getColor(R.color.mobile_surface)
                assertTrue("$role night=$night must reach 4.5:1, actual=${ColorUtils.calculateContrast(holder.text.currentTextColor, background)}",
                    ColorUtils.calculateContrast(holder.text.currentTextColor, background) >= 4.5)
                if (role in listOf("ai", "friend", "user")) {
                    message.recalledAt = "recalled"
                    adapter.onBindViewHolder(holder, 0)
                    assertTrue("recalled $role night=$night", ColorUtils.calculateContrast(holder.text.currentTextColor, background) >= 4.5)
                    message.recalledAt = null
                    adapter.onBindViewHolder(holder, 0)
                    assertTrue("recycled $role night=$night", ColorUtils.calculateContrast(holder.text.currentTextColor, background) >= 4.5)
                }
                adapter.onViewRecycled(holder)
            }
        }
    }
}
