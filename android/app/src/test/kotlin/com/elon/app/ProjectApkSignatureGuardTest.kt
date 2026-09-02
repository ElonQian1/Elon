package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ProjectApkSignatureGuardTest {
    @Test
    fun allowsFreshInstallWhenArchiveHasSigner() {
        val compatibility = evaluateProjectApkSignatureCompatibility(
            archivePackageName = "com.example.merchant",
            archiveSignerSha256 = setOf("release-a"),
            installedPackageName = null,
            installedSignerSha256 = emptySet(),
        )

        assertEquals(ProjectApkSignatureCompatibility.FRESH_INSTALL, compatibility)
        assertTrue(projectApkSignatureDecision(compatibility).allowed)
    }

    @Test
    fun allowsUpdateWhenSigningLineageOverlaps() {
        val compatibility = evaluateProjectApkSignatureCompatibility(
            archivePackageName = "com.example.merchant",
            archiveSignerSha256 = setOf("release-a", "release-b"),
            installedPackageName = "com.example.merchant",
            installedSignerSha256 = setOf("release-a"),
        )

        assertEquals(ProjectApkSignatureCompatibility.COMPATIBLE_UPDATE, compatibility)
        assertTrue(projectApkSignatureDecision(compatibility).allowed)
    }

    @Test
    fun blocksSamePackageSignedByDifferentCertificate() {
        val compatibility = evaluateProjectApkSignatureCompatibility(
            archivePackageName = "com.example.merchant",
            archiveSignerSha256 = setOf("new-release"),
            installedPackageName = "com.example.merchant",
            installedSignerSha256 = setOf("old-release"),
        )
        val decision = projectApkSignatureDecision(compatibility)

        assertEquals(ProjectApkSignatureCompatibility.SIGNATURE_CONFLICT, compatibility)
        assertFalse(decision.allowed)
        assertTrue(decision.message.contains("卸载"))
        assertTrue(decision.message.contains("本地数据"))
    }

    @Test
    fun blocksArchiveWithoutReadableSigningIdentity() {
        val compatibility = evaluateProjectApkSignatureCompatibility(
            archivePackageName = "com.example.merchant",
            archiveSignerSha256 = emptySet(),
            installedPackageName = null,
            installedSignerSha256 = emptySet(),
        )

        assertEquals(ProjectApkSignatureCompatibility.UNVERIFIABLE, compatibility)
        assertFalse(projectApkSignatureDecision(compatibility).allowed)
    }

    @Test
    fun blocksMismatchedPackageIdentity() {
        val compatibility = evaluateProjectApkSignatureCompatibility(
            archivePackageName = "com.example.merchant",
            archiveSignerSha256 = setOf("release-a"),
            installedPackageName = "com.example.other",
            installedSignerSha256 = setOf("release-a"),
        )

        assertEquals(ProjectApkSignatureCompatibility.UNVERIFIABLE, compatibility)
    }
}
