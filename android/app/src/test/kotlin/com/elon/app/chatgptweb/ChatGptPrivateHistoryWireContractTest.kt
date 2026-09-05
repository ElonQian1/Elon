package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptPrivateHistoryWireContractTest {
    @Test
    fun consumesTheExactPrivateHistoryProducerFixtureWithoutWaitingForDom() {
        val source = requireNotNull(javaClass.classLoader?.getResourceAsStream(
            "webchat/private-history-contract.json",
        )).bufferedReader().use { it.readText() }
        val event = JSONObject(source).getJSONObject("event")
        val payload = JSONObject()
            .put("schema", "yilong.ai.ui.v1")
            .put("event", event)
        val snapshot = (ChatGptWebProtocol.parse(payload.toString()) as ChatGptWebEvent.Snapshot).value

        assertEquals(2, snapshot.messages.size)
        assertEquals("Read the test file.", snapshot.messages[0].content)
        assertEquals("file", snapshot.messages[0].parts.single().type)
        assertEquals("fixture.txt", snapshot.messages[0].parts.single().label)
        assertEquals("text/plain", snapshot.messages[0].parts.single().metadata?.mediaType)
        assertEquals("Reference [Example](https://example.com/source).", snapshot.messages[1].content)
        assertEquals("citation", snapshot.messages[1].parts.single().type)
        assertEquals("example.com", snapshot.messages[1].parts.single().metadata?.targetHost)
        assertTrue(snapshot.authenticated)
        assertTrue(snapshot.contentOnly)
        assertFalse(snapshot.composerReady)
    }
}
