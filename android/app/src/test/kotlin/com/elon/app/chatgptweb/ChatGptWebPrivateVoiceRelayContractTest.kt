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
        val bootstrap = ChatGptWebPrivateVoiceRelayContract.bootstrapScript()
        val arm = ChatGptWebPrivateVoiceRelayContract.armScript("relay_12345678", offer)
        val cancel = ChatGptWebPrivateVoiceRelayContract.cancelScript("relay_12345678")
        val poll = ChatGptWebPrivateVoiceRelayContract.pollScript("relay_12345678")

        assertNotNull(arm)
        assertNotNull(cancel)
        assertNotNull(poll)
        assertTrue(bootstrap.contains("bootstrap"))
        assertTrue(arm!!.contains("armExchange"))
        assertTrue(cancel!!.contains("cancelExchange"))
        assertTrue(poll!!.contains("takeResult"))
        assertTrue(
            ChatGptWebPrivateVoiceRelayContract.setOfficialMediaEnabledScript(false)
                .contains("setOfficialMediaEnabled(false)"),
        )
        assertTrue(
            ChatGptWebPrivateVoiceRelayContract.closeOfficialPeerScript()
                .contains("closeOfficialPeer()"),
        )
        assertTrue(
            ChatGptWebPrivateVoiceRelayContract.resetTakeoverScript()
                .contains("resetTakeover()"),
        )
        assertNull(ChatGptWebPrivateVoiceRelayContract.armScript("bad", offer))
        assertNull(ChatGptWebPrivateVoiceRelayContract.armScript("relay_12345678", "bad"))
    }

    @Test
    fun parsesOnlyVersionedAtomicArmReceipts() {
        assertEquals(
            ChatGptWebPrivateVoiceRelayArm.Accepted,
            ChatGptWebPrivateVoiceRelayContract.parseArm(
                JSONObject.quote("""{"version":4,"armed":true,"code":null}"""),
            ),
        )
        assertEquals(
            ChatGptWebPrivateVoiceRelayArm.Rejected("busy"),
            ChatGptWebPrivateVoiceRelayContract.parseArm(
                JSONObject.quote("""{"version":4,"armed":false,"code":"busy"}"""),
            ),
        )
        assertEquals(
            ChatGptWebPrivateVoiceRelayArm.Rejected("relay_failed"),
            ChatGptWebPrivateVoiceRelayContract.parseArm(
                JSONObject.quote("""{"version":4,"armed":false,"code":"private"}"""),
            ),
        )
    }

    @Test
    fun parsesOnlyStructuralOfficialMediaOwnershipResults() {
        val applied = JSONObject.quote(
            JSONObject()
                .put("version", 4)
                .put("applied", true)
                .put("enabled", false)
                .put("senderTracks", 1)
                .put("receiverTracks", 1)
                .put("closed", false)
                .toString(),
        )
        assertEquals(
            ChatGptWebPrivateVoiceMediaControl.Applied(
                enabled = false,
                senderTracks = 1,
                receiverTracks = 1,
                closed = false,
            ),
            ChatGptWebPrivateVoiceRelayContract.parseMediaControl(applied),
        )
        assertEquals(
            ChatGptWebPrivateVoiceMediaControl.Unavailable("peer_unavailable"),
            ChatGptWebPrivateVoiceRelayContract.parseMediaControl(
                JSONObject.quote(
                    """{"version":4,"applied":false,"code":"peer_unavailable"}""",
                ),
            ),
        )
    }

    @Test
    fun parsesOnlyBoundedNonSecretDataChannelHints() {
        val payload = JSONObject()
            .put("version", 4)
            .put("available", true)
            .put("templateState", "ready")
            .put("dataChannelState", "ready")
            .put(
                "dataChannel",
                JSONObject()
                    .put("label", "oai-events")
                    .put("ordered", true)
                    .put("maxRetransmits", JSONObject.NULL)
                    .put("protocol", "")
                    .put("negotiated", false)
                    .put("id", JSONObject.NULL),
            )
        val result = ChatGptWebPrivateVoiceRelayContract.parseBootstrap(
            JSONObject.quote(payload.toString()),
        )

        assertEquals(
            ChatGptWebPrivateVoiceBootstrap.Ready(
                ChatGptWebPrivateVoiceDataChannelHint(
                    label = "oai-events",
                    ordered = true,
                    maxRetransmits = null,
                    protocol = "",
                    negotiated = false,
                    id = null,
                ),
            ),
            result,
        )
        val emptyLabel = JSONObject(payload.toString())
            .put(
                "dataChannel",
                JSONObject(payload.getJSONObject("dataChannel").toString()).put("label", ""),
            )
        assertEquals(
            ChatGptWebPrivateVoiceBootstrap.Ready(
                ChatGptWebPrivateVoiceDataChannelHint(
                    label = "",
                    ordered = true,
                    maxRetransmits = null,
                    protocol = "",
                    negotiated = false,
                    id = null,
                ),
            ),
            ChatGptWebPrivateVoiceRelayContract.parseBootstrap(
                JSONObject.quote(emptyLabel.toString()),
            ),
        )
        val invalid = JSONObject(payload.toString())
            .put("dataChannel", JSONObject().put("label", "\nprivate"))
        assertEquals(
            ChatGptWebPrivateVoiceBootstrap.Unavailable("invalid_data_channel"),
            ChatGptWebPrivateVoiceRelayContract.parseBootstrap(
                JSONObject.quote(invalid.toString()),
            ),
        )
    }

    @Test
    fun acceptsColdStartPresetWithoutAConsumedRequestTemplate() {
        val payload = JSONObject()
            .put("version", 4)
            .put("available", true)
            .put("templateState", "missing")
            .put("dataChannelState", "preset")
            .put(
                "dataChannel",
                JSONObject()
                    .put("label", "")
                    .put("ordered", true)
                    .put("maxRetransmits", JSONObject.NULL)
                    .put("protocol", "")
                    .put("negotiated", false)
                    .put("id", JSONObject.NULL),
            )

        assertTrue(
            ChatGptWebPrivateVoiceRelayContract.parseBootstrap(
                JSONObject.quote(payload.toString()),
            ) is ChatGptWebPrivateVoiceBootstrap.Ready,
        )
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
