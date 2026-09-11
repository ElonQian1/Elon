package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebLibrarySourceDiagnosticTest {
    private fun fixture(): JSONObject = javaClass.classLoader!!.getResourceAsStream("webchat/private-library-sources-contract.json")!!
        .bufferedReader().use { JSONObject(it.readText()) }
    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString())

    @Test fun actualCatalogFixtureReachesTheNativeCommandReceipt() {
        assertTrue("library_sources" in ChatGptWebPrivateProtocolEvidence.MODES)
        val value = fixture()
        assertEquals(value.toString(), detail(value))
        val event = ChatGptWebProtocol.parse(JSONObject().put("type", "command_result")
            .put("action", "private_protocol_probe").put("requestId", "mcp_sources")
            .put("ok", true).put("detail", value.toString()).toString()) as ChatGptWebEvent.CommandResult
        assertEquals(value.toString(), event.detail)
    }

    @Test fun unknownEmptyAndCappedCatalogsAreExplicit() {
        val empty = fixture().put("observed", false).put("total", 0).put("groups", JSONArray())
        assertEquals(empty.toString(), detail(empty))
        val capped = fixture().put("total", 500).put("omitted", 497).put("stale", true)
        assertEquals(capped.toString(), detail(capped))
        val image = fixture().apply { getJSONArray("groups").getJSONObject(0).put("artifact", "image_gen") }
        assertEquals(image.toString(), detail(image))
    }

    @Test fun countMismatchDuplicatesAndCoercionAreRejected() {
        val invalid = mutableListOf(fixture().put("total", 4), fixture().put("omitted", -1),
            fixture().put("total", "3"), fixture().put("total", 501), fixture().put("observed", false),
            fixture().put("stale", "false"), fixture().put("groups", JSONArray().put(fixture().getJSONArray("groups").get(0))
                .put(fixture().getJSONArray("groups").get(0))).put("total", 4))
        for (key in listOf("node", "file", "artifact", "mime", "kind")) {
            invalid.add(fixture().apply { getJSONArray("groups").getJSONObject(0).put(key, "private") })
        }
        for (key in listOf("download", "attach", "rename", "trash", "count")) {
            invalid.add(fixture().apply { getJSONArray("groups").getJSONObject(0).put(key, "1") })
        }
        invalid.forEach { assertEquals("invalid_protocol_evidence", detail(it)) }
    }

    @Test fun privateValuesAndUnexpectedFieldsNeverEnterLedger() {
        for (key in listOf("url", "name", "id", "headers", "token", "source")) {
            assertEquals("invalid_protocol_evidence", detail(fixture().put(key, "private")))
            assertEquals("invalid_protocol_evidence", detail(fixture().apply {
                getJSONArray("groups").getJSONObject(0).put(key, "private")
            }))
        }
        for (flags in listOf(JSONArray().put("private-field"), JSONArray().put(JSONObject()),
            JSONArray().put("saved_entity").put("saved_entity"))) {
            assertEquals("invalid_protocol_evidence", detail(fixture().apply {
                getJSONArray("groups").getJSONObject(0).put("flags", flags)
            }))
        }
    }
}
