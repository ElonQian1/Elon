package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebOfficialActivityContractTest {
    @Test
    fun officialFallbackKeepsTheCompleteWebSessionWithoutOwningNativeChat() {
        val activity = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebOfficialActivity.kt",
        )
        val intent = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebOfficialFallbackIntent.kt",
        )
        val manifest = read("android/app/src/main/AndroidManifest.xml")

        assertTrue(activity.contains("class ChatGptWebOfficialActivity"))
        assertTrue(activity.contains("setAcceptCookie(true)"))
        assertTrue(activity.contains("setAcceptThirdPartyCookies(webView, true)"))
        assertTrue(activity.contains("ChatGptWebAuthenticationSupport.configure(settings)"))
        assertTrue(activity.contains("fileChooserController.show("))
        assertTrue(activity.contains("audioPermissionController.handle(request)"))
        assertTrue(activity.contains("proxyController.prepare"))
        assertTrue(activity.contains("sessionRestorer.onPageReady(url)"))
        assertTrue(activity.contains("cookieManager.flush()"))
        assertFalse(activity.contains("removeAllCookies"))
        assertFalse(activity.contains("clearCache("))
        assertFalse(activity.contains("evaluateJavascript"))
        assertFalse(activity.contains("McpNativeControlBinding"))
        assertFalse(activity.contains("ChatGptWebPageAdapter"))
        assertTrue(intent.contains("ChatGptWebOfficialActivity::class.java"))
        assertTrue(manifest.contains(".chatgptweb.ChatGptWebOfficialActivity"))
        assertFalse(manifest.contains(".chatgptweb.ChatGptWebTestActivity"))
    }

    @Test
    fun legacyDiagnosticPageAndLayoutsAreRemoved() {
        val root = root()
        listOf(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt",
            "android/app/src/main/res/layout/activity_chatgpt_web_test.xml",
            "android/app/src/main/res/layout/sheet_chatgpt_conversations.xml",
            "android/app/src/main/res/layout/sheet_chatgpt_features.xml",
        ).forEach { relative ->
            assertFalse("Legacy diagnostic artifact must be removed: $relative", Files.exists(root.resolve(relative)))
        }
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
