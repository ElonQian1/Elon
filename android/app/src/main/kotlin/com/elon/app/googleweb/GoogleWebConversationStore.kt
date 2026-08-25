package com.elon.app.googleweb

import android.content.Context
import android.util.AtomicFile
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationCollection
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject
import java.io.File
import java.io.FileOutputStream
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

internal data class GoogleWebConversationCache(
    val records: List<GoogleWebConversationRecord>,
    val officialCachedAtMs: Long,
)

internal class GoogleWebConversationStore(
    context: Context,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val file = AtomicFile(File(context.noBackupFilesDir, FILE_NAME))
    private val projectStore = GoogleWebProjectStore(context)
    private val restored = restore()
    private var records = restored.records
    private var officialCachedAtMs = restored.officialCachedAtMs

    fun index(activePath: String?): ChatGptWebConversationIndexState {
        val organization = projectStore.snapshot()
        val projectsById = organization.projects.associateBy(GoogleWebProjectRecord::id)
        return ChatGptWebConversationIndexState(
            conversations = records.map { record ->
                val project = organization.assignments[record.path]?.let(projectsById::get)
                ChatGptWebConversation(
                    id = record.id,
                    title = record.title,
                    path = record.path,
                    active = record.path == activePath,
                    groupLabel = "Google AI 搜索",
                    projectId = project?.id,
                    projectTitle = project?.title,
                    projectPath = project?.path,
                    activityDates = record.activityDates,
                )
            },
            projects = organization.projects.map { project ->
                ChatGptWebProject(project.id, project.title, project.path)
            },
            collection = GoogleWebConversationCachePolicy.collection(
                recordCount = records.size,
                officialCachedAtMs = officialCachedAtMs,
            ),
        )
    }

    fun createProject(title: String): Boolean = projectStore.createProject(title)

    fun assignConversation(path: String, projectId: String?): Boolean {
        if (records.none { it.path == path }) return false
        return projectStore.assignConversation(path, projectId)
    }

    fun acceptOfficial(conversations: List<ChatGptWebConversation>): Boolean {
        var next = records
        conversations.take(MAX_ITEMS).asReversed().forEach { conversation ->
            val url = GoogleWebNavigationPolicy.sanitizeRestorableUrl(conversation.providerUrl)
                ?: return@forEach
            val title = conversation.title.trim().takeIf(String::isNotBlank) ?: return@forEach
            next = GoogleWebConversationIndexPolicy.upsert(
                records = next,
                restorableUrl = url,
                title = title,
                date = null,
                preferredPath = null,
            ).records
        }
        val changed = next != records
        records = next.take(MAX_ITEMS)
        officialCachedAtMs = nowMs().coerceAtLeast(0L)
        save()
        return changed
    }

    fun observe(
        url: String,
        title: String,
        date: LocalDate = LocalDate.now(),
        preferredPath: String? = null,
    ): String? {
        val safeUrl = GoogleWebNavigationPolicy.sanitizeRestorableUrl(url) ?: return null
        if (!safeUrl.contains('?')) return null
        val upsert = GoogleWebConversationIndexPolicy.upsert(
            records = records,
            restorableUrl = safeUrl,
            title = title,
            date = date,
            preferredPath = preferredPath,
        )
        if (records != upsert.records) {
            records = upsert.records
            save()
        }
        return upsert.path
    }

    fun restorableUrl(path: String): String? = records
        .firstOrNull { it.path == path }
        ?.restorableUrl

    fun currentPath(url: String?): String? {
        val safeUrl = GoogleWebNavigationPolicy.sanitizeRestorableUrl(url) ?: return null
        if (!safeUrl.contains('?')) return null
        return GoogleWebConversationIndexPolicy.currentPath(records, safeUrl)
    }

    private fun restore(): GoogleWebConversationCache {
        val raw = runCatching { file.readFully().toString(Charsets.UTF_8) }.getOrNull()
            ?: return GoogleWebConversationCache(emptyList(), 0L)
        return GoogleWebConversationCodec.decodeCache(raw)
    }

    private fun save() {
        val payload = GoogleWebConversationCodec.encode(
            records = records.take(MAX_ITEMS),
            officialCachedAtMs = officialCachedAtMs,
        ).toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_BYTES) return
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    private companion object {
        const val FILE_NAME = "google-web-conversation-index-v1.json"
        const val MAX_ITEMS = 200
        const val MAX_BYTES = 256 * 1024
    }
}

internal object GoogleWebConversationCachePolicy {
    fun collection(
        recordCount: Int,
        officialCachedAtMs: Long,
    ): ChatGptWebConversationCollection {
        val safeCount = recordCount.coerceAtLeast(0)
        val safeCachedAtMs = officialCachedAtMs.coerceAtLeast(0L)
        val neverRefreshed = safeCachedAtMs == 0L
        return ChatGptWebConversationCollection(
            observedCount = safeCount,
            source = if (safeCount == 0 && neverRefreshed) {
                ChatGptWebConversationCollection.SOURCE_NONE
            } else {
                ChatGptWebConversationCollection.SOURCE_CACHE
            },
            stale = safeCount > 0 && neverRefreshed,
            officialLoadState = ChatGptWebConversationCollection.LOAD_READY,
            cachedAtMs = safeCachedAtMs,
        )
    }
}

internal object GoogleWebConversationCodec {
    private const val SCHEMA = "elon.google_web.conversation_index.v2"
    private const val LEGACY_SCHEMA = "elon.google_web.conversation_index.v1"
    private const val MAX_ITEMS = 200
    private const val MAX_TITLE_LENGTH = 160
    private const val PATH_PREFIX = "/google-ai-mode/conversation/"
    private val ID = Regex("[a-f0-9]{64}")
    private val DATE = Regex("\\d{4}-\\d{2}-\\d{2}")

    fun encode(
        records: List<GoogleWebConversationRecord>,
        officialCachedAtMs: Long = 0L,
    ): String = JSONObject()
        .put("schema", SCHEMA)
        .put("official_cached_at_ms", officialCachedAtMs.coerceAtLeast(0L))
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

    fun decode(raw: String): List<GoogleWebConversationRecord> = decodeCache(raw).records

    fun decodeCache(raw: String): GoogleWebConversationCache {
        val root = runCatching { JSONObject(raw) }.getOrNull()
            ?: return GoogleWebConversationCache(emptyList(), 0L)
        val schema = root.optString("schema")
        if (schema != SCHEMA && schema != LEGACY_SCHEMA) {
            return GoogleWebConversationCache(emptyList(), 0L)
        }
        val values = root.optJSONArray("conversations")
            ?: return GoogleWebConversationCache(emptyList(), 0L)
        val records = buildList {
            val seen = mutableSetOf<String>()
            val seenUrls = mutableSetOf<String>()
            for (index in 0 until minOf(values.length(), MAX_ITEMS)) {
                val value = values.optJSONObject(index) ?: continue
                val id = value.optString("id").takeIf(ID::matches) ?: continue
                if (!seen.add(id)) continue
                val path = value.optString("path").takeIf { it == "$PATH_PREFIX$id" } ?: continue
                val url = GoogleWebNavigationPolicy.sanitizeRestorableUrl(value.optString("url")) ?: continue
                if (!seenUrls.add(url)) continue
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
        val cachedAtMs = if (schema == SCHEMA) {
            root.optLong("official_cached_at_ms", 0L).coerceAtLeast(0L)
        } else {
            0L
        }
        return GoogleWebConversationCache(records, cachedAtMs)
    }
}
