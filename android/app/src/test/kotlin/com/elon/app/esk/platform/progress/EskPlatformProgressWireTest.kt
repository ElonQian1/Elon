package com.elon.app.esk.platform.progress

import com.elon.app.esk.platform.sellback.SellbackPage
import com.elon.app.esk.platform.sellback.SellbackPolicy
import com.elon.app.esk.platform.sellback.SellbackRecord
import com.elon.app.esk.platform.sellback.SellbackSummary
import com.elon.eskcontract.EskPlatformProgressContract as Contract
import org.junit.Assert.*
import org.junit.Test

/** Pure projection and adapter wiring; this is not Android-runtime or real-account acceptance. */
class EskPlatformProgressWireTest {
    private val nonce = "a".repeat(64)
    private val digest = "b".repeat(64)
    private fun record(index: Int = 1, amount: Long = 1_000_000, canceled: Boolean = false) = SellbackRecord(
        "eskpsr_" + index.toString(16).padStart(32, '0'), "private-key-$index", amount, "private-expected", "private-request",
        "private-policy-revision", "private-policy-digest", "private-terms-digest", "2026-09-04T10:00:00Z",
        if (canceled) "2026-09-04T11:00:00Z" else null, if (canceled) "private-cancel-event" else null,
        if (canceled) "canceled" else "submitted",
    )
    private fun page() = SellbackPage(SellbackSummary(digest, 10_000_000, 1_000_000, 9_000_000, 1, 2, true, "enabled",
        SellbackPolicy("private-policy", "private-version", "private-terms-digest", "private-terms", 1, 10_000_000,
            20, 10_000_000, "private-recovery")), listOf(record(2), record(1, canceled = true)), 1, 2, null)
    private fun compose(value: SellbackPage = page(), cursor: String = "", n: String = nonce,
        started: Long = 1000L, observed: Long = 2000L, expires: Long = 62000L) =
        composeEskPlatformProgress(value, n, cursor, started, observed, expires)

    @Test fun projectionUsesExactlyThirtyFivePlusSixPerRowAndNeverLeaksWriteMaterial() {
        val fields = compose()
        assertEquals(47, fields.size)
        assertEquals(Contract.keysForCount(2), fields.keys)
        assertEquals("10.000000", fields["total"])
        assertEquals("1.000000", fields["reserved"])
        assertEquals("9.000000", fields["available"])
        assertEquals("2", fields["request_count"])
        assertEquals("1", fields["open_count"])
        assertEquals("", fields["request_0_canceled_at"])
        assertEquals("canceled", fields["request_1_status"])
        assertTrue(Contract.validSnapshot(fields, nonce, "", 1000, 3000))
        assertFalse(fields.values.any { it.contains("private-") })
        for (name in listOf("token", "user_id", "nickname", "idempotency_key", "policy", "new_requests_enabled", "entry_count"))
            assertFalse(fields.containsKey(name))
        for (name in listOf("funds_moved", "simulated", "external_payment_verified", "submit_request", "cancel_request",
            "service_spending", "quant_subscription", "sellback_settlement", "onchain_transfer", "chain_migration"))
            assertEquals(name, "false", fields[name])
    }

    @Test fun emptyPageIsRealZeroWithNoCursorAndMayHaveSubsecondExpiry() {
        val empty = page().copy(summary = page().summary.copy(total = 0, reserved = 0, available = 0, openCount = 0,
            count = 0, enabled = false, reason = "disabled", policy = null), requests = emptyList(), start = 0, end = 0)
        val fields = compose(empty, expires = 2001)
        assertEquals(35, fields.size)
        assertEquals("0.000000", fields["available"])
        assertEquals("0", fields["page_count"])
        assertEquals("", fields["next_cursor"])
    }

    @Test fun twentyRowsKeepWholeQuotaAndTheContinuationBindsTheExactCursor() {
        val rows = (30 downTo 11).map { record(it) }
        val next = "esbr1.$digest.${rows.last().id}"
        val partial = page().copy(summary = page().summary.copy(total = 40_000_000, reserved = 30_000_000,
            available = 10_000_000, openCount = 30, count = 30), requests = rows, start = 1, end = 20, nextCursor = next)
        val fields = compose(partial)
        assertEquals(155, fields.size)
        assertEquals("40.000000", fields["total"])
        assertEquals(next, fields["next_cursor"])
        val tail = partial.copy(requests = (10 downTo 1).map { record(it) }, start = 21, end = 30, nextCursor = null)
        assertEquals(next, compose(tail, cursor = next)["requested_cursor"])
        assertThrows(IllegalArgumentException::class.java) { compose(tail) }
        assertThrows(IllegalArgumentException::class.java) { compose(tail, cursor = next.replace(digest, "c".repeat(64))) }
    }

    @Test fun rejectsExtraRowsInvalidQuotaRangeStatusesDatesAndDuplicateRecords() {
        val good = page()
        val invalid = listOf(good.copy(requests = List(21) { record(it) }),
            good.copy(summary = good.summary.copy(available = 1)), good.copy(summary = good.summary.copy(total = -1)),
            good.copy(summary = good.summary.copy(openCount = 2)), good.copy(start = 2),
            good.copy(requests = listOf(record(), record())),
            good.copy(requests = listOf(record(2).copy(status = "paid"), record(1, canceled = true))),
            good.copy(requests = listOf(record(2).copy(created = "2026-02-30T10:00:00Z"), record(1, canceled = true))),
            good.copy(requests = listOf(record(2).copy(amount = 0), record(1, canceled = true))))
        invalid.forEach { assertThrows(IllegalArgumentException::class.java) { compose(it) } }
    }

    @Test fun rejectsWrongNonceExpiredWindowAndRetiredProtocolNames() {
        assertThrows(IllegalArgumentException::class.java) { compose(n = "A".repeat(64)) }
        assertThrows(IllegalArgumentException::class.java) { compose(observed = 121000, expires = 181000) }
        assertThrows(IllegalArgumentException::class.java) { compose(expires = 62001) }
        for (protocol in listOf("yilong.esk.android_snapshot.v1", "yilong.esk.platform_android_snapshot.v1")) {
            assertFalse(Contract.validSnapshot(compose() + ("protocol" to protocol), nonce, "", 1000, 3000))
            assertFalse(Contract.validRequest(mapOf("protocol" to protocol, "nonce" to nonce, "cursor" to "")))
        }
    }

    @Test fun latestClockAfterIdentityMustRejectAnExpiredShortSnapshotOrRequestWindow() {
        val short = compose(expires = 2001)
        assertTrue(Contract.validSnapshot(short, nonce, "", 1000, 2000))
        assertFalse(Contract.validSnapshot(short, nonce, "", 1000, 2001))
        val nearDeadline = compose(observed = 120999, expires = 180999)
        assertTrue(Contract.validSnapshot(nearDeadline, nonce, "", 1000, 120999))
        assertFalse(Contract.validSnapshot(nearDeadline, nonce, "", 1000, 121001))
    }

    @Test fun requestAdapterChecksActualStringsAndAnExactUnredirectableEnvelope() {
        val request = EskProgressProviderSources.kotlin("EskPlatformProgressWire.kt").substringBefore("/** Explicit")
        for (marker in listOf("intent.action == EskPlatformProgressContract.ACTION && intent.flags == 0",
            "intent.data == null", "intent.type == null", "intent.clipData == null", "intent.selector == null",
            "intent.sourceBounds == null", "intent.categories.isNullOrEmpty()", "Build.VERSION.SDK_INT >= 29",
            "intent.identifier == null", "ComponentName(ESK_PROGRESS_MAIN_PACKAGE, ESK_PROGRESS_CONSENT_ACTIVITY)",
            "intent.`package` == null || intent.`package` == ESK_PROGRESS_MAIN_PACKAGE", "extras.keySet() == EskPlatformProgressContract.REQUEST_KEYS",
            "extras.get(key)", "value as? String", "EskPlatformProgressContract.MAX_VALUE_LENGTH", "::validRequest", "}.getOrNull()"))
            assertTrue(marker, request.contains(marker))
        assertFalse(request.contains("getString("))
        assertFalse(request.contains("toString()"))
    }

    @Test fun resultAllocatesFreshIntentWithOnlyValidatedIndexedStringWhitelist() {
        val result = EskProgressProviderSources.kotlin("EskPlatformProgressWire.kt").substringAfter("internal fun eskPlatformProgressResult")
        assertTrue(result.indexOf("validSnapshot(fields, nonce, cursor, startedAt, now)") < result.indexOf("val extras = Bundle()"))
        assertTrue(result.contains("keysForCount(count).forEach { extras.putString(it, fields.getValue(it)) }"))
        assertTrue(result.contains("return Intent().putExtras(extras)"))
        for (forbidden in listOf("putParcelable", "putSerializable", "putAll(", "addFlags(", "setData(", "setSelector(", "setComponent(", "token", "userId"))
            assertFalse(forbidden, result.contains(forbidden))
    }
}
