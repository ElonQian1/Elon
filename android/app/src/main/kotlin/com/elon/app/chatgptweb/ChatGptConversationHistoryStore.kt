package com.elon.app.chatgptweb

import android.content.Context
import android.util.AtomicFile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream

internal data class ChatGptConversationHistoryCache(
    val conversations: List<ChatGptWebConversation>,
    val savedAtMs: Long,
    val projects: List<ChatGptWebProject> = emptyList(),
)

internal class ChatGptConversationHistoryStore(
    context: Context,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val file = AtomicFile(File(context.noBackupFilesDir, FILE_NAME))

    fun restore(): ChatGptConversationHistoryCache? {
        val bytes = runCatching { file.readFully() }.getOrNull() ?: return null
        if (bytes.size > MAX_BYTES) return null
        val cache = ChatGptConversationHistoryCodec.decode(bytes.toString(Charsets.UTF_8))
            ?: return null
        if (nowMs() - cache.savedAtMs !in 0..MAX_AGE_MS) return null
        return cache
    }

    fun save(
        conversations: List<ChatGptWebConversation>,
        projects: List<ChatGptWebProject> = emptyList(),
    ) {
        if (conversations.isEmpty() && projects.isEmpty()) {
            clear()
            return
        }
        val payload = ChatGptConversationHistoryCodec.encode(
            ChatGptConversationHistoryCache(
                conversations.take(MAX_ITEMS),
                nowMs(),
                projects.take(MAX_PROJECTS),
            ),
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

    fun clear() {
        file.delete()
    }

    private companion object {
        const val FILE_NAME = "chatgpt-conversation-index-v1.json"
        const val MAX_ITEMS = 100
        const val MAX_PROJECTS = 40
        const val MAX_BYTES = 64 * 1024
        const val MAX_AGE_MS = 7L * 24L * 60L * 60L * 1_000L
    }
}

internal object ChatGptConversationHistoryCodec {
    private const val SCHEMA = "elon.chatgpt_web.conversation_index.v2"
    private const val LEGACY_SCHEMA = "elon.chatgpt_web.conversation_index.v1"
    private const val MAX_ITEMS = 100
    private const val MAX_PROJECTS = 40
    private const val MAX_ID_LENGTH = 160
    private const val MAX_TITLE_LENGTH = 160

    fun encode(cache: ChatGptConversationHistoryCache): String = JSONObject()
        .put("schema", SCHEMA)
        .put("saved_at_ms", cache.savedAtMs)
        .put("conversations", JSONArray().apply {
            cache.conversations.take(MAX_ITEMS).forEach { conversation ->
                put(JSONObject()
                    .put("id", conversation.id.take(MAX_ID_LENGTH))
                    .put("title", conversation.title.take(MAX_TITLE_LENGTH))
                    .put("path", conversation.path)
                    .put("group_label", conversation.groupLabel.take(MAX_GROUP_LABEL_LENGTH))
                    .put("project_id", conversation.projectId ?: JSONObject.NULL)
                    .put("project_title", conversation.projectTitle ?: JSONObject.NULL)
                    .put("project_path", conversation.projectPath ?: JSONObject.NULL)
                    .put("activity_dates", JSONArray(conversation.activityDates.sorted()))
                )
            }
        })
        .put("projects", JSONArray().apply {
            cache.projects.take(MAX_PROJECTS).forEach { project ->
                put(JSONObject()
                    .put("id", project.id)
                    .put("title", project.title.take(MAX_TITLE_LENGTH))
                    .put("path", project.path)
                )
            }
        })
        .toString()

    fun decode(raw: String): ChatGptConversationHistoryCache? {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return null
        if (root.optString("schema") !in setOf(SCHEMA, LEGACY_SCHEMA)) return null
        val savedAtMs = root.optLong("saved_at_ms", -1L)
        if (savedAtMs < 0L) return null
        val values = root.optJSONArray("conversations") ?: return null
        val conversations = buildList {
            val seen = mutableSetOf<String>()
            for (index in 0 until minOf(values.length(), MAX_ITEMS)) {
                val value = values.optJSONObject(index) ?: continue
                val path = value.optString("path")
                val title = value.optString("title").trim().take(MAX_TITLE_LENGTH)
                val normalizedPath = ChatGptWebConversationPath.normalize(path) ?: continue
                if (title.isBlank() || !seen.add(normalizedPath)) continue
                add(ChatGptWebConversation(
                    id = value.optString("id").ifBlank { normalizedPath.substringAfterLast('/') }
                        .take(MAX_ID_LENGTH),
                    title = title,
                    path = normalizedPath,
                    active = false,
                    groupLabel = value.optString("group_label").trim().take(MAX_GROUP_LABEL_LENGTH),
                    projectId = value.optString("project_id").takeIf(PROJECT_ID::matches)
                        ?: ChatGptWebConversationPath.projectId(normalizedPath),
                    projectTitle = value.optString("project_title").trim()
                        .take(MAX_TITLE_LENGTH)
                        .takeIf(String::isNotBlank),
                    projectPath = ChatGptWebConversationPath.normalizeProject(
                        value.optString("project_path"),
                    ),
                    activityDates = buildSet {
                        value.optString("activity_date").takeIf(ACTIVITY_DATE::matches)?.let(::add)
                        val dates = value.optJSONArray("activity_dates") ?: return@buildSet
                        for (dateIndex in 0 until minOf(dates.length(), MAX_ACTIVITY_DATES)) {
                            dates.optString(dateIndex).takeIf(ACTIVITY_DATE::matches)?.let(::add)
                        }
                    },
                ))
            }
        }
        val projects = buildList {
            val projectValues = root.optJSONArray("projects") ?: return@buildList
            val seen = mutableSetOf<String>()
            for (index in 0 until minOf(projectValues.length(), MAX_PROJECTS)) {
                val value = projectValues.optJSONObject(index) ?: continue
                val path = ChatGptWebConversationPath.normalizeProject(value.optString("path")) ?: continue
                val id = value.optString("id").takeIf(PROJECT_ID::matches)
                    ?: ChatGptWebConversationPath.projectId(path)
                    ?: continue
                val title = value.optString("title").trim().take(MAX_TITLE_LENGTH)
                if (title.isBlank() || !seen.add(path)) continue
                add(ChatGptWebProject(id, title, path))
            }
        }
        if (conversations.isEmpty() && projects.isEmpty()) return null
        return ChatGptConversationHistoryCache(conversations, savedAtMs, projects)
    }

    private const val MAX_GROUP_LABEL_LENGTH = 80
    private val PROJECT_ID = Regex("g-p-[A-Za-z0-9_-]{1,160}")
    private val ACTIVITY_DATE = Regex("\\d{4}-\\d{2}-\\d{2}")
    private const val MAX_ACTIVITY_DATES = 32
}
