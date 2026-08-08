package com.elon.app.chatkit

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class OpenAiChatKitContractTest {
    @Test
    fun apkUsesYilongAuthNativelyAndExposesOnlyTheShortLivedChatKitSecret() {
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatkit/OpenAiChatKitActivity.kt"
        )
        val api = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/AiProviderAccountsApi.kt"
        )

        assertTrue(api.contains("AuthManager.applyAuth"))
        assertTrue(api.contains("/api/openai-chatkit/"))
        assertTrue(activity.contains("api.createChatKitSession()"))
        assertTrue(activity.contains("window.__elonResolveChatKitSecret"))
        assertFalse(activity.contains("lodex_token"))
        assertFalse(activity.contains("elon_token"))
        assertFalse(activity.contains("Authorization"))
        assertFalse(activity.contains("getCookie("))
    }

    @Test
    fun chatKitIsAnOwnerAppOnlyApiChatSurface() {
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")
        val layout = readRepositoryFile(
            "android/app/src/main/res/layout/activity_ai_provider_accounts.xml"
        )
        val declarationStart = manifest.indexOf("android:name=\".chatkit.OpenAiChatKitActivity\"")
        assertTrue(declarationStart >= 0)
        val declarationEnd = manifest.indexOf("/>", declarationStart)
        assertTrue(manifest.substring(declarationStart, declarationEnd).contains("android:exported=\"false\""))
        assertTrue(layout.contains("android:id=\"@+id/aiProviderOpenAiChatKit\""))
        assertTrue(layout.contains("不需要登录 ChatGPT"))
        assertTrue(layout.contains("不读取 ChatGPT Cookie、历史或 Plus 额度"))
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
