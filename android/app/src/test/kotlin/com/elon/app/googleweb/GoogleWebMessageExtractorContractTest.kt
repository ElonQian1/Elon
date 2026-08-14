package com.elon.app.googleweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebMessageExtractorContractTest {
    @Test
    fun extractorUsesSemanticFallbacksAndKeepsDiagnosticsContentFree() {
        val source = read("android/app/src/main/assets/google_web_message_extractor.js")

        assertTrue(source.contains("main [role=\"article\"]"))
        assertTrue(source.contains("main [aria-live=\"polite\"]"))
        assertTrue(source.contains("'main div'"))
        assertTrue(source.contains("rememberQuery"))
        assertTrue(source.contains("queryFound"))
        assertTrue(source.contains("answerFound"))
        assertTrue(source.contains(".slice(0, 160)"))
        assertTrue(!source.contains("outerHTML"))
        assertTrue(!source.contains("document.documentElement.innerHTML"))
        assertTrue(!source.contains("document.cookie"))
        assertTrue(!source.contains("sessionStorage"))
        assertTrue(!source.contains("localStorage"))
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
