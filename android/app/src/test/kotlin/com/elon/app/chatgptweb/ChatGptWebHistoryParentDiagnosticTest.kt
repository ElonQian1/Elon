package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebHistoryParentDiagnosticTest {
    private fun fixture(): JSONObject = JSONObject().put("schema", ChatGptWebHistoryParentDiagnostic.SCHEMA)
        .put("code", "observed").put("user_found", true).put("terminal", "null_parent")
        .put("nodes", JSONArray().put(JSONObject().put("role", "user").put("id_kind", "uuid")
            .put("parent_kind", "empty_root").put("node_id_matches", true).put("message_id_matches", true)
            .put("hidden", false).put("content_kind", "text").put("reciprocal", true).put("children", 1)))
    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString())

    @Test fun acceptsBoundedStructuralEvidence() {
        assertTrue("history_parent" in ChatGptWebPrivateProtocolEvidence.MODES)
        assertEquals("observed", JSONObject(detail(fixture())).getString("code"))
    }
    @Test fun acceptsStaticFailuresWithoutNodes() {
        val value = fixture().put("code", "timeout").put("user_found", false)
            .put("nodes", JSONArray()).put("terminal", "not_observed")
        assertEquals("timeout", JSONObject(detail(value)).getString("code"))
    }
    @Test fun acceptsCurrentPaginationDescriptorAndRetainsLegacyShape() {
        val value = fixture().put("pagination", JSONObject().put("present", true).put("complete", true)
            .put("root_owned", true).put("current_leaf_matches", true))
        value.getJSONArray("nodes").getJSONObject(0).put("parent_kind", "paginated_root")
        assertEquals(value.toString(), detail(value))
        assertEquals(fixture().toString(), detail(fixture()))
        for (key in listOf("present", "complete", "root_owned", "current_leaf_matches")) {
            val invalid = JSONObject(value.toString())
            invalid.getJSONObject("pagination").put(key, "true")
            assertEquals("invalid_protocol_evidence", detail(invalid))
        }
        val extra = JSONObject(value.toString())
        extra.getJSONObject("pagination").put("cursor", "private")
        assertEquals("invalid_protocol_evidence", detail(extra))
        val inconsistent = JSONObject(value.toString())
        inconsistent.getJSONObject("pagination").put("present", false)
        assertEquals("invalid_protocol_evidence", detail(inconsistent))
        assertEquals("invalid_protocol_evidence", detail(value.put("pagination", "private")))
    }
    @Test fun rejectsContentAndIdsAtEveryLevel() {
        for (key in listOf("content", "id", "parent", "token")) {
            assertEquals("invalid_protocol_evidence", detail(fixture().put(key, "private")))
            val value = fixture()
            value.getJSONArray("nodes").getJSONObject(0).put(key, "private")
            assertEquals("invalid_protocol_evidence", detail(value))
        }
    }
    @Test fun rejectsUnknownEnums() {
        for (key in listOf("code", "terminal")) {
            assertEquals("invalid_protocol_evidence", detail(fixture().put(key, "private")))
        }
        for (key in listOf("role", "id_kind", "parent_kind", "content_kind")) {
            val value = fixture()
            value.getJSONArray("nodes").getJSONObject(0).put(key, "private")
            assertEquals("invalid_protocol_evidence", detail(value))
        }
    }
    @Test fun rejectsCoercedBooleansAndCounts() {
        for (key in listOf("node_id_matches", "message_id_matches", "hidden", "reciprocal")) {
            val value = fixture()
            value.getJSONArray("nodes").getJSONObject(0).put(key, "true")
            assertEquals("invalid_protocol_evidence", detail(value))
        }
        for (count in listOf<Any>(-1, 4097, 0.5, "1")) {
            val value = fixture()
            value.getJSONArray("nodes").getJSONObject(0).put("children", count)
            assertEquals("invalid_protocol_evidence", detail(value))
        }
    }
    @Test fun rejectsInconsistentObservationAndExcessiveAncestry() {
        for (value in listOf(fixture().put("user_found", false), fixture().put("terminal", "not_observed"),
            fixture().put("code", "timeout"), fixture().put("nodes", JSONArray()))) {
            assertEquals("invalid_protocol_evidence", detail(value))
        }
        val value = fixture()
        repeat(16) { value.getJSONArray("nodes").put(fixture().getJSONArray("nodes").getJSONObject(0)) }
        assertEquals("invalid_protocol_evidence", detail(value))
    }
}
