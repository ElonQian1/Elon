package com.elon.app.mcp

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class McpWebChatProductionContextContractTest {
    @Test
    fun genericContextActionTargetsTheRealMainFriendChatSurface() {
        assertEquals("main", McpNativeControlBridge.targetSurfaceFor("get_web_chat_context"))

        val main = read("android/app/src/main/kotlin/com/elon/app/MainMcpNativeControlActions.kt")
        val catalog = read("android/app/src/main/kotlin/com/elon/app/mcp/McpToolCatalog.kt")
        val capability = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionCapabilityContract.kt")
        assertTrue(main.contains("\"get_web_chat_context\" -> webChatContextJson(args)"))
        assertTrue(main.contains("WebChatProductionContextPager.page("))
        assertTrue(catalog.contains("get_web_chat_context"))
        assertTrue(capability.contains("readAction = \"get_web_chat_context\""))
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
