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

        assertTrue(layout.contains("@+id/socialAiModeControlHost"))
        assertTrue(controller.contains("SocialAiModeSegmentedControl"))
        assertTrue(control.contains("onSelected(mode)"))
        assertTrue(control.contains("social_ai_mode_\${mode.wireValue}"))
        assertFalse(controller.contains("setSingleChoiceItems"))
        assertFalse(controller.contains("showSelector"))
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
