package com.elon.app.update

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUpdateInstallGuardTest {
    @Test
    fun prunesInstalledLatestVersionAndReadySnapshot() {
        val decision = appUpdatePruneDecision(
            installedVersionCode = 864,
            latestVersionCode = 864,
            snapshotVersionCode = 863,
        )

        assertTrue(decision.clearLatestVersion)
        assertTrue(decision.clearSnapshot)
    }

    @Test
    fun prunesOrphanSnapshotWithoutDroppingNewerVersion() {
        val decision = appUpdatePruneDecision(
            installedVersionCode = 864,
            latestVersionCode = 866,
            snapshotVersionCode = 865,
        )

        assertFalse(decision.clearLatestVersion)
        assertTrue(decision.clearSnapshot)
    }

    @Test
    fun keepsMatchingNewerDownloadState() {
        val decision = appUpdatePruneDecision(
            installedVersionCode = 864,
            latestVersionCode = 865,
            snapshotVersionCode = 865,
        )

        assertFalse(decision.changed)
    }

    @Test
    fun blocksArchiveThatWouldDowngradeInstalledApp() {
        val decision = validateAppUpdateArchive(
            expectedPackageName = "com.elon.app",
            installedVersionCode = 864,
            expectedVersionCode = 863,
            archive = AppUpdateArchiveIdentity("com.elon.app", 863),
        )

        assertFalse(decision.allowed)
        assertEquals(AppUpdateInstallBlockReason.NOT_NEWER, decision.blockReason)
    }

    @Test
    fun blocksWrongPackageAndMismatchedTaskVersion() {
        val wrongPackage = validateAppUpdateArchive(
            expectedPackageName = "com.elon.app",
            installedVersionCode = 864,
            expectedVersionCode = 865,
            archive = AppUpdateArchiveIdentity("example.foreign", 865),
        )
        val mismatchedVersion = validateAppUpdateArchive(
            expectedPackageName = "com.elon.app",
            installedVersionCode = 864,
            expectedVersionCode = 866,
            archive = AppUpdateArchiveIdentity("com.elon.app", 865),
        )

        assertEquals(AppUpdateInstallBlockReason.WRONG_PACKAGE, wrongPackage.blockReason)
        assertEquals(AppUpdateInstallBlockReason.VERSION_MISMATCH, mismatchedVersion.blockReason)
    }

    @Test
    fun allowsVerifiedNewerMatchingArchive() {
        val decision = validateAppUpdateArchive(
            expectedPackageName = "com.elon.app",
            installedVersionCode = 864,
            expectedVersionCode = 865,
            archive = AppUpdateArchiveIdentity("com.elon.app", 865),
        )

        assertTrue(decision.allowed)
    }
}
