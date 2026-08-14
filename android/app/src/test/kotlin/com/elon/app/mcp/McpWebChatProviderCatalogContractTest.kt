package com.elon.app.mcp

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class McpWebChatProviderCatalogContractTest {
    @Test
    fun advertisesBothWebChatProvidersAndOpaqueNavigationPaths() {
        val catalog = read("android/app/src/main/kotlin/com/elon/app/mcp/McpToolCatalog.kt")

        assertTrue(catalog.contains("chatgpt_web or google_web"))
        assertTrue(catalog.contains("Provider-scoped path returned by get_web_chat_navigation"))
        assertTrue(catalog.contains("treat it as opaque"))
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
