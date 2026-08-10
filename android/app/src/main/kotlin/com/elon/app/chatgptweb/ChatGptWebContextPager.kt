package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.security.MessageDigest

internal object ChatGptWebContextPager {
    const val SCHEMA = "elon.chatgpt_web.context.v2"

    fun page(
        snapshot: ChatGptWebSnapshot,
        cursor: String,
        requestedOffset: Int,
        requestedLimit: Int,
        maxLimit: Int,
    ): Result {
        val revision = revision(snapshot)
        val decoded = if (cursor.isBlank()) null else decode(cursor)
            ?: return failure(INVALID_CURSOR, revision, snapshot)
        if (decoded != null && decoded.revision != revision) {
            return failure(STALE_CURSOR, revision, snapshot)
        }

        val windowStart = snapshot.messageWindowStart
        val windowEnd = windowStart + snapshot.messages.size
        val requestedGlobalOffset = decoded?.offset ?: requestedOffset
        if (requestedGlobalOffset < windowStart) {
            return failure(HISTORY_UNAVAILABLE, revision, snapshot)
        }
        val offset = requestedGlobalOffset.coerceIn(windowStart, windowEnd)
        val localOffset = offset - windowStart
        val limit = requestedLimit.coerceIn(1, maxLimit)
        val messages = snapshot.messages.drop(localOffset).take(limit)
        val nextOffset = offset + messages.size
        return Result.Success(
            Page(
                revision = revision,
                offset = offset,
                limit = limit,
                nextOffset = nextOffset,
                hasMore = nextOffset < windowEnd,
                hasMoreBefore = windowStart > 0,
                cursor = encode(revision, offset),
                nextCursor = if (nextOffset < windowEnd) {
                    encode(revision, nextOffset)
                } else {
                    null
                },
                messages = messages,
            ),
        )
    }

    internal fun revision(snapshot: ChatGptWebSnapshot): String {
        val digest = MessageDigest.getInstance("SHA-256")
        digest.addField(snapshot.url)
        digest.addField(snapshot.messageWindowStart.toString())
        digest.addField(snapshot.observedMessageCount.toString())
        snapshot.messages.forEach { message ->
            digest.addField(message.id)
            digest.addField(message.role)
            digest.addField(message.state)
            digest.addField(message.content)
            message.parts.forEach { part ->
                digest.addField(part.type)
                digest.addField(part.label)
            }
        }
        return digest.digest().take(REVISION_BYTES).joinToString("") { byte ->
            "%02x".format(byte.toInt() and 0xff)
        }
    }

    private fun encode(revision: String, offset: Int): String =
        "$CURSOR_PREFIX.$revision.${offset.toString(RADIX)}"

    private fun failure(
        code: String,
        revision: String,
        snapshot: ChatGptWebSnapshot,
    ): Result.Failure = Result.Failure(
        code = code,
        currentRevision = revision,
        observedMessageCount = snapshot.observedMessageCount,
        messageWindowStart = snapshot.messageWindowStart,
        messageWindowEnd = snapshot.messageWindowStart + snapshot.messages.size,
    )

    private fun decode(value: String): Cursor? {
        if (!CURSOR.matches(value)) return null
        val parts = value.split('.')
        val offset = parts[2].toIntOrNull(RADIX) ?: return null
        return Cursor(parts[1], offset)
    }

    private fun MessageDigest.addField(value: String) {
        val bytes = value.toByteArray(StandardCharsets.UTF_8)
        update(byteArrayOf(
            (bytes.size ushr 24).toByte(),
            (bytes.size ushr 16).toByte(),
            (bytes.size ushr 8).toByte(),
            bytes.size.toByte(),
        ))
        update(bytes)
    }

    internal sealed interface Result {
        data class Success(val page: Page) : Result

        data class Failure(
            val code: String,
            val currentRevision: String,
            val observedMessageCount: Int,
            val messageWindowStart: Int,
            val messageWindowEnd: Int,
        ) : Result
    }

    internal data class Page(
        val revision: String,
        val offset: Int,
        val limit: Int,
        val nextOffset: Int,
        val hasMore: Boolean,
        val hasMoreBefore: Boolean,
        val cursor: String,
        val nextCursor: String?,
        val messages: List<ChatGptWebMessage>,
    )

    private data class Cursor(
        val revision: String,
        val offset: Int,
    )

    private const val CURSOR_PREFIX = "ctx1"
    private const val REVISION_BYTES = 12
    private const val RADIX = 36
    private const val INVALID_CURSOR = "invalid_context_cursor"
    private const val STALE_CURSOR = "stale_context_cursor"
    private const val HISTORY_UNAVAILABLE = "context_history_unavailable"
    private val CURSOR = Regex("$CURSOR_PREFIX\\.[a-f0-9]{24}\\.[a-z0-9]{1,7}")
}
