package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatDomObservationLanguageContractTest {
    @Test
    fun transientDomAbsenceIsNeverDescribedAsAnOfficialProductLimitation() {
        productionObservationSources().forEach { relativePath ->
            val source = read(relativePath)
            forbiddenProductClaims.forEach { phrase ->
                assertFalse(
                    "$relativePath must not infer an official capability from an empty DOM: $phrase",
                    source.contains(phrase),
                )
            }
        }
    }

    @Test
    fun productionUsesTheSharedEvidencePolicy() {
        val policy = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionCapabilityEvidence.kt",
        )
        val featureNavigation = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureNavigation.kt",
        )
        val pageActions = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionPageActions.kt",
        )

        assertTrue(policy.contains("TEMPORARILY_UNOBSERVED"))
        assertTrue(policy.contains("ADAPTER_UNSUPPORTED"))
        assertTrue(featureNavigation.contains("WebChatProductionCapabilityEvidencePolicy.resolve"))
        assertTrue(pageActions.contains("WebChatProductionCapabilityEvidencePolicy.resolve"))
    }

    private fun read(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun productionObservationSources(): List<String> {
        val root = repositoryRoot()
        val sourceDirectory = root.resolve("android/app/src/main")
        return Files.walk(sourceDirectory).use { files ->
            files.iterator().asSequence()
                .filter(Files::isRegularFile)
                .filter { it.fileName.toString().substringAfterLast('.', "") in SOURCE_EXTENSIONS }
                .map { root.relativize(it).toString().replace('\\', '/') }
                .toList()
        }
    }

    private fun repositoryRoot(): Path {
        var current = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        repeat(5) {
            if (Files.exists(current.resolve("android/app/src/main"))) return current
            current = current.parent ?: return@repeat
        }
        error("Repository root not found from ${System.getProperty("user.dir")}")
    }

    private companion object {
        val forbiddenProductClaims = listOf(
            "官网没有返回",
            "官网当前未提供",
            "当前网页没有返回",
            "暂不支持从一龙",
        )
        val SOURCE_EXTENSIONS = setOf("kt", "java", "js", "xml")
    }
}
