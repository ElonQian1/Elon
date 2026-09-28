package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.*
import org.junit.Test

/** Compatibility and semantic roles replace the retired mandatory metal skin. */
class MobileThemeAssetContractTest {
    @Test fun chatBackgroundsPairWithTheSharedThemeRoles() {
        val ai = source("android/app/src/main/res/drawable/bg_bubble_ai.xml")
        val user = source("android/app/src/main/res/drawable/bg_bubble_user.xml")
        assertTrue(ai.contains("@color/mobile_surface_container"))
        assertTrue(user.contains("@color/mobile_primary_container"))
        assertFalse(ai.contains("<gradient"))
        assertFalse(user.contains("<gradient"))
        assertTrue(source("android/app/src/main/res/layout/item_message_user.xml")
            .contains("@color/mobile_on_primary_container"))
    }

    @Test fun existingThemeAssetRoutesKeepWorkingWithV2AndReducedMotion() {
        val page = source("server/src/assets/web_page.html").substringBefore("</head>")
        val theme = source("server/src/assets/orbital_mobile_theme.css")
        val router = source("server/src/router.rs")
        val web = source("server/src/web.rs")
        assertTrue(page.indexOf("/assets/orbital_mobile_theme.css") > page.indexOf("/assets/project_home.css"))
        assertTrue(source("server/src/assets/web_page.html").contains("data-ui-system=\"mobile-design-v2\""))
        assertTrue(router.contains("/assets/orbital_mobile_theme.css"))
        assertTrue(web.contains("assets/orbital_mobile_theme.css"))
        assertTrue(web.contains("enforce_orbital_mobile_theme"))
        assertTrue(theme.contains("@media (prefers-reduced-motion: reduce)"))
        assertTrue(theme.contains(":focus-visible"))
    }

    private fun source(path: String) = String(Files.readAllBytes(root().resolve(path)), StandardCharsets.UTF_8)
    private fun root(): Path = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath()) { it.parent }
        .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
}
