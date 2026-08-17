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
        val layoutAdapter = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_layout.js",
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
        assertTrue(adapter.contains("if (!scopes.length) return []"))
        assertTrue(adapter.contains("function isSidebarScope"))
        assertTrue(adapter.contains("rect.width >= window.innerWidth * 0.35"))
        assertTrue(adapter.contains("rect.height >= window.innerHeight * 0.6"))
        assertTrue(adapter.contains("layout.requestSemanticTouch('navigation'"))
        val dismiss = adapter.substringAfter("function dismiss")
            .substringBefore("function capabilities")
        assertTrue(dismiss.contains("layout.requestSemanticTouch('close'"))
        assertTrue(dismiss.contains("'overlay'"))
        assertTrue(dismiss.contains("result('dismiss_navigation', false"))
        assertTrue(layoutAdapter.contains("function requestSemanticTouch"))
        val requestList = adapter.substringAfter("function requestList")
            .substringBefore("function collectList")
        assertTrue(requestList.indexOf("emitSnapshot") < requestList.indexOf("sidebarButton(true)"))
        listOf("document.cookie", "fetch(", "XMLHttpRequest", "WebSocket", "Authorization").forEach {
            assertFalse("navigation adapter must not contain $it", adapter.contains(it))
        }
    }

    @Test
    fun productionFeatureNavigationKeepsTheOfficialFallback() {
        val coordinator = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureNavigation.kt",
        )
        val background = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )

        assertTrue(coordinator.contains("openOfficialFallback"))
        assertTrue(coordinator.contains("打开官方页"))
        assertTrue(background.contains("ChatGptWebEvent.FeatureNavigation"))
        assertTrue(background.contains("adapter::collectFeatures"))
    }

    @Test
    fun conversationListOpensTheMobileSidebarBeforeTakingAStableSnapshot() {
        val adapter = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversations.js",
        )
        val history = readRepositoryFile(
            "android/app/src/main/assets/chatgpt_web_adapter_conversation_history.js",
        )
        val pageAdapter = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )
        val requestList = adapter.substringAfter("function requestList")
            .substringBefore("function newConversation")

        assertTrue(requestList.indexOf("findSidebarButton(true)") < requestList.indexOf("readConversations()"))
        assertTrue(adapter.contains("Date.now() - stableSince >= 500"))
        assertTrue(adapter.contains("conversations.length > best.length"))
        assertTrue(adapter.contains("direct && isVisible(direct)"))
        assertTrue(adapter.contains("collectConversationHistory"))
        assertTrue(adapter.contains("findConversationScroller"))
        assertTrue(history.contains("scrollRestored"))
        assertTrue(history.contains("stablePassesRequired"))
        assertTrue(history.contains("collected.size >= maximum"))
        assertTrue(
            pageAdapter.indexOf("chatgpt_web_adapter_conversation_history.js") <
                pageAdapter.indexOf("chatgpt_web_adapter_conversations.js"),
        )
    }

    @Test
    fun productionSideMenuUsesStableSelectorsForProjectsAndConversations() {
        val sideMenu = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt",
        )

        assertTrue(sideMenu.contains("ChatGptNativeNavigationSelector.conversation"))
        assertTrue(sideMenu.contains("ChatGptNativeNavigationSelector.project"))
        assertTrue(sideMenu.contains("ChatGptNativeNavigationSelector.NEW_CONVERSATION"))
        assertTrue(sideMenu.contains("ChatGptNativeNavigationSelector.REFRESH_CONVERSATIONS"))
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
