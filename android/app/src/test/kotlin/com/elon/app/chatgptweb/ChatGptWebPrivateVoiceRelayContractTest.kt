package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPrivateVoiceRelayContractTest {
    private val offer = "v=0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\n"

    @Test
    fun buildsOnlyBoundedResearchRelayScripts() {
        val start = ChatGptWebPrivateVoiceRelayContract.startScript("relay_12345678", offer)
        val poll = ChatGptWebPrivateVoiceRelayContract.pollScript("relay_12345678")

        assertNotNull(start)
        assertNotNull(poll)
        assertTrue(start!!.contains("startExchange"))
        assertTrue(poll!!.contains("takeResult"))
        assertNull(ChatGptWebPrivateVoiceRelayContract.startScript("bad", offer))
        assertNull(ChatGptWebPrivateVoiceRelayContract.startScript("relay_12345678", "bad"))
    }

    @Test
    fun parsesAnswerWithoutExposingItThroughToString() {
        val payload = JSONObject()
            .put("status", "ok")
            .put("answer", offer)
            .toString()
        val raw = JSONObject.quote(payload)
        val result = ChatGptWebPrivateVoiceRelayContract.parsePoll(raw)

        assertTrue(result is ChatGptWebPrivateVoiceRelayPoll.Ready)
        val answer = (result as ChatGptWebPrivateVoiceRelayPoll.Ready).answer
        assertEquals(offer, answer.value())
        assertFalse(answer.toString().contains(offer))
        assertEquals(
            ChatGptWebPrivateVoiceRelayPoll.Pending,
            ChatGptWebPrivateVoiceRelayContract.parsePoll("null"),
        )
    }

    @Test
    fun collapsesUnknownFailuresAndRejectsMalformedAnswers() {
        assertEquals(
            ChatGptWebPrivateVoiceRelayPoll.Failed("relay_failed"),
            ChatGptWebPrivateVoiceRelayContract.parsePoll(
                JSONObject.quote("""{"status":"failed","code":"private-upstream-detail"}"""),
            ),
        )
        assertEquals(
            ChatGptWebPrivateVoiceRelayPoll.Failed("invalid_answer"),
            ChatGptWebPrivateVoiceRelayContract.parsePoll(
                JSONObject.quote("""{"status":"ok","answer":"not-sdp"}"""),
            ),
        )
    }
}
