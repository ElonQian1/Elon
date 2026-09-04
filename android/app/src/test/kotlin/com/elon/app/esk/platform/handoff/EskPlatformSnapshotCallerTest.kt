package com.elon.app.esk.platform.handoff

import com.elon.app.OfficialQuantApkPolicy
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.*
import org.junit.Test

class EskPlatformSnapshotCallerTest {
    private val pin = setOf(OfficialQuantApkPolicy.SIGNER_SHA256)
    private fun accepts(packageName: String? = "com.elon.quant", activityName: String? = ESK_PLATFORM_QUANT_ASSETS_ACTIVITY,
        signers: Set<String> = pin, version: Long? = 4L, enabled: Boolean = true, aliasTarget: String? = null): Boolean =
        acceptsEskPlatformSnapshotCaller(packageName, activityName, signers, version, enabled, aliasTarget)

    @Test fun exactOfficialFormalComponentAtMinimumOrLaterVersionIsAccepted() {
        assertTrue(accepts())
        assertTrue(accepts(version = 5L))
        assertEquals("com.elon.app", ESK_PLATFORM_MAIN_PACKAGE)
        assertEquals("com.elon.app.esk.platform.handoff.EskPlatformSnapshotConsentActivity", ESK_PLATFORM_CONSENT_ACTIVITY)
        assertEquals("com.elon.quant.assets.platform.EskPlatformAssetsActivity", ESK_PLATFORM_QUANT_ASSETS_ACTIVITY)
    }

    @Test fun missingDebugAndLookalikePackagesCannotCall() {
        for (name in listOf(null, "", "com.elon.quant.debug", "com.elon.quant ", "COM.ELON.QUANT", "com.elon.app")) {
            assertFalse(name, accepts(packageName = name))
        }
    }

    @Test fun oldPaperMainAndUnrelatedActivitiesCannotRequestFormalSummary() {
        for (name in listOf(null, "", "com.elon.quant.MainActivity", "com.elon.quant.assets.EskAssetsActivity",
            "$ESK_PLATFORM_QUANT_ASSETS_ACTIVITY ", "com.other.EskPlatformAssetsActivity")) {
            assertFalse(name, accepts(activityName = name))
        }
    }

    @Test fun absentWrongHistoricalOrAdditionalCurrentSignersFailClosed() {
        for (signers in listOf(emptySet(), setOf("wrong-current-signer"), pin + "another-current-signer",
            setOf(OfficialQuantApkPolicy.SIGNER_SHA256.uppercase()))) {
            assertFalse(accepts(signers = signers))
        }
    }

    @Test fun disabledAliasedAndOldVersionComponentsFailClosed() {
        for (version in listOf(null, -1L, 0L, 2L, 3L)) assertFalse(accepts(version = version))
        assertFalse(accepts(enabled = false))
        assertFalse(accepts(aliasTarget = ESK_PLATFORM_QUANT_ASSETS_ACTIVITY))
        assertFalse(accepts(aliasTarget = ""))
    }

    @Test fun productionAdapterChecksOsCallerAndCurrentSignerWithoutTrustingIntentFields() {
        val source = source("EskPlatformSnapshotCaller.kt")
        for (marker in listOf("callingPackage != OfficialQuantApkPolicy.PACKAGE_NAME",
            "val caller = callingActivity ?: return false", "caller.packageName != callingPackage",
            "caller.className != ESK_PLATFORM_QUANT_ASSETS_ACTIVITY",
            "readInstalledPackageInfo(packageManager, caller.packageName)",
            "getActivityInfo(caller, PackageManager.MATCH_DISABLED_COMPONENTS)",
            "getComponentEnabledSetting(caller)", "getApplicationEnabledSetting(caller.packageName)",
            "currentPackageSignerSha256(installed)", "activity.applicationInfo.enabled",
            "activity.packageName == caller.packageName", "componentSetting in permittedSettings",
            "appSetting in permittedSettings", "activity.targetActivity", "}.getOrDefault(false)")) {
            assertTrue("Missing OS boundary: $marker", source.contains(marker))
        }
        for (marker in listOf("signingCertificateHistory", "getStringExtra(", "getParcelableExtra(", "intent.")) {
            assertFalse("Unexpected caller identity input: $marker", source.contains(marker))
        }
    }

    private fun source(file: String): String = String(Files.readAllBytes(root().resolve(
        "android/app/src/main/kotlin/com/elon/app/esk/platform/handoff/$file")), StandardCharsets.UTF_8)

    private fun root(): Path = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()) { it.parent }
        .take(6).firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
        ?: error("Repository root unavailable")
}
