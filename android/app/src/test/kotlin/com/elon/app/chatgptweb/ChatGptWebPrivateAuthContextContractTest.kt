package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPrivateAuthContextContractTest {
    @Test
    fun prewarmsInsideDocumentAndNeverExportsAuthorization() {
        val auth = read("android/app/src/main/assets/chatgpt_web_private_auth_context.js")
        val conversation = read("android/app/src/main/assets/chatgpt_web_private_transport.js")
        val dictation = read("android/app/src/main/assets/chatgpt_web_private_dictation_transport.js")
        val readAloud = read("android/app/src/main/assets/chatgpt_web_private_read_aloud_transport.js")
        val pageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )

        assertTrue(auth.contains("AUTH_PATH = '/api/auth/session'"))
        assertTrue(auth.contains("credentials: 'include'"))
        assertTrue(auth.contains("cache: 'no-store'"))
        assertTrue(auth.contains("REQUEST_TIMEOUT_MS = 5000"))
        assertTrue(auth.contains("CIRCUIT_COOLDOWN_MS = 45 * 1000"))
        assertTrue(auth.contains("let inFlight = null"))
        assertTrue(auth.contains("copyRequestHeaders"))
        assertFalse(auth.contains("sessionStorage"))
        assertFalse(auth.contains("localStorage"))
        assertFalse(auth.contains("postMessage"))
        assertFalse(auth.contains("console."))

        assertTrue(conversation.contains("authContext.copyRequestHeaders()"))
        assertTrue(conversation.contains("authContext.acquireRequestHeaders()"))
        assertTrue(conversation.contains("authContext.acceptObservedHeaders(entries)"))
        assertTrue(dictation.contains("authContext.acquireRequestHeaders()"))
        assertFalse(dictation.contains("fetchWithTimeout('/api/auth/session'"))
        assertTrue(readAloud.contains("privateTransport.acquireSameOriginRequestHeaders()"))

        assertTrue(pageAdapter.contains("privateAuthContextScript"))
        assertTrue(pageAdapter.contains("\"chatgpt_web_private_json_request.js\", PRIVATE_AUTH_CONTEXT_ASSET"))
        assertTrue(auth.contains("request.request(root, AUTH_PATH"))
        assertTrue(conversation.contains("request.request(window,"))
        assertTrue(pageAdapter.contains("PRIVATE_AUTH_CONTEXT_ASSET"))
        assertTrue(pageAdapter.contains("WebViewCompat.addDocumentStartJavaScript"))
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
