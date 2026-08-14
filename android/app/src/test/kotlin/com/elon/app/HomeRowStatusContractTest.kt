package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeRowStatusContractTest {
    @Test
    fun androidAndWebUseTitleBadgesInsteadOfPersistentProjectMarkers() {
        val rows = read("android/app/src/main/kotlin/com/elon/app/MainHomeRows.kt")
        val decorations = read("android/app/src/main/kotlin/com/elon/app/HomeRowStatusDecorations.kt")
        val web = read("server/src/assets/web_page.html")

        assertTrue(rows.contains("friend.isSocialAi() -> HomeRowBadge.AI"))
        assertTrue(rows.contains("showProjectMarker) HomeRowBadge.PROJECT"))
        assertTrue(decorations.contains("text = if (isAi) \"AI\" else \"项目\""))
        assertTrue(web.contains("badge.textContent = item.kind === 'project' ? '项目' : 'AI'"))
        assertFalse(rows.contains("createProjectMarkerIcon"))
        assertFalse(web.contains("className = 'project-marker'"))
    }

    @Test
    fun workingDotExistsOnlyForActiveProjectsAndBreathesByScaling() {
        val rows = read("android/app/src/main/kotlin/com/elon/app/MainHomeRows.kt")
        val decorations = read("android/app/src/main/kotlin/com/elon/app/HomeRowStatusDecorations.kt")
        val web = read("server/src/assets/web_page.html")

        assertTrue(rows.contains("if (projectWorking) {"))
        assertTrue(rows.contains("statusDecorations.createWorkingIndicator()"))
        assertTrue(decorations.contains("setColor(Color.parseColor(\"#F8F7F4\"))"))
        assertTrue(decorations.contains("ValueAnimator.ofFloat(WORKING_DOT_MIN_SCALE, 1f)"))
        assertTrue(web.contains("if (projectWorking) {"))
        assertTrue(web.contains("animation: project-working-breath 900ms ease-in-out infinite alternate"))
        assertTrue(web.contains("from { transform: scale(0.62); }"))
    }

    private fun read(relativePath: String): String {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        val path: Path = generateSequence(cwd) { it.parent }
            .map { it.resolve(relativePath) }
            .take(6)
            .firstOrNull(Files::isRegularFile)
            ?: error("Unable to find $relativePath from $cwd")
        return String(Files.readAllBytes(path), StandardCharsets.UTF_8)
    }
}
