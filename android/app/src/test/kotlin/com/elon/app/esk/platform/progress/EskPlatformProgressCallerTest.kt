package com.elon.app.esk.platform.progress

import com.elon.app.OfficialQuantApkPolicy
import java.nio.file.Files
import java.nio.file.Paths
import org.junit.Assert.*
import org.junit.Test

class EskPlatformProgressCallerTest {
    private val pin = setOf(OfficialQuantApkPolicy.SIGNER_SHA256)
    private fun accepts(pkg: String? = "com.elon.quant", activity: String? = ESK_PROGRESS_QUANT_ACTIVITY,
        signers: Set<String> = pin, version: Long? = 5L, enabled: Boolean = true, alias: String? = null) =
        acceptsEskPlatformProgressCaller(pkg, activity, signers, version, enabled, alias)

    @Test fun acceptsOnlyTheNewOfficialProgressComponentAtVersionFiveOrLater() {
        assertTrue(accepts())
        assertTrue(accepts(version = Long.MAX_VALUE))
        assertEquals("com.elon.app", ESK_PROGRESS_MAIN_PACKAGE)
        assertEquals("com.elon.app.esk.platform.progress.EskPlatformProgressConsentActivity", ESK_PROGRESS_CONSENT_ACTIVITY)
        assertEquals("com.elon.quant.assets.progress.EskPlatformProgressActivity", ESK_PROGRESS_QUANT_ACTIVITY)
        for (version in listOf(null, -1L, 0L, 3L, 4L)) assertFalse(accepts(version = version))
    }

    @Test fun rejectsMissingDebugOldPaperAndOldFormalCallers() {
        for (pkg in listOf(null, "", "com.elon.quant.debug", "com.elon.app", "com.elon.quant ")) assertFalse(accepts(pkg = pkg))
        for (activity in listOf(null, "", "com.elon.quant.MainActivity", "com.elon.quant.assets.EskAssetsActivity",
            "com.elon.quant.assets.platform.EskPlatformAssetsActivity", "$ESK_PROGRESS_QUANT_ACTIVITY ")) {
            assertFalse(accepts(activity = activity))
        }
    }

    @Test fun rejectsAbsentWrongMultipleHistoricalOrAliasedIdentity() {
        for (signers in listOf(emptySet(), setOf("old-certificate"), pin + "another", setOf(pin.single().uppercase())))
            assertFalse(accepts(signers = signers))
        assertFalse(accepts(enabled = false))
        assertFalse(accepts(alias = ""))
        assertFalse(accepts(alias = ESK_PROGRESS_QUANT_ACTIVITY))
    }

    @Test fun adapterUsesOsIdentityCurrentSignersAndEnabledComponentNotIntentClaims() {
        val source = EskProgressProviderSources.kotlin("EskPlatformProgressCaller.kt")
        for (text in listOf("callingPackage != OfficialQuantApkPolicy.PACKAGE_NAME", "val caller = callingActivity ?: return false",
            "caller.packageName != callingPackage", "caller.className != ESK_PROGRESS_QUANT_ACTIVITY",
            "readInstalledPackageInfo(packageManager, caller.packageName)", "currentPackageSignerSha256(installed)",
            "getActivityInfo(caller, PackageManager.MATCH_DISABLED_COMPONENTS)", "getComponentEnabledSetting(caller)",
            "getApplicationEnabledSetting(caller.packageName)", "activity.applicationInfo.enabled", "activity.targetActivity",
            "componentSetting in permittedSettings", "appSetting in permittedSettings", "}.getOrDefault(false)"))
            assertTrue(text, source.contains(text))
        for (text in listOf("signingCertificateHistory", "intent.", "getStringExtra", "getParcelableExtra"))
            assertFalse(text, source.contains(text))
    }
}

internal object EskProgressProviderSources {
    fun kotlin(file: String) = read("android/app/src/main/kotlin/com/elon/app/esk/platform/progress/$file")
    fun read(relative: String): String {
        val root = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()) { it.parent }
            .take(6).firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Repository root unavailable")
        return String(Files.readAllBytes(root.resolve(relative)), Charsets.UTF_8)
    }
}
