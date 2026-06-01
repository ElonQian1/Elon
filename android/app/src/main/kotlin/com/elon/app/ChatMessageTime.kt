package com.elon.app

import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

private const val TIMELINE_GAP_MS = 5 * 60 * 1000L

private val timeFormatter: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm", Locale.CHINA)
private val monthDayFormatter: DateTimeFormatter = DateTimeFormatter.ofPattern("M月d日 HH:mm", Locale.CHINA)
private val yearFormatter: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy年M月d日 HH:mm", Locale.CHINA)
private val exactFormatter: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy年M月d日 EEEE HH:mm:ss", Locale.CHINA)
private val weekdayFormatter: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE HH:mm", Locale.CHINA)

internal fun parseChatMessageCreatedAt(value: String): Long? {
    val trimmed = value.trim()
    if (trimmed.isBlank()) return null
    return runCatching { Instant.parse(trimmed).toEpochMilli() }.getOrNull()
}

internal fun shouldShowChatTimelineLabel(messages: List<ChatMessage>, position: Int): Boolean {
    val current = messages.getOrNull(position)?.createdAtMs?.takeIf { it > 0L } ?: return false
    if (position <= 0) return true

    val previous = messages.getOrNull(position - 1)?.createdAtMs?.takeIf { it > 0L } ?: return true
    if (isDifferentLocalDate(previous, current)) return true
    return current - previous >= TIMELINE_GAP_MS
}

internal fun formatChatTimelineLabel(timestampMs: Long, nowMs: Long = System.currentTimeMillis()): String {
    if (timestampMs <= 0L) return ""
    val zone = ZoneId.systemDefault()
    val dateTime = Instant.ofEpochMilli(timestampMs).atZone(zone)
    val today = Instant.ofEpochMilli(nowMs).atZone(zone).toLocalDate()
    val date = dateTime.toLocalDate()
    return when {
        date == today -> dateTime.format(timeFormatter)
        date == today.minusDays(1) -> "昨天 ${dateTime.format(timeFormatter)}"
        date.isAfter(today.minusDays(7)) -> dateTime.format(weekdayFormatter)
        date.year == today.year -> dateTime.format(monthDayFormatter)
        else -> dateTime.format(yearFormatter)
    }
}

internal fun formatChatMessageExactTime(timestampMs: Long): String {
    if (timestampMs <= 0L) return "旧消息未记录发送时间"
    return Instant.ofEpochMilli(timestampMs)
        .atZone(ZoneId.systemDefault())
        .format(exactFormatter)
}

private fun isDifferentLocalDate(leftMs: Long, rightMs: Long): Boolean {
    val zone = ZoneId.systemDefault()
    val leftDate: LocalDate = Instant.ofEpochMilli(leftMs).atZone(zone).toLocalDate()
    val rightDate: LocalDate = Instant.ofEpochMilli(rightMs).atZone(zone).toLocalDate()
    return leftDate != rightDate
}
