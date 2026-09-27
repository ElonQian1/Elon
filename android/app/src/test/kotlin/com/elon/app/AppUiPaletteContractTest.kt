package com.elon.app

import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import kotlin.math.pow

/** Test readable roles and a single source, rather than freezing a retired visual style. */
class AppUiPaletteContractTest {
    @Test fun semanticTextPairsHaveEnoughContrastInBothModes() {
        val tokens = JSONObject(read("docs/design/mobile-tokens-v2.json"))
        for (mode in listOf("light", "dark")) {
            val colors = tokens.getJSONObject(mode)
            for (surface in listOf("surface", "surface_container", "surface_container_high")) {
                for (ink in listOf("on_surface", "on_surface_variant", "success", "warning", "error")) {
                    assertTrue("$mode $ink/$surface", contrast(colors.getString(ink), colors.getString(surface)) >= 4.5)
                }
            }
            assertTrue(contrast(colors.getString("on_primary"), colors.getString("primary")) >= 4.5)
            assertTrue(contrast(colors.getString("on_primary_container"), colors.getString("primary_container")) >= 4.5)
        }
    }

    @Test fun pwaDoesNotOverrideTheSharedPaletteWithInlineColors() {
        val web = read("server/src/assets/web_page.html")
        val theme = read("server/src/assets/orbital_mobile_theme.css")
        val inlineRoot = web.substringAfter(":root {").substringBefore("}")
        assertFalse(inlineRoot.contains("--bg:"))
        assertFalse(inlineRoot.contains("--brand:"))
        assertTrue(theme.contains("prefers-color-scheme: dark"))
        assertTrue(theme.contains("--bg: var(--mobile-surface)"))
        assertTrue(web.lastIndexOf("/assets/orbital_mobile_theme.css") > web.lastIndexOf("</style>"))
    }

    private fun contrast(a: String, b: String): Double {
        fun luminance(hex: String): Double {
            val rgb = hex.removePrefix("#").chunked(2).map { it.toInt(16) / 255.0 }
                .map { if (it <= .04045) it / 12.92 else ((it + .055) / 1.055).pow(2.4) }
            return rgb[0] * .2126 + rgb[1] * .7152 + rgb[2] * .0722
        }
        val x = luminance(a); val y = luminance(b)
        return (maxOf(x, y) + .05) / (minOf(x, y) + .05)
    }
    private fun read(path: String): String = String(Files.readAllBytes(root().resolve(path)), Charsets.UTF_8)
    private fun root(): Path = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath()) { it.parent }
        .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
}
