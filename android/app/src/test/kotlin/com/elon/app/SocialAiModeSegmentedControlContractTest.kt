package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SocialAiModeSegmentedControlContractTest {
    @Test
    fun socialAiUsesOneTapSegmentedModeSwitchInsteadOfNestedDialogs() {
        val controller = read("android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt")
        val control = read("android/app/src/main/kotlin/com/elon/app/SocialAiModeSegmentedControl.kt")
        val layout = read("android/app/src/main/res/layout/activity_main.xml")
        val navigation = read("android/app/src/main/kotlin/com/elon/app/MainNavigationController.kt")

        assertTrue(layout.contains("@+id/socialAiModeControlHost"))
        assertTrue(layout.contains("android:layout_width=\"160dp\""))
        assertTrue(layout.contains("android:layout_height=\"40dp\""))
        assertTrue(controller.contains("SocialAiModeSegmentedControl"))
        assertTrue(control.contains("onSelected(mode)"))
        assertTrue(control.contains("social_ai_mode_\${mode.wireValue}"))
        assertTrue(layout.contains("android:src=\"@drawable/ic_top_add_ring_custom\""))
        assertTrue(layout.contains("android:layout_marginEnd=\"16dp\""))
        assertTrue(navigation.contains("if (isDirectSocialAiChatActive()) View.GONE else View.VISIBLE"))
        assertFalse(controller.contains("setSingleChoiceItems"))
        assertFalse(controller.contains("showSelector"))
    }

    @Test
    fun socialAiToolbarAndComposerControlsKeepSeparateVisualHierarchy() {
        val composer = read("android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt")
        val selector = read("android/app/src/main/res/drawable/bg_bottom_mode_selector.xml")
        val web = read("server/src/assets/web_page.html")

        assertTrue(composer.contains("R.drawable.bg_bottom_mode_selector"))
        assertTrue(composer.contains("LinearLayout.LayoutParams(dp(76), dp(48))"))
        assertTrue(selector.contains("android:radius=\"999dp\""))
        assertTrue(web.contains("border-radius: 999px"))
    }

    @Test
    fun socialAiComposerHasNoTranslucentBackdropBehindItsPills() {
        val composer = read("android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt")
        val layout = read("android/app/src/main/res/layout/activity_main.xml")
        val web = read("server/src/assets/web_page.html")

        assertTrue(composer.contains("root.setBackgroundColor(Color.TRANSPARENT)"))
        assertFalse(composer.contains("root.setBackgroundColor(Color.argb(77, 0, 0, 0))"))
        assertTrue(layout.contains("android:id=\"@+id/inputLayout\""))
        assertTrue(layout.contains("android:background=\"@android:color/transparent\""))
        assertTrue(web.contains(".input-bar {"))
        assertFalse(web.contains("background: rgba(0, 0, 0, 0.70);"))
    }

    private fun read(relative: String): String =
        String(Files.readAllBytes(root().resolve(relative)), StandardCharsets.UTF_8)

    private fun root(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
