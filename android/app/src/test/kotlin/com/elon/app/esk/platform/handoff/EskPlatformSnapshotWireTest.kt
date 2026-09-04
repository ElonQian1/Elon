package com.elon.app.esk.platform.handoff

import com.elon.app.esk.platform.EskPlatformAccount
import com.elon.app.esk.platform.EskPlatformEntry
import com.elon.eskcontract.EskPlatformSnapshotContract
import com.elon.eskcontract.EskSnapshotContract
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.*
import org.junit.Test

/** Pure projection plus source wiring tests; no Android-runtime or device-acceptance claim. */
class EskPlatformSnapshotWireTest {
    private val nonce = "a".repeat(64)
    private fun account() = EskPlatformAccount("10.000000", "10000000", "1", "2026-09-04T10:00:00Z", false,
        listOf(EskPlatformEntry("private-entry-id", "private-allocation-id", "10.000000", "10000000", "2026-09-04T10:00:00Z")))
    private fun compose(value: EskPlatformAccount = account(), requestNonce: String = nonce,
        started: Long = 1000L, observed: Long = 2000L, expires: Long = 62000L) =
        composeEskPlatformSnapshot(value, requestNonce, started, observed, expires)

    @Test fun formalAccountProjectionContainsOnlyTwentyOneStringFields() {
        val fields = compose()
        assertEquals(21, fields.size)
        assertEquals(EskPlatformSnapshotContract.KEYS, fields.keys)
        assertEquals("10.000000", fields["total"])
        assertEquals("10000000", fields["total_base_units"])
        assertEquals("1", fields["entry_count"])
        assertEquals("platform_recorded", fields["source"])
        assertEquals("not_deployed", fields["chain_status"])
        assertEquals("authenticated_operator_review", fields["verification_basis"])
        for (key in listOf("simulated", "funds_moved", "external_payment_verified", "service_spending",
            "quant_subscription", "sellback_settlement", "onchain_transfer", "chain_migration")) {
            assertEquals(key, "false", fields[key])
        }
        assertTrue(EskPlatformSnapshotContract.validSnapshot(fields, nonce, 1000L, 3000L))
    }

    @Test fun accountHistoryIdentityAndLegacyBalanceMeaningsNeverLeaveProjection() {
        val fields = compose()
        for (forbidden in listOf("entries", "entry_id", "allocation_id", "updated_at", "history_has_more",
            "user_id", "nickname", "token", "available", "reserved_total", "reserved_for_quant", "revision")) {
            assertFalse(forbidden, fields.containsKey(forbidden))
        }
        assertFalse(fields.values.any { it.contains("private-entry-id") || it.contains("private-allocation-id") })
        assertFalse(fields.values.any { it.contains("2026-09-04") })
    }

    @Test fun totalComesFromWholeAccountNotCurrentPageSum() {
        val partial = account().copy(total = "20.000000", totalBaseUnits = "20000000", entryCount = "2", historyHasMore = true)
        val fields = compose(partial)
        assertEquals("20.000000", fields["total"])
        assertEquals("2", fields["entry_count"])
    }

    @Test fun explicitEmptyAccountAndShortenedExpiryAreSupported() {
        val empty = EskPlatformAccount("0.000000", "0", "0", null, false, emptyList())
        assertEquals("0.000000", compose(empty)["total"])
        assertEquals("2001", compose(expires = 2001L)["expires_elapsed_ms"])
    }

    @Test fun invalidAmountUnitsAndCountsCannotBeProjected() {
        for (invalid in listOf(account().copy(total = "1e1"), account().copy(total = "10.0"),
            account().copy(totalBaseUnits = "10000001"), account().copy(totalBaseUnits = "-1"),
            account().copy(entryCount = "01"), account().copy(entryCount = "-1"),
            account().copy(total = "9223372036854.775808", totalBaseUnits = "9223372036854775808"))) {
            assertThrows(IllegalArgumentException::class.java) { compose(invalid) }
        }
    }

    @Test fun invalidNonceOrRequestAndDisplayWindowsCannotBeProjected() {
        for (invalid in listOf("", "A".repeat(64), "a".repeat(63), "g".repeat(64))) {
            assertThrows(IllegalArgumentException::class.java) { compose(requestNonce = invalid) }
        }
        for ((started, observed, expires) in listOf(Triple(-1L, 2000L, 62000L), Triple(2001L, 2000L, 62000L),
            Triple(0L, 120000L, 180000L), Triple(1000L, 2000L, 2000L), Triple(1000L, 2000L, 62001L))) {
            assertThrows(IllegalArgumentException::class.java) { compose(started = started, observed = observed, expires = expires) }
        }
    }

    @Test fun formalProjectionIsNeverAcceptedByOriginalPaperProtocol() {
        val fields = compose()
        assertFalse(EskSnapshotContract.validSnapshot(fields, nonce, 1000L, 3000L))
        assertNotEquals(EskSnapshotContract.PROTOCOL, EskPlatformSnapshotContract.PROTOCOL)
        assertNotEquals(EskSnapshotContract.ACTION, EskPlatformSnapshotContract.ACTION)
        assertEquals(17, EskSnapshotContract.KEYS.size)
    }

    @Test fun requestEnvelopeRejectsAlternateRoutingAndNonStringOrExtraFieldsAtRealAdapter() {
        val source = source()
        val request = source.substringBefore("/** Project")
        for (marker in listOf("intent.action == EskPlatformSnapshotContract.ACTION && intent.flags == 0",
            "intent.data == null", "intent.type == null", "intent.clipData == null", "intent.selector == null",
            "intent.sourceBounds == null", "intent.categories.isNullOrEmpty()", "Build.VERSION.SDK_INT >= 29", "intent.identifier == null",
            "intent.component == ComponentName(ESK_PLATFORM_MAIN_PACKAGE, ESK_PLATFORM_CONSENT_ACTIVITY)",
            "intent.`package` == null || intent.`package` == ESK_PLATFORM_MAIN_PACKAGE",
            "extras.keySet() == EskPlatformSnapshotContract.REQUEST_KEYS", "extras.get(key)",
            "value as? String", "it.length <= 128", "fields.takeIf(EskPlatformSnapshotContract::validRequest)",
            "}.getOrNull()")) assertTrue("Missing request boundary: $marker", request.contains(marker))
        assertFalse(request.contains("getString("))
        assertFalse(request.contains("toString()"))
    }

    @Test fun resultUsesNewIntentAndOnlyWhitelistedStringBundleAfterValidation() {
        val result = source().substringAfter("internal fun eskPlatformSnapshotResult")
        val validate = result.indexOf("require(EskPlatformSnapshotContract.validSnapshot(fields, nonce, startedAt, now))")
        val bundle = result.indexOf("val extras = Bundle()")
        val allowlist = result.indexOf("EskPlatformSnapshotContract.KEYS.forEach { extras.putString(it, fields.getValue(it)) }")
        val output = result.indexOf("return Intent().putExtras(extras)")
        assertTrue(validate >= 0 && validate < bundle && bundle < allowlist && allowlist < output)
        for (marker in listOf("putParcelable", "putSerializable", "putAll(", "setData(", "setClipData(",
            "addFlags(", "setSelector(", "setComponent(", "WebView", "token", "userId")) {
            assertFalse("Result acquired extra channel: $marker", result.contains(marker))
        }
    }

    private fun source(): String = String(Files.readAllBytes(root().resolve(
        "android/app/src/main/kotlin/com/elon/app/esk/platform/handoff/EskPlatformSnapshotWire.kt")), StandardCharsets.UTF_8)

    private fun root(): Path = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()) { it.parent }
        .take(6).firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
        ?: error("Repository root unavailable")
}
