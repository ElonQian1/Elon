package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebNavigationContractTest {
    @Test
    fun navigationAdapterDiscoversCurrentAccountFeaturesWithoutProviderTraffic() {
        val adapter = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_navigation.js",
        )
        val core = readRepositoryFile("android/app/src/main/assets/chatgpt_web_adapter.js")
        val pageAdapter = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )

        assertTrue(pageAdapter.contains("chatgpt_web_adapter_navigation.js"))
        assertTrue(core.contains("navigationAdapter.capabilities()"))
        assertTrue(core.contains("action === 'list_navigation'"))
        assertTrue(core.contains("action === 'select_navigation'"))
        assertTrue(adapter.contains("navigation_snapshot"))
        assertTrue(adapter.contains("featureNodes()"))
        assertTrue(adapter.contains("web_touch_request"))
        assertTrue(adapter.contains("lastFeatures.find"))
        assertTrue(adapter.contains("location.origin !== 'https://chatgpt.com'"))
        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
            assertFalse("navigation adapter must not contain $it", adapter.contains(it))
        }
    }

    @Test
    fun nativeFeatureHubIsCapabilityGatedAndKeepsOfficialFallback() {
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeFeatureHubController.kt",
        )
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebTestActivity.kt",
        )
        val layout = readRepositoryFile(
            "android/app/src/main/res/layout/activity_chatgpt_web_test.xml",
        )

        assertTrue(layout.contains("android:id=\"@+id/chatGptNativeFeatures\""))
        assertTrue(controller.contains("ChatGptWebCapabilityId.FEATURE_NAVIGATION"))
        assertTrue(controller.contains("onOpenOfficial"))
        assertTrue(controller.contains("onDismissNavigation"))
        assertTrue(activity.contains("pageAdapter::collectFeatures"))
        assertTrue(activity.contains("ChatGptWebEvent.FeatureNavigation"))
    }

    @Test
    fun nativeNavigationRowsAndComposerOptionsUseTheSharedStableSelectorContract() {
        val composer = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeComposerToolsController.kt",
        )
        val optionDialog = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeComposerOptionDialog.kt",
        )
        val conversations = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeConversationListController.kt",
        )
        val features = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptNativeFeatureHubController.kt",
        )

        assertTrue(composer.contains("ChatGptNativeComposerOptionDialog.show"))
        assertTrue(optionDialog.contains("ChatGptNativeNavigationSelector.composerOption"))
        assertTrue(conversations.contains("ChatGptNativeNavigationSelector.conversation"))
        assertTrue(features.contains("ChatGptNativeNavigationSelector.feature"))
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
