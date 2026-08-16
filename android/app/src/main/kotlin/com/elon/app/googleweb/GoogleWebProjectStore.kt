package com.elon.app.googleweb

import android.content.Context
import android.util.AtomicFile
import java.io.File
import java.io.FileOutputStream
import java.util.UUID
import org.json.JSONArray
import org.json.JSONObject

internal data class GoogleWebProjectRecord(
    val id: String,
    val title: String,
    val path: String,
)

internal data class GoogleWebProjectSnapshot(
    val projects: List<GoogleWebProjectRecord> = emptyList(),
    val assignments: Map<String, String> = emptyMap(),
)

internal object GoogleWebProjectPolicy {
    fun create(
        snapshot: GoogleWebProjectSnapshot,
        title: String,
        id: String,
    ): GoogleWebProjectSnapshot? {
        val cleanTitle = title.trim().take(MAX_TITLE_LENGTH)
        if (cleanTitle.isBlank() || !PROJECT_ID.matches(id)) return null
        if (snapshot.projects.any { it.title.equals(cleanTitle, ignoreCase = true) }) return null
        return snapshot.copy(
            projects = listOf(GoogleWebProjectRecord(
                id = id,
                title = cleanTitle,
                path = "$PROJECT_PATH_PREFIX$id",
            )) + snapshot.projects,
        )
    }

    fun assign(
        snapshot: GoogleWebProjectSnapshot,
        conversationPath: String,
        projectId: String?,
    ): GoogleWebProjectSnapshot? {
        if (!CONVERSATION_PATH.matches(conversationPath)) return null
        if (projectId != null && snapshot.projects.none { it.id == projectId }) return null
        val assignments = snapshot.assignments.toMutableMap().apply {
            if (projectId == null) remove(conversationPath) else put(conversationPath, projectId)
        }
        return snapshot.copy(assignments = assignments)
    }

    internal const val MAX_TITLE_LENGTH = 80
    internal const val PROJECT_PATH_PREFIX = "/google-ai-mode/project/"
    internal val PROJECT_ID = Regex("[a-f0-9-]{36}")
    internal val CONVERSATION_PATH = Regex("/google-ai-mode/conversation/[a-f0-9]{64}")
}

internal class GoogleWebProjectStore(context: Context) {
    private val file = AtomicFile(File(context.noBackupFilesDir, FILE_NAME))
    private var snapshot = restore()

    fun snapshot(): GoogleWebProjectSnapshot = snapshot

    fun createProject(title: String): Boolean {
        val next = GoogleWebProjectPolicy.create(snapshot, title, UUID.randomUUID().toString())
            ?: return false
        snapshot = next
        save()
        return true
    }

    fun assignConversation(conversationPath: String, projectId: String?): Boolean {
        val next = GoogleWebProjectPolicy.assign(snapshot, conversationPath, projectId)
            ?: return false
        if (next == snapshot) return true
        snapshot = next
        save()
        return true
    }

    private fun restore(): GoogleWebProjectSnapshot {
        val bytes = runCatching { file.readFully() }.getOrNull() ?: return GoogleWebProjectSnapshot()
        if (bytes.size > MAX_BYTES) return GoogleWebProjectSnapshot()
        return GoogleWebProjectCodec.decode(bytes.toString(Charsets.UTF_8))
    }

    private fun save() {
        val payload = GoogleWebProjectCodec.encode(snapshot).toByteArray(Charsets.UTF_8)
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
        const val FILE_NAME = "google-web-projects-v1.json"
        const val MAX_BYTES = 128 * 1024
    }
}

internal object GoogleWebProjectCodec {
    private const val SCHEMA = "elon.google_web.projects.v1"
    private const val MAX_PROJECTS = 80
    private const val MAX_ASSIGNMENTS = 200

    fun encode(snapshot: GoogleWebProjectSnapshot): String = JSONObject()
        .put("schema", SCHEMA)
        .put("projects", JSONArray().apply {
            snapshot.projects.take(MAX_PROJECTS).forEach { project ->
                put(JSONObject()
                    .put("id", project.id)
                    .put("title", project.title.take(GoogleWebProjectPolicy.MAX_TITLE_LENGTH))
                    .put("path", project.path))
            }
        })
        .put("assignments", JSONArray().apply {
            snapshot.assignments.entries.take(MAX_ASSIGNMENTS).forEach { (conversationPath, projectId) ->
                put(JSONObject()
                    .put("conversation_path", conversationPath)
                    .put("project_id", projectId))
            }
        })
        .toString()

    fun decode(raw: String): GoogleWebProjectSnapshot {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return GoogleWebProjectSnapshot()
        if (root.optString("schema") != SCHEMA) return GoogleWebProjectSnapshot()
        val projects = buildList {
            val values = root.optJSONArray("projects") ?: return@buildList
            val seen = mutableSetOf<String>()
            for (index in 0 until minOf(values.length(), MAX_PROJECTS)) {
                val value = values.optJSONObject(index) ?: continue
                val id = value.optString("id").takeIf(GoogleWebProjectPolicy.PROJECT_ID::matches) ?: continue
                if (!seen.add(id)) continue
                val path = value.optString("path")
                if (path != "${GoogleWebProjectPolicy.PROJECT_PATH_PREFIX}$id") continue
                val title = value.optString("title").trim().take(GoogleWebProjectPolicy.MAX_TITLE_LENGTH)
                if (title.isBlank()) continue
                add(GoogleWebProjectRecord(id, title, path))
            }
        }
        val projectIds = projects.mapTo(mutableSetOf(), GoogleWebProjectRecord::id)
        val assignments = linkedMapOf<String, String>()
        val values = root.optJSONArray("assignments")
        if (values != null) for (index in 0 until minOf(values.length(), MAX_ASSIGNMENTS)) {
            val value = values.optJSONObject(index) ?: continue
            val conversationPath = value.optString("conversation_path")
                .takeIf(GoogleWebProjectPolicy.CONVERSATION_PATH::matches)
                ?: continue
            val projectId = value.optString("project_id").takeIf(projectIds::contains) ?: continue
            assignments.putIfAbsent(conversationPath, projectId)
        }
        return GoogleWebProjectSnapshot(projects, assignments)
    }
}
