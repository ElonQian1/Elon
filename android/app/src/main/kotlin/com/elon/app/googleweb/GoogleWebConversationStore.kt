package com.elon.app.googleweb

import android.content.Context
import android.util.AtomicFile
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationCollection
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest
import java.time.LocalDate
import org.json.JSONArray
import org.json.JSONObject

internal data class GoogleWebConversationRecord(
    val id: String,
    val title: String,
    val path: String,
    val restorableUrl: String,
    val activityDates: Set<String>,
)

internal class GoogleWebConversationStore(context: Context) {
    private val file = AtomicFile(File(context.noBackupFilesDir, FILE_NAME))
    private var records = restore()

    fun index(activePath: String?): ChatGptWebConversationIndexState = ChatGptWebConversationIndexState(
        conversations = records.map { record ->
            ChatGptWebConversation(
                id = record.id,
                title = record.title,
                path = record.path,
                active = record.path == activePath,
                groupLabel = "Google AI 搜索",
                activityDates = record.activityDates,
            )
        },
        projects = emptyList(),
        collection = ChatGptWebConversationCollection(
            observedCount = records.size,
            source = ChatGptWebConversationCollection.SOURCE_CACHE,
            stale = false,
            officialLoadState = ChatGptWebConversationCollection.LOAD_READY,
            cachedAtMs = System.currentTimeMillis(),
        ),
    )

    fun observe(url: String, title: String, date: LocalDate = LocalDate.now()): String? {
        val safeUrl = GoogleWebNavigationPolicy.sanitizeRestorableUrl(url) ?: return null
        if (!safeUrl.contains('?')) return null
        val id = sha256(safeUrl)
        val path = "$PATH_PREFIX$id"
        val cleanTitle = title.trim().ifBlank { "Google AI 搜索" }.take(MAX_TITLE_LENGTH)
        val previous = records.firstOrNull { it.id == id }
        val next = GoogleWebConversationRecord(
            id = id,
            title = cleanTitle,
            path = path,
            restorableUrl = safeUrl,
            activityDates = previous?.activityDates.orEmpty() + date.toString(),
        )
        records = listOf(next) + records.filterNot { it.id == id }
        save()
        return path
    }

    fun restorableUrl(path: String): String? = records
        .firstOrNull { it.path == path }
        ?.restorableUrl

    fun currentPath(url: String?): String? {
        val safeUrl = GoogleWebNavigationPolicy.sanitizeRestorableUrl(url) ?: return null
        if (!safeUrl.contains('?')) return null
        return "$PATH_PREFIX${sha256(safeUrl)}"
    }

    private fun restore(): List<GoogleWebConversationRecord> {
        val raw = runCatching { file.readFully().toString(Charsets.UTF_8) }.getOrNull() ?: return emptyList()
        return GoogleWebConversationCodec.decode(raw)
    }

    private fun save() {
        val payload = GoogleWebConversationCodec.encode(records.take(MAX_ITEMS)).toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_BYTES) return
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    private fun sha256(value: String): String = MessageDigest.getInstance("SHA-256")
        .digest(value.toByteArray(Charsets.UTF_8))
        .joinToString("") { "%02x".format(it) }

    private companion object {
        const val FILE_NAME = "google-web-conversation-index-v1.json"
        const val PATH_PREFIX = "/google-ai-mode/conversation/"
        const val MAX_ITEMS = 200
        const val MAX_BYTES = 256 * 1024
        const val MAX_TITLE_LENGTH = 160
    }
}

internal object GoogleWebConversationCodec {
    private const val SCHEMA = "elon.google_web.conversation_index.v1"
    private const val MAX_ITEMS = 200
    private const val MAX_TITLE_LENGTH = 160
    private const val PATH_PREFIX = "/google-ai-mode/conversation/"
    private val ID = Regex("[a-f0-9]{64}")
    private val DATE = Regex("\\d{4}-\\d{2}-\\d{2}")

    fun encode(records: List<GoogleWebConversationRecord>): String = JSONObject()
        .put("schema", SCHEMA)
        .put("conversations", JSONArray().apply {
            records.take(MAX_ITEMS).forEach { record ->
                put(JSONObject()
                    .put("id", record.id)
                    .put("title", record.title.take(MAX_TITLE_LENGTH))
                    .put("path", record.path)
                    .put("url", record.restorableUrl)
                    .put("activity_dates", JSONArray(record.activityDates.sorted())))
            }
        })
        .toString()

    fun decode(raw: String): List<GoogleWebConversationRecord> {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return emptyList()
        if (root.optString("schema") != SCHEMA) return emptyList()
        val values = root.optJSONArray("conversations") ?: return emptyList()
        return buildList {
            val seen = mutableSetOf<String>()
            for (index in 0 until minOf(values.length(), MAX_ITEMS)) {
                val value = values.optJSONObject(index) ?: continue
                val id = value.optString("id").takeIf(ID::matches) ?: continue
                if (!seen.add(id)) continue
                val path = value.optString("path").takeIf { it == "$PATH_PREFIX$id" } ?: continue
                val url = GoogleWebNavigationPolicy.sanitizeRestorableUrl(value.optString("url")) ?: continue
                val title = value.optString("title").trim().take(MAX_TITLE_LENGTH)
                if (title.isBlank()) continue
                val dates = buildSet {
                    val rawDates = value.optJSONArray("activity_dates") ?: return@buildSet
                    for (dateIndex in 0 until minOf(rawDates.length(), 64)) {
                        rawDates.optString(dateIndex).takeIf(DATE::matches)?.let(::add)
                    }
                }
                add(GoogleWebConversationRecord(id, title, path, url, dates))
            }
        }
    }
}
