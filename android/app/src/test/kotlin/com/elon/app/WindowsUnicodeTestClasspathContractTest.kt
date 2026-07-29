package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class WindowsUnicodeTestClasspathContractTest {
    @Test
    fun appAutomaticallyEnablesTheWindowsUnicodeTestClasspathCompatibilityLayer() {
        val appBuild = readRepositoryFile("android/app/build.gradle")
        val compatibility = readRepositoryFile(
            "android/gradle/windows-unicode-test-classpath.gradle"
        )

        assertTrue(
            appBuild.contains(
                "apply from: rootProject.file(\"gradle/windows-unicode-test-classpath.gradle\")"
            )
        )
        assertTrue(compatibility.contains("isWindowsHost && containsNonAscii(androidRoot.path)"))
        assertTrue(compatibility.contains("ELON_GRADLE_TEST_ASCII_ROOT"))
        assertTrue(compatibility.contains("originalTestClasses.files + originalClasspath.files") ||
            compatibility.contains("originalClasspath.files + originalTestClasses.files"))
        assertTrue(compatibility.contains("testTask.testClassesDirs = project.files"))
        assertTrue(compatibility.contains("testTask.classpath = project.files"))
        assertTrue(compatibility.contains("unresolvedLocalEntries"))
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
