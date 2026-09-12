package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebTextBlockInventoryTest {
    private fun sample() = JSONObject().put("schema", ChatGptWebTextBlockInventory.SCHEMA)
        .put("status", "ready").put("bounded", true).put("messages", 1).put("code_blocks", 1)
        .put("writing_blocks", 1).put("writable_blocks", 1).put("metadata_messages", 0).put("unparsed_messages", 0)
        .put("dom_id_matches", 0).put("dom_code_matches", 0).put("dom_writing_matches", 0).put("dom_line_ending_matches", 0)
    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString())

    @Test fun boundedCountOnlyReceiptIsAccepted() {
        val value = sample()
        assertEquals(value.toString(), detail(value))
    }

    @Test fun extraContentMalformedCountsAndErrorWithCountsAreRejected() {
        for (value in listOf(sample().put("content", "private"), sample().put("messages", "1"),
            sample().put("code_blocks", 0.5), sample().put("messages", 81), sample().put("writable_blocks", 2),
            sample().put("metadata_messages", 2), sample().put("writing_blocks", 17), sample().put("bounded", false),
            sample().put("status", "read_failed"))) {
            assertEquals("invalid_protocol_evidence", detail(value))
        }
    }
}
