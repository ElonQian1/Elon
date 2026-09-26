package com.elon.app.grid.chat

import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceGridReadTest {
    private var clock = 1000L
    private val context = BinanceGridReadStore.Context("owner", "account", "sub", "doc_test")
    private fun request(kind: String = "list", id: String? = null) = BinanceGridReadRequest.parse(
        mapOf("kind" to kind, "request_id" to "request_001", "start" to true) + if (id == null) emptyMap() else mapOf("strategy_id" to id))
    private fun row(id: String = "123") = mapOf("id" to id, "account" to "42", "symbol" to "龙虾USDT",
        "status" to "WORKING", "direction" to "SHORT", "spacing" to "ARITH", "lower" to "0.1032",
        "upper" to "0.1628", "count" to "169", "leverage" to "4", "profit" to "1.2500", "created" to "1788790000000",
        "metrics" to mapOf("investment" to "4029.33780960", "perGridQty" to "377", "fee" to "-92.63435196"))
    private fun reject(action: () -> Unit) { try { action(); fail("accepted invalid input") } catch (_: RuntimeException) {} }

    @Test fun requestsRejectWrongTypesUnknownOperationsAndInvalidPaging() {
        val base = mapOf<String, Any?>("kind" to "list", "request_id" to "request_001")
        for (bad in listOf(mapOf("kind" to "trade"), mapOf("url" to "https://example.com"), mapOf("offset" to 1.0),
            mapOf("limit" to 51), mapOf("start" to "true"), mapOf("strategy_id" to "123"), mapOf("start" to true, "offset" to 1))) {
            reject { BinanceGridReadRequest.parse(base + bad) }
        }
        for (id in listOf("0", "-1", "1e3", "1;fetch()", "123456789012345678901")) reject { request("detail", id) }
        reject { BinanceGridReadRequest.parse(base + mapOf("kind" to "detail", "strategy_id" to "123", "limit" to 1)) }
        assertFalse(BinanceGridReadRequest.parse(base).start)
    }
    @Test fun absentPollingNeverCreatesASnapshot() {
        val store = BinanceGridReadStore { clock }; val q = request().copy(start = false)
        assertEquals("snapshot_not_found", store.reply(q, context)["error"])
        assertFalse(store.contains(q.requestId))
    }
    @Test fun immutablePagesKeepDecimalsAndExcludeIdentityAndExtraFields() {
        val store = BinanceGridReadStore { clock }; val q = request()
        store.start(q); store.complete(q.requestId, context, 99, listOf(row("1"), row("2"), row("3")))
        val first = store.reply(q.copy(limit = 2), context)
        val second = store.reply(q.copy(start = false, offset = 2, limit = 2), context)
        assertEquals(2, first["next_offset"]); assertNull(second["next_offset"])
        assertEquals(first["observed_at_ms"], second["observed_at_ms"])
        val text = StrictJson.encode(first)
        assertFalse(text.contains("account")); assertFalse(text.contains("doc_test")); assertFalse(text.contains("owner"))
        assertTrue(text.contains("4029.33780960")); assertTrue(text.contains("龙虾USDT")); assertTrue(text.contains("\"matchedPnl\":null"))
        assertEquals("invalid_offset", store.reply(q.copy(start = false, offset = 4), context)["error"])
    }
    @Test fun successfulEmptyListIsDistinctFromUnavailable() {
        val store = BinanceGridReadStore { clock }; val q = request()
        store.start(q); store.complete(q.requestId, context, 99, emptyList())
        val result = store.reply(q, context)
        assertEquals("ready", result["status"]); assertEquals(0, result["total"]); assertNull(result["next_offset"])
    }
    @Test fun changedAccountOrDocumentRevokesReadyRowsPermanently() {
        for (changed in listOf(context.copy(account = "another"), context.copy(owner = "other"), context.copy(document = "next"), null)) {
            val store = BinanceGridReadStore { clock }; val q = request()
            store.start(q); store.complete(q.requestId, context, 99, listOf(row()))
            assertEquals("context_changed", store.reply(q, changed)["error"])
            assertFalse(store.reply(q, context).containsKey("rows"))
        }
    }
    @Test fun detailRequiresExactSingleRowAndRequestCannotChangeKind() {
        val store = BinanceGridReadStore { clock }; val q = request("detail", "123")
        store.start(q); reject { store.complete(q.requestId, context, 99, listOf(row("124"))) }
        store.complete(q.requestId, context, 100, listOf(row()))
        assertEquals("request_conflict", store.reply(request(), context)["error"])
        assertTrue(store.reply(q, context).containsKey("row"))
    }
    @Test fun monotonicTtlAndBoundedCapacityDoNotRefreshOnPolling() {
        val store = BinanceGridReadStore { clock }; val q = request()
        repeat(16) { store.start(q.copy(requestId = "request_$it")) }
        reject { store.start(q.copy(requestId = "capacity_17")) }
        clock += 300000
        assertEquals("snapshot_expired", store.reply(q.copy(requestId = "request_0"), context)["error"])
        store.start(q); assertTrue(store.pending(q.requestId))
    }
    @Test fun observationsDistinguishNewListsAndExactDetailWithoutBreakingLegacyRead() {
        val host = BinanceHostState({ clock }, { 1800000000000 })
        fun observe(kind: String, extra: Map<String, Any?>) = host.accept(StrictJson.encode(mapOf("kind" to kind, "account" to "42", "account_kind" to "sub") + extra))
        observe("list", mapOf("rows" to listOf(row())))
        val list = host.listRevision; val grant = host.grant()
        observe("detail", mapOf("row" to row())); assertNotNull(host.detailObservation("123")); assertNull(host.detailObservation("124"))
        assertEquals(list, host.listRevision); assertFalse(host.reply(grant).contains("metrics"))
        observe("list", mapOf("rows" to emptyList<Any>()))
        assertTrue(host.listRevision > list); assertNull(host.detailObservation("123")); assertTrue(host.authorized(grant))
        host.unavailable(); assertTrue(host.snapshotRows().isEmpty())
    }
}
