package com.elon.app.grid.share

import org.junit.Assert.*
import org.junit.Test

class GridSharePresentationTest {
    private fun grid(vararg fields: Pair<String, String>) = GridShareModel.project(mapOf("symbol" to "TESTUSDT") + fields, 1000)
    @Test fun prefersRealRoiThenTotalThenGridProfitWithoutInferringMissingValues() {
        val data = grid("profit" to "5.000", "totalPnl" to "-2", "roi" to "0")
        assertEquals("roi", GridSharePresentation.metric(data))
        data.getJSONObject("fields").remove("roi"); assertEquals("totalPnl", GridSharePresentation.metric(data))
        data.getJSONObject("fields").remove("totalPnl"); assertEquals("profit", GridSharePresentation.metric(data))
        data.put("show_amounts", false); assertNull(GridSharePresentation.metric(data)); assertNull(GridSharePresentation.raw(data, "profit"))
        assertNull(GridSharePresentation.metric(grid()))
    }
    @Test fun unknownZeroNegativeAndTinyDecimalsStayDistinct() {
        assertEquals("未读取", GridSharePresentation.format(null))
        assertEquals("0", GridSharePresentation.format("0.000", true))
        assertEquals("-12.25", GridSharePresentation.format("-12.2500", true))
        assertEquals("+12.25", GridSharePresentation.format("12.2500", true))
        assertEquals("<0.00000001", GridSharePresentation.format("0.00000000001", true))
        assertNull(GridSharePresentation.number("NaN")); assertNull(GridSharePresentation.number(""))
    }
    @Test fun rangeClampsVisualPositionButKeepsOutOfRangeStatus() {
        val data = grid("lower" to "100", "upper" to "200", "markPrice" to "150")
        assertEquals(.5f, GridSharePresentation.range(data))
        data.getJSONObject("fields").put("markPrice", "250")
        assertEquals(1f, GridSharePresentation.range(data)); assertEquals("高于区间", GridSharePresentation.rangeLabel(data))
        data.getJSONObject("fields").remove("markPrice"); assertNull(GridSharePresentation.range(data))
    }
    @Test fun exactProfitSortingDoesNotTreatMissingAsZero() {
        val rows = listOf(mapOf("symbol" to "UNKNOWN"), mapOf("symbol" to "LOSS", "profit" to "-1"), mapOf("symbol" to "ZERO", "profit" to "0"), mapOf("symbol" to "GAIN", "profit" to "0.0000000000001"))
        assertEquals(listOf("GAIN", "ZERO", "LOSS", "UNKNOWN"), GridSharePresentation.sortRows(rows).map { it["symbol"] })
        assertEquals("QNT", GridSharePresentation.token("QNTUSDT")); assertEquals("?", GridSharePresentation.token("../../QNT"))
    }
}
