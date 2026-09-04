package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPrivateReadAloudContractTest {
    @Test
    fun keepsAuthorizationInsideTheIdentityPageAndBoundsPlayback() {
        val transport = read(
            "android/app/src/main/assets/chatgpt_web_private_read_aloud_transport.js",
        )
        val adapter = read("android/app/src/main/assets/chatgpt_web_adapter.js")
        val pageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )
        val backgroundWebView = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundWebViewFactory.kt",
        )
        val backgroundSession = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )

        assertTrue(transport.contains("privateTransport.copySameOriginRequestHeaders()"))
        assertTrue(transport.contains("'/backend-api/synthesize?'"))
        assertTrue(transport.contains("credentials: 'include'"))
        assertTrue(transport.contains("new AbortController()"))
        assertTrue(transport.contains("REQUEST_TIMEOUT_MS = 15000"))
        assertTrue(transport.contains("STREAM_STALL_TIMEOUT_MS = 12000"))
        assertTrue(transport.contains("PLAYBACK_START_TIMEOUT_MS = 8000"))
        assertTrue(transport.contains("response.body.getReader()"))
        assertTrue(transport.contains("new MediaSource()"))
        assertTrue(transport.contains("sourceBuffer.appendBuffer(bytes)"))
        assertTrue(transport.contains("document.documentElement.appendChild(element)"))
        assertFalse(transport.contains("response.blob()"))
        assertTrue(transport.contains("COOLDOWN_MS = 45000"))
        assertTrue(transport.contains("URL.revokeObjectURL(objectUrl)"))
        assertFalse(transport.contains("Cookie"))
        assertFalse(transport.contains("postMessage(JSON.stringify(headers"))
        assertTrue(adapter.contains("privateReadAloudTransport.toggle"))
        assertTrue(adapter.contains("privateReadAloudState"))
        assertTrue(pageAdapter.contains("chatgpt_web_private_read_aloud_transport.js"))
        assertTrue(backgroundWebView.contains(
            "mediaPlaybackRequiresUserGesture = !BuildConfig.CHATGPT_PRIVATE_READ_ALOUD_ENABLED",
        ))
        assertTrue(backgroundSession.contains(
            "latestSnapshot?.privateReadAloudState == \"playing\"",
        ))
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
