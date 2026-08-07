package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebLabContractTest {
    @Test
    fun activityPersistsWebViewCookiesWithoutExportingThem() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt"
        )

        assertTrue(activity.contains("setAcceptCookie(true)"))
        assertTrue(activity.contains("setAcceptThirdPartyCookies(binding.chatGptWebView, true)"))
        assertTrue(activity.contains("cookieManager.flush()"))
        assertTrue(activity.contains("removeAllCookies"))
        assertFalse(activity.contains("getCookie("))
        assertFalse(activity.contains("addJavascriptInterface"))
        assertFalse(activity.contains("evaluateJavascript"))
        assertFalse(activity.contains("OkHttpClient"))
    }

    @Test
    fun enhancedBridgeIsOriginScopedAndDoesNotReplayProviderTraffic() {
        val bridge = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt"
        )
        val adapter = readRepositoryFile("android/app/src/main/assets/chatgpt_web_adapter.js")

        assertTrue(bridge.contains("WebViewCompat.addWebMessageListener"))
        assertTrue(bridge.contains("ALLOWED_ORIGIN = \"https://chatgpt.com\""))
        assertFalse(bridge.contains("addJavascriptInterface"))
        assertFalse(bridge.contains("getCookie("))
        assertTrue(adapter.contains("new MutationObserver"))
        assertTrue(adapter.contains("schema: 'yilong.ai.ui.v1'"))
        assertTrue(adapter.contains("action === 'send_prompt'"))
        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
            assertFalse("page adapter must not contain $it", adapter.contains(it))
        }
    }

    @Test
    fun webViewLabIsOwnerAppOnlyAndReachableFromProviderSettings() {
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")
        val providerActivity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/AiProviderAccountsActivity.kt"
        )
        val providerLayout = readRepositoryFile(
            "android/app/src/main/res/layout/activity_ai_provider_accounts.xml"
        )

        val declarationStart = manifest.indexOf("android:name=\".chatgptweb.ChatGptWebTestActivity\"")
        assertTrue(declarationStart >= 0)
        val declarationEnd = manifest.indexOf("/>", declarationStart)
        val declaration = manifest.substring(declarationStart, declarationEnd)
        assertTrue(declaration.contains("android:exported=\"false\""))
        assertTrue(providerActivity.contains("ChatGptWebTestActivity::class.java"))
        assertTrue(providerLayout.contains("android:id=\"@+id/aiProviderChatGptWebLab\""))
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
