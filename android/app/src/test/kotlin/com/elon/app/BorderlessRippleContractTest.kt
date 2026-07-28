package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class BorderlessRippleContractTest {
    @Test
    fun androidThemesDisableBorderlessRippleAcrossTheApp() {
        val themes = readRepositoryFile("android/app/src/main/res/values/themes.xml")

        assertEquals(
            2,
            Regex("""<item name="selectableItemBackgroundBorderless">@null</item>""")
                .findAll(themes)
                .count()
        )
        assertEquals(
            2,
            Regex("""<item name="android:selectableItemBackgroundBorderless">@null</item>""")
                .findAll(themes)
                .count()
        )
    }

    @Test
    fun webMirrorDisablesNativeTapHighlightGlobally() {
        val web = readRepositoryFile("server/src/assets/web_page.html")

        assertTrue(
            web.contains(
                """
                * {
                  box-sizing: border-box;
                  -webkit-tap-highlight-color: transparent;
                }
                """.trimIndent()
            )
        )
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
