package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.Rule
import org.junit.rules.TemporaryFolder

class OfficialQuantApkPolicyTest {
    @get:Rule val temporaryDirectory = TemporaryFolder()
    private val officialSigner = setOf(OfficialQuantApkPolicy.SIGNER_SHA256)

    private fun accepted(
        packageName: String? = OfficialQuantApkPolicy.PACKAGE_NAME,
        signers: Set<String> = officialSigner,
        versionCode: Long? = 5L,
    ) = OfficialQuantApkPolicy.accepts(packageName, signers, versionCode)

    @Test fun acceptsNewOnlyBaselineAndNewerVersions() {
        assertTrue(accepted())
        assertTrue(accepted(versionCode = 6L))
    }

    @Test fun rejectsOtherPackagesIncludingDebugAndWhitespace() {
        listOf(null, "", "com.elon.quant.debug", "com.impostor.quant", " com.elon.quant").forEach {
            assertFalse(accepted(packageName = it))
        }
    }

    @Test fun rejectsMissingWrongOrMultipleCurrentSigners() {
        listOf(emptySet(), setOf("0".repeat(64)), officialSigner + "1".repeat(64)).forEach {
            assertFalse(accepted(signers = it))
        }
    }

    @Test fun historicalPinCannotAuthorizeAnUnapprovedCurrentSigner() {
        // Only the current signer set is accepted by the policy; lineage is not an input.
        assertFalse(accepted(signers = setOf("2".repeat(64))))
    }

    @Test fun rejectsUnsupportedOrUnknownVersions() {
        listOf(null, -1L, 0L, 1L, 2L, 3L, 4L).forEach {
            assertFalse(accepted(versionCode = it))
        }
    }

    @Test fun officialProjectSelectionNeverUsesDisplayName() {
        assertTrue(OfficialQuantApkPolicy.appliesTo("yilong-quant"))
        assertTrue(OfficialQuantApkPolicy.appliesTo(" yilong-quant "))
        listOf(null, "", "一龙量化交易", "yilong-quant-copy", "YILONG-QUANT").forEach {
            assertFalse(OfficialQuantApkPolicy.appliesTo(it))
        }
    }

    @Test fun launchIdentityAndCertificateAreStableReviewedValues() {
        assertEquals("com.elon.quant", OfficialQuantApkPolicy.PACKAGE_NAME)
        assertEquals("com.elon.quant.MainActivity", OfficialQuantApkPolicy.ACTIVITY_NAME)
        assertEquals(
            "019a3d95366fb4c6fe578c1f7f26fb96e462dc54f41b9a7c7b5a715052e418bb",
            OfficialQuantApkPolicy.SIGNER_SHA256,
        )
    }

    @Test fun concurrentDownloadsNeverReuseTheVerifiedFile() {
        val first = createOfficialQuantApkFile(temporaryDirectory.root)
        val second = createOfficialQuantApkFile(temporaryDirectory.root)
        assertTrue(first.isFile && second.isFile)
        assertFalse(first.canonicalPath == second.canonicalPath)
        assertEquals("official-quant-apk", first.parentFile.name)
        assertEquals(temporaryDirectory.root.canonicalFile, first.parentFile.parentFile.canonicalFile)
    }

    @Test fun officialIdentityFailureCannotFallThroughToInstallation() {
        val decision = projectApkSignatureDecision(ProjectApkSignatureCompatibility.OFFICIAL_IDENTITY_MISMATCH)
        assertFalse(decision.allowed)
        assertFalse(ProjectApkSignatureInspection(
            ProjectApkSignatureCompatibility.OFFICIAL_IDENTITY_MISMATCH, "com.elon.quant", 4L,
        ).allowed)
    }
}
