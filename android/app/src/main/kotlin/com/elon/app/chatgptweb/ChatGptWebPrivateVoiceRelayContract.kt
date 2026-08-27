package com.elon.app.chatgptweb

import org.json.JSONObject
import org.json.JSONTokener

internal class ChatGptWebPrivateVoiceAnswer internal constructor(
    private val value: String,
) {
    internal fun value(): String = value

    override fun toString(): String = "ChatGptWebPrivateVoiceAnswer(<redacted>)"
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

internal object ChatGptWebPrivateVoiceRelayContract {
    private const val RELAY_OBJECT = "window.__elonChatGptPrivateVoiceRelay"
    private const val MAX_OFFER_LENGTH = 240_000
    private const val MAX_ANSWER_LENGTH = 320_000
    private val REQUEST_ID = Regex("^relay_[a-z0-9]{8,32}$")
    private val SAFE_FAILURES = setOf(
        "busy",
        "invalid_answer",
        "invalid_offer",
        "network_error",
        "template_consumed",
        "template_expired",
        "template_unavailable",
        "timeout",
        "upstream_rejected",
    )

    fun validOffer(value: String): Boolean =
        value.length in 16..MAX_OFFER_LENGTH && isAudioSdp(value)

    fun startScript(requestId: String, offer: String): String? {
        if (!REQUEST_ID.matches(requestId) || !validOffer(offer)) return null
        return """
            (function () {
              var relay = $RELAY_OBJECT;
              if (!relay || relay.version !== 1) return false;
              return relay.startExchange(${JSONObject.quote(requestId)}, ${JSONObject.quote(offer)});
            })();
        """.trimIndent()
    }

    fun pollScript(requestId: String): String? {
        if (!REQUEST_ID.matches(requestId)) return null
        return """
            (function () {
              var relay = $RELAY_OBJECT;
              return relay && relay.version === 1
                ? relay.takeResult(${JSONObject.quote(requestId)})
                : JSON.stringify({status: "failed", code: "template_unavailable"});
            })();
        """.trimIndent()
    }

    fun parsePoll(rawEvaluateValue: String?): ChatGptWebPrivateVoiceRelayPoll {
        if (rawEvaluateValue.isNullOrBlank() || rawEvaluateValue == "null") {
            return ChatGptWebPrivateVoiceRelayPoll.Pending
        }
        val unwrapped = runCatching { JSONTokener(rawEvaluateValue).nextValue() }.getOrNull()
        val payload = when (unwrapped) {
            is String -> unwrapped
            else -> return ChatGptWebPrivateVoiceRelayPoll.Failed("malformed_result")
        }
        if (payload.isBlank()) return ChatGptWebPrivateVoiceRelayPoll.Pending
        val value = runCatching { JSONObject(payload) }.getOrNull()
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
}
