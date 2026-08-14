package com.elon.app.googleweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebAdapterContractTest {
    @Test
    fun adapterExposesVisibleChatSemanticsWithoutCredentialsOrPrivateApis() {
        val adapter = read("android/app/src/main/assets/google_web_adapter.js")
        val pageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt",
        )
        val session = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )

        assertTrue(adapter.contains("providerId: 'google_web'"))
        assertTrue(adapter.contains("documentToken"))
        assertTrue(adapter.contains("type: 'message_snapshot'"))
        assertTrue(adapter.contains("action === 'send_prompt'"))
        assertTrue(adapter.contains("action === 'stop_generation'"))
        assertTrue(adapter.contains("action === 'new_conversation'"))
        assertTrue(adapter.contains("MutationObserver"))
        assertTrue(!adapter.contains("document.cookie"))
        assertTrue(!adapter.contains("Authorization"))
        assertTrue(!adapter.contains("fetch("))
        assertTrue(pageAdapter.contains("WEB_MESSAGE_LISTENER"))
        assertTrue(pageAdapter.contains("ALLOWED_ORIGINS"))
        assertTrue(pageAdapter.contains("WebBridgeDocumentSession"))
        assertTrue(pageAdapter.contains("WebBridgeReadinessPolicy.stateAfterPageReady"))
        assertTrue(pageAdapter.contains("ChatGptWebProtocol.parseMessage"))
        assertTrue(session.contains("GoogleWebConversationStore"))
        assertTrue(session.contains("ChatGptWebProxyController"))
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
