package com.elon.app.chatgptweb

import org.json.JSONObject
import org.json.JSONTokener

internal class ChatGptWebPrivateVoiceAnswer internal constructor(
    private val value: String,
) {
    internal fun value(): String = value

    override fun toString(): String = "ChatGptWebPrivateVoiceAnswer(<redacted>)"
}

internal data class ChatGptWebPrivateVoiceDataChannelHint(
    val label: String,
    val ordered: Boolean,
    val maxRetransmits: Int?,
    val protocol: String,
    val negotiated: Boolean,
    val id: Int?,
)

internal sealed interface ChatGptWebPrivateVoiceBootstrap {
    data class Ready(
        val dataChannel: ChatGptWebPrivateVoiceDataChannelHint,
    ) : ChatGptWebPrivateVoiceBootstrap

    data class Unavailable(
        val code: String,
    ) : ChatGptWebPrivateVoiceBootstrap
}

internal sealed interface ChatGptWebPrivateVoiceRelayPoll {
    data object Pending : ChatGptWebPrivateVoiceRelayPoll

    data class Ready(
        val answer: ChatGptWebPrivateVoiceAnswer,
    ) : ChatGptWebPrivateVoiceRelayPoll

    data class Failed(
        val code: String,
    ) : ChatGptWebPrivateVoiceRelayPoll
}

internal sealed interface ChatGptWebPrivateVoiceRelayArm {
    data object Accepted : ChatGptWebPrivateVoiceRelayArm

    data class Rejected(
        val code: String,
    ) : ChatGptWebPrivateVoiceRelayArm
}

internal sealed interface ChatGptWebPrivateVoiceMediaControl {
    data class Applied(
        val enabled: Boolean,
        val senderTracks: Int,
        val receiverTracks: Int,
        val closed: Boolean,
    ) : ChatGptWebPrivateVoiceMediaControl

    data class Unavailable(
        val code: String,
    ) : ChatGptWebPrivateVoiceMediaControl
}

internal object ChatGptWebPrivateVoiceRelayContract {
    private const val RELAY_OBJECT = "window.__elonChatGptPrivateVoiceRelay"
    private const val MAX_OFFER_LENGTH = 240_000
    private const val MAX_ANSWER_LENGTH = 320_000
    private val REQUEST_ID = Regex("^relay_[a-z0-9]{8,32}$")
    private val SAFE_FAILURES = setOf(
        "busy",
        "invalid_answer",
        "invalid_offer",
        "invalid_request",
        "network_error",
        "official_start_unavailable",
        "template_consumed",
        "template_expired",
        "template_unavailable",
        "timeout",
        "upstream_rejected",
    )

    fun validOffer(value: String): Boolean =
        value.length in 16..MAX_OFFER_LENGTH && isAudioSdp(value)

    fun bootstrapScript(): String =
        """
            (function () {
              var relay = $RELAY_OBJECT;
              return relay && relay.version >= 4
                ? relay.bootstrap()
                : JSON.stringify({version: 4, available: false, templateState: "missing", dataChannelState: "missing"});
            })();
        """.trimIndent()

    fun parseBootstrap(rawEvaluateValue: String?): ChatGptWebPrivateVoiceBootstrap {
        val value = parseEvaluateObject(rawEvaluateValue)
            ?: return ChatGptWebPrivateVoiceBootstrap.Unavailable("malformed_bootstrap")
        if (value.optInt("version") < 4) {
            return ChatGptWebPrivateVoiceBootstrap.Unavailable("unsupported_relay")
        }
        if (!value.optBoolean("available")) {
            val code = when {
                value.optBoolean("armed") || value.optBoolean("inFlight") -> "busy"
                value.optString("templateState") == "consumed" -> "template_consumed"
                value.optString("templateState") == "expired" -> "template_expired"
                value.optString("dataChannelState") == "expired" -> "data_channel_expired"
                value.optString("dataChannelState") !in setOf("ready", "preset") ->
                    "data_channel_unavailable"
                else -> "template_unavailable"
            }
            return ChatGptWebPrivateVoiceBootstrap.Unavailable(code)
        }
        val dataChannel = value.optJSONObject("dataChannel")
            ?: return ChatGptWebPrivateVoiceBootstrap.Unavailable("data_channel_unavailable")
        val label = dataChannel.optString("label")
        val protocol = dataChannel.optString("protocol")
        val maxRetransmits = dataChannel.optNullableInt("maxRetransmits", 0..65_535)
        val id = dataChannel.optNullableInt("id", 0..65_534)
        if (!validDataChannelLabel(label) || (protocol.isNotEmpty() && !validToken(protocol, 64))) {
            return ChatGptWebPrivateVoiceBootstrap.Unavailable("invalid_data_channel")
        }
        if (
            dataChannel.has("maxRetransmits") && !dataChannel.isNull("maxRetransmits") && maxRetransmits == null ||
            dataChannel.has("id") && !dataChannel.isNull("id") && id == null
        ) {
            return ChatGptWebPrivateVoiceBootstrap.Unavailable("invalid_data_channel")
        }
        return ChatGptWebPrivateVoiceBootstrap.Ready(
            ChatGptWebPrivateVoiceDataChannelHint(
                label = label,
                ordered = dataChannel.optBoolean("ordered", true),
                maxRetransmits = maxRetransmits,
                protocol = protocol,
                negotiated = dataChannel.optBoolean("negotiated", false),
                id = id,
            ),
        )
    }

    fun armScript(requestId: String, offer: String): String? {
        if (!REQUEST_ID.matches(requestId) || !validOffer(offer)) return null
        return """
            (function () {
              var relay = $RELAY_OBJECT;
              return relay && relay.version >= 4 && typeof relay.armExchange === "function"
                ? relay.armExchange(${JSONObject.quote(requestId)}, ${JSONObject.quote(offer)})
                : JSON.stringify({version: 4, armed: false, code: "unsupported_relay"});
            })();
        """.trimIndent()
    }

    fun cancelScript(requestId: String): String? {
        if (!REQUEST_ID.matches(requestId)) return null
        return """
            (function () {
              var relay = $RELAY_OBJECT;
              return Boolean(relay && relay.version >= 4 && relay.cancelExchange(${JSONObject.quote(requestId)}));
            })();
        """.trimIndent()
    }

    fun pollScript(requestId: String): String? {
        if (!REQUEST_ID.matches(requestId)) return null
        return """
            (function () {
              var relay = $RELAY_OBJECT;
              return relay && relay.version >= 4
                ? relay.takeResult(${JSONObject.quote(requestId)})
                : JSON.stringify({status: "failed", code: "template_unavailable"});
            })();
        """.trimIndent()
    }

    fun setOfficialMediaEnabledScript(enabled: Boolean): String =
        mediaControlScript("setOfficialMediaEnabled(${enabled.toString()})")

    fun closeOfficialPeerScript(): String = mediaControlScript("closeOfficialPeer()")

    fun resetTakeoverScript(): String = mediaControlScript("resetTakeover()")

    fun parseArm(rawEvaluateValue: String?): ChatGptWebPrivateVoiceRelayArm {
        val value = parseEvaluateObject(rawEvaluateValue)
            ?: return ChatGptWebPrivateVoiceRelayArm.Rejected("malformed_result")
        if (value.optInt("version") < 4) {
            return ChatGptWebPrivateVoiceRelayArm.Rejected("unsupported_relay")
        }
        if (value.optBoolean("armed")) return ChatGptWebPrivateVoiceRelayArm.Accepted
        return ChatGptWebPrivateVoiceRelayArm.Rejected(
            value.optString("code").takeIf(SAFE_FAILURES::contains) ?: "relay_failed",
        )
    }

    fun parseMediaControl(rawEvaluateValue: String?): ChatGptWebPrivateVoiceMediaControl {
        val value = parseEvaluateObject(rawEvaluateValue)
            ?: return ChatGptWebPrivateVoiceMediaControl.Unavailable("malformed_result")
        if (value.optInt("version") < 3) {
            return ChatGptWebPrivateVoiceMediaControl.Unavailable("unsupported_relay")
        }
        if (!value.optBoolean("applied")) {
            val code = value.optString("code").takeIf {
                it == "peer_unavailable"
            } ?: "operation_failed"
            return ChatGptWebPrivateVoiceMediaControl.Unavailable(code)
        }
        val senderTracks = value.optNullableInt("senderTracks", 0..16)
            ?: return ChatGptWebPrivateVoiceMediaControl.Unavailable("malformed_result")
        val receiverTracks = value.optNullableInt("receiverTracks", 0..16)
            ?: return ChatGptWebPrivateVoiceMediaControl.Unavailable("malformed_result")
        return ChatGptWebPrivateVoiceMediaControl.Applied(
            enabled = value.optBoolean("enabled"),
            senderTracks = senderTracks,
            receiverTracks = receiverTracks,
            closed = value.optBoolean("closed"),
        )
    }

    fun parsePoll(rawEvaluateValue: String?): ChatGptWebPrivateVoiceRelayPoll {
        if (rawEvaluateValue.isNullOrBlank() || rawEvaluateValue == "null") {
            return ChatGptWebPrivateVoiceRelayPoll.Pending
        }
        val value = parseEvaluateObject(rawEvaluateValue)
            ?: return ChatGptWebPrivateVoiceRelayPoll.Failed("malformed_result")
        return when (value.optString("status")) {
            "ok" -> {
                val answer = value.optString("answer")
                if (answer.length !in 16..MAX_ANSWER_LENGTH || !isAudioSdp(answer)) {
                    ChatGptWebPrivateVoiceRelayPoll.Failed("invalid_answer")
                } else {
                    ChatGptWebPrivateVoiceRelayPoll.Ready(
                        ChatGptWebPrivateVoiceAnswer(answer),
                    )
                }
            }
            "failed" -> ChatGptWebPrivateVoiceRelayPoll.Failed(
                value.optString("code").takeIf(SAFE_FAILURES::contains) ?: "relay_failed",
            )
            else -> ChatGptWebPrivateVoiceRelayPoll.Failed("malformed_result")
        }
    }

    private fun isAudioSdp(value: String): Boolean =
        (value.startsWith("v=0\r\n") || value.startsWith("v=0\n")) &&
            Regex("(?:\\r?\\n)m=audio\\s", RegexOption.IGNORE_CASE).containsMatchIn(value)

    private fun parseEvaluateObject(rawEvaluateValue: String?): JSONObject? {
        if (rawEvaluateValue.isNullOrBlank() || rawEvaluateValue == "null") return null
        val unwrapped = runCatching { JSONTokener(rawEvaluateValue).nextValue() }.getOrNull()
        val payload = unwrapped as? String ?: return null
        if (payload.isBlank()) return null
        return runCatching { JSONObject(payload) }.getOrNull()
    }

    private fun mediaControlScript(operation: String): String =
        """
            (function () {
              var relay = $RELAY_OBJECT;
              return relay && relay.version >= 4 && typeof relay.${operation.substringBefore('(')} === "function"
                ? relay.$operation
                : JSON.stringify({version: 4, applied: false, code: "unsupported_relay"});
            })();
        """.trimIndent()

    private fun validToken(value: String, maxLength: Int): Boolean =
        value.length in 1..maxLength && value.all { it.code in 0x20..0x7e }

    private fun validDataChannelLabel(value: String): Boolean =
        value.length <= 64 && value.all { it.code in 0x20..0x7e }

    private fun JSONObject.optNullableInt(key: String, range: IntRange): Int? {
        if (!has(key) || isNull(key)) return null
        val value = opt(key) as? Number ?: return null
        val longValue = value.toLong()
        if (value.toDouble() != longValue.toDouble() || longValue !in range.first.toLong()..range.last.toLong()) {
            return null
        }
        return longValue.toInt()
    }
}
