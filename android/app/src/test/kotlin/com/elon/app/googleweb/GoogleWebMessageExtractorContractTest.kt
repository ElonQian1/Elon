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
        val policy = read("android/app/src/main/assets/google_web_answer_candidate_policy.js")
        val queryPolicy = read("android/app/src/main/assets/google_web_query_policy.js")

        assertTrue(source.contains("'[role=\"article\"]'"))
        assertTrue(source.contains("'body [aria-live=\"polite\"]'"))
        assertTrue(source.contains("'body div'"))
        assertTrue(source.contains("rememberQuery"))
        assertTrue(source.contains("queryFound"))
        assertTrue(source.contains("answerFound"))
        assertTrue(source.contains("candidatePolicy.accepts(metrics)"))
        assertTrue(source.contains("candidatePolicy.select(candidates)"))
        assertTrue(source.contains("TRUSTED_ANSWER_SELECTORS"))
        assertTrue(source.contains("'[data-sfc-cp][data-hveid]'"))
        assertTrue(source.contains("'roots=' + responseRootCount"))
        assertTrue(source.contains("rememberedOwned: rememberedQueryOwned"))
        assertTrue(source.contains("const answer = query ? answerCandidate"))
        assertTrue(source.contains("hasQuery: !!query"))
        assertTrue(source.contains("queryPolicy.select"))
        assertTrue(source.contains("currentQueryMatches"))
        assertTrue(source.contains("hasCurrentQuery"))
        assertTrue(queryPolicy.contains("explicitQuery"))
        assertTrue(queryPolicy.contains("rememberedQuery"))
        assertTrue(queryPolicy.contains("urlQuery"))
        assertTrue(!source.contains("main h1, main [role=\"heading\"]"))
        assertTrue(source.contains("[role=\"tablist\"]"))
        assertTrue(policy.contains("links >= 3"))
        assertTrue(policy.contains("textLength < 80"))
        assertTrue(policy.contains("if (!hasQuery"))
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
