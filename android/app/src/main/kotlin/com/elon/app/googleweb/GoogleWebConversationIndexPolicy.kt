package com.elon.app.googleweb

import java.security.MessageDigest
import java.time.LocalDate

internal data class GoogleWebConversationUpsert(
    val records: List<GoogleWebConversationRecord>,
    val path: String,
)

internal object GoogleWebConversationIndexPolicy {
    fun upsert(
        records: List<GoogleWebConversationRecord>,
        restorableUrl: String,
        title: String,
        date: LocalDate,
        preferredPath: String?,
    ): GoogleWebConversationUpsert {
        val previous = records.firstOrNull { it.path == preferredPath }
            ?: records.firstOrNull { it.restorableUrl == restorableUrl }
            ?: records.firstOrNull { it.id == sha256(restorableUrl) }
        val id = previous?.id ?: sha256(restorableUrl)
        val path = "$PATH_PREFIX$id"
        val cleanTitle = title.trim().ifBlank { "Google AI 搜索" }.take(MAX_TITLE_LENGTH)
        val next = GoogleWebConversationRecord(
            id = id,
            title = previous?.title?.takeUnless { it == "Google AI 搜索" } ?: cleanTitle,
            path = path,
            restorableUrl = restorableUrl,
            activityDates = previous?.activityDates.orEmpty() + date.toString(),
        )
        return GoogleWebConversationUpsert(
            records = listOf(next) + records.filterNot { it.id == id },
            path = path,
        )
    }

    fun currentPath(records: List<GoogleWebConversationRecord>, restorableUrl: String): String? =
        records.firstOrNull { it.restorableUrl == restorableUrl }?.path

    private fun sha256(value: String): String = MessageDigest.getInstance("SHA-256")
        .digest(value.toByteArray(Charsets.UTF_8))
        .joinToString("") { "%02x".format(it) }

    private const val PATH_PREFIX = "/google-ai-mode/conversation/"
    private const val MAX_TITLE_LENGTH = 160
}
