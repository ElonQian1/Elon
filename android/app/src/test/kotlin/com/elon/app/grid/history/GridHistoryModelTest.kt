package com.elon.app.grid.history

import com.elon.app.grid.chat.BinanceGridReadStore
import com.elon.app.grid.chat.GridChatDraft
import com.elon.app.grid.share.GridShareModel
import com.elon.app.grid.share.GridSharePresentation
import org.junit.Assert.*
import org.junit.Test

class GridHistoryModelTest {
    private val now = 1790928000000L
    private val row = listOf("id", "symbol", "status", "direction", "lower", "upper", "count", "leverage", "profit", "matchedPnl", "fundingFee", "fee", "created", "end", "investment", "initialNotional").associateWith { null as String? } + mapOf("id" to "123", "symbol" to "龙虾USDT", "status" to "CANCELLED",
        "created" to "1790800000000", "end" to "1790900000000", "profit" to "12.00005000", "investment" to "100.00")
    @Test fun chineseIdentityAndExactMoneySurviveWithoutInventedTotals() {
        val fields = GridHistoryModel.project(row, now)
        assertThrows(IllegalArgumentException::class.java) { GridHistoryModel.project(row + ("totalPnl" to "9000"), now) }
        assertEquals("龙虾", GridSharePresentation.token(fields.getValue("symbol")!!))
        assertEquals("12.00005000", fields["profit"])
        assertEquals("UNKNOWN", fields["settlement"])
        listOf("totalPnl", "positionQty", "Cookie").forEach { assertFalse(fields.containsKey(it)) }
        val grid = GridShareModel.project(fields, now)
        assertNull(GridSharePresentation.metric(grid))
        assertFalse(grid.getJSONObject("fields").has("id"))
        assertEquals("龙虾USDT 历史网格", GridShareModel.document(grid).getString("title"))
    }
    @Test fun zeroEndIsUnknownAndImpossibleTimeOrRunningStateIsRejected() {
        assertNull(GridHistoryModel.project(row + ("end" to "0"), now)["end"])
        for (changed in listOf(row + ("end" to "1"), row + ("status" to "WORKING"), row + ("end" to (now + 6000).toString()))) {
            assertThrows(IllegalStateException::class.java) { GridHistoryModel.project(changed, now) }
        }
        assertEquals("0", GridHistoryModel.project(row + ("profit" to "0"), now)["profit"])
    }
    @Test fun oldHistoryRemainsReviewableButSourceAndDraftChangesFail() {
        var elapsed = 1L
        val draft = GridChatDraft { elapsed }
        val source = BinanceGridReadStore.Context("owner-proof", "account-proof", "sub", "document-proof")
        val block = draft.stageHistory("chat", source, now, GridHistoryModel.project(row, now))
        elapsed += 900_000
        assertTrue(draft.historical)
        assertNull(draft.validate("请复盘\n$block", "chat", source))
        assertEquals("context_changed", draft.validate("请复盘\n$block", "chat", source.copy(account = "other")))
        assertEquals("context_changed", draft.validate("请复盘\n$block", "other", source))
        assertEquals("snapshot_modified", draft.validate("请复盘\n" + block.replace("龙虾", "ABC"), "chat", source))
        assertTrue(block.contains("网格利润不等于最终总盈亏"))
        listOf("owner-proof", "account-proof", "document-proof").forEach { assertFalse(block.contains(it)) }
    }
}
