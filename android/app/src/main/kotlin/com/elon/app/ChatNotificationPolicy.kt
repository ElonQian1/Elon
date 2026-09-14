package com.elon.app

import java.security.MessageDigest
import java.time.Instant

/** Shared by realtime messages and summary polling; contains no Android audio calls. */
internal class ChatNotificationPolicy {
    data class Decision(val show: Boolean, val alert: Boolean)

    private val messageIds = LinkedHashSet<String>()
    // true means a summary arrived before its realtime message ID.
    private val observations = LinkedHashMap<String, Boolean>()
    private val lastAlerts = LinkedHashMap<String, Long>()
    private var lastGlobalAlert: Long? = null

    @Synchronized
    fun receive(
        conversation: String,
        messageId: String,
        createdAt: String?,
        content: String,
        visible: Boolean,
        group: Boolean,
        nowMs: Long,
    ): Decision {
        val id = messageId.takeIf { it.isNotBlank() }?.let { "$conversation:id:$it" }
        val timestamp = createdAt?.trim()?.takeIf { it.isNotEmpty() }?.let {
            runCatching { Instant.parse(it).toString() }.getOrDefault(it)
        }
        // Summary APIs may abbreviate text/attachments and omit the message ID.
        // Their timestamp is the same server timestamp as the realtime event.
        val observation = "$conversation:at:${timestamp ?: digest(content.trim())}"
        val duplicate = if (id != null) {
            !messageIds.add(id) || observations[observation] == true
        } else {
            observations.containsKey(observation)
        }
        if (id != null || !observations.containsKey(observation)) {
            observations[observation] = id == null
        }
        while (messageIds.size > MAX_RECENT) messageIds.remove(messageIds.first())
        trim(observations)
        if (duplicate || visible) return Decision(show = false, alert = false)

        val interval = if (group) 10_000L else 3_000L
        val alert = elapsed(lastAlerts[conversation], nowMs) >= interval &&
            elapsed(lastGlobalAlert, nowMs) >= 1_500L
        if (alert) {
            lastAlerts[conversation] = nowMs
            lastGlobalAlert = nowMs
            trim(lastAlerts)
        }
        return Decision(show = true, alert = alert)
    }

    private fun elapsed(previous: Long?, now: Long): Long =
        if (previous == null || now < previous) Long.MAX_VALUE else now - previous

    private fun <T> trim(values: LinkedHashMap<String, T>) {
        while (values.size > MAX_RECENT) values.remove(values.keys.first())
    }

    private fun digest(value: String): String = MessageDigest.getInstance("SHA-256")
        .digest(value.toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it) }

    private companion object {
        const val MAX_RECENT = 512
    }
}
