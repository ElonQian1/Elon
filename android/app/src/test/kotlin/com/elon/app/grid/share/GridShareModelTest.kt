package com.elon.app.grid.share

import org.junit.Assert.*
import org.junit.Test

class GridShareModelTest {
    private val row = mapOf("symbol" to "TESTUSDT", "id" to "1234", "account" to "42", "Cookie" to "secret",
        "profit" to "1.234500", "investment" to "1000", "perGridQty" to "4", "direction" to "SHORT",
        "leverage" to "4", "count" to "60", "lower" to "0.1", "upper" to "0.2", "positionQty" to null)
    @Test fun legacyProjectionRemovesHiddenAndPrivateFields() {
        val grid = GridShareModel.project(row, 1000, false)
        val fields = grid.getJSONObject("fields")
        listOf("id", "account", "Cookie", "profit", "investment", "perGridQty", "positionQty").forEach { assertFalse(it, fields.has(it)) }
        assertFalse(grid.has("note")); assertEquals("未公开", GridShareModel.value(grid, "profit"))
        assertEquals("未读取", GridShareModel.value(grid, "markPrice"))
    }
    @Test fun defaultAmountsKeepExactPrecisionAndUnknownIsNotZero() {
        val grid = GridShareModel.project(row, 1000)
        assertTrue(grid.getBoolean("show_amounts"))
        assertEquals("1.234500", GridShareModel.value(grid, "profit"))
        assertEquals("未读取", GridShareModel.value(grid, "positionQty"))
        assertFalse(grid.getJSONObject("fields").has("account"))
        assertFalse(grid.getJSONObject("fields").has("Cookie"))
        assertFalse(grid.getJSONObject("fields").has("id"))
        assertEquals("1000", GridShareModel.value(grid, "investment"))
        assertEquals("4", GridShareModel.value(grid, "perGridQty"))
    }
    @Test fun publicContainerHasCanonicalTitleSummaryAndImmutablePrevious() {
        val grid = GridShareModel.project(row, 1000, false, "我的说明", "ai_snapshot_previous")
        val document = GridShareModel.document(grid)
        assertEquals("TESTUSDT 网格快照", document.getString("title"))
        assertEquals("做空 · 4× · 0.1–0.2 · 60 格 · 历史快照", document.getString("summary"))
        assertEquals("ai_snapshot_previous", document.getJSONObject("grid").getString("previous_snapshot_id"))
        assertEquals(0, document.getJSONArray("messages").length())
    }
}
