package com.elon.app

import android.content.Context
import android.util.AtomicFile
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebProject
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream

internal enum class WebChatConversationProjectMoveStage(val wireValue: String) {
    PREPARED("prepared"),
    WRITE_ARMED("write_armed"),
}

internal data class WebChatConversationProjectMoveRecoveryRecord(
    val conversationPath: String,
    val sourceProjectId: String?,
    val destinationProjectId: String,
    val stage: WebChatConversationProjectMoveStage,
    val createdAtMs: Long,
    val updatedAtMs: Long,
)

internal class WebChatConversationProjectMoveRecoveryStore(
    context: Context,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val file = AtomicFile(File(context.noBackupFilesDir, FILE_NAME))

    @Synchronized
    fun prepare(
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
    ): WebChatConversationProjectMoveRecoveryRecord? {
        val path = ChatGptWebConversationPath.normalize(conversation.path) ?: return null
        val destinationProjectId = ChatGptWebConversationPath.canonicalProjectId(destination.id)
            ?: return null
        val sourceProjectId = ChatGptWebConversationPath.canonicalProjectId(conversation.projectId)
            ?: ChatGptWebConversationPath.projectId(path)
        if (sourceProjectId == destinationProjectId) return null
        val timestamp = nowMs()
        return WebChatConversationProjectMoveRecoveryRecord(
            conversationPath = path,
            sourceProjectId = sourceProjectId,
            destinationProjectId = destinationProjectId,
            stage = WebChatConversationProjectMoveStage.PREPARED,
            createdAtMs = timestamp,
            updatedAtMs = timestamp,
        ).takeIf(::save)
    }

    @Synchronized
    fun armWrite(): WebChatConversationProjectMoveRecoveryRecord? {
        val current = restoreInternal() ?: return null
        if (current.stage != WebChatConversationProjectMoveStage.PREPARED) return current
        return current.copy(
            stage = WebChatConversationProjectMoveStage.WRITE_ARMED,
            updatedAtMs = nowMs(),
        ).takeIf(::save)
    }

    @Synchronized
    fun restore(): WebChatConversationProjectMoveRecoveryRecord? {
        val record = restoreInternal() ?: return null
        if (nowMs() - record.updatedAtMs <= MAX_AGE_MS) return record
        clear()
        return null
    }

    @Synchronized
    fun clear() {
        file.delete()
    }

    private fun restoreInternal(): WebChatConversationProjectMoveRecoveryRecord? {
        val bytes = runCatching { file.readFully() }.getOrNull() ?: return null
        if (bytes.size > MAX_FILE_BYTES) return null
        return WebChatConversationProjectMoveRecoveryCodec.decode(bytes.toString(Charsets.UTF_8))
    }

    private fun save(record: WebChatConversationProjectMoveRecoveryRecord): Boolean {
        val payload = WebChatConversationProjectMoveRecoveryCodec.encode(record)
            .toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_FILE_BYTES) return false
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull()
            ?: return false
        return try {
            output.write(payload)
            file.finishWrite(output)
            true
        } catch (_: Exception) {
            file.failWrite(output)
            false
        }
    }

    private companion object {
        const val FILE_NAME = "chatgpt-project-move-recovery-v1.json"
        const val MAX_FILE_BYTES = 8 * 1024
        const val MAX_AGE_MS = 24L * 60L * 60L * 1_000L
    }
}

internal object WebChatConversationProjectMoveRecoveryCodec {
    private const val SCHEMA = "elon.chatgpt_web.project_move_recovery.v1"

    fun encode(record: WebChatConversationProjectMoveRecoveryRecord): String = JSONObject()
        .put("schema", SCHEMA)
        .put("conversation_path", record.conversationPath)
        .put("source_project_id", record.sourceProjectId ?: JSONObject.NULL)
        .put("destination_project_id", record.destinationProjectId)
        .put("stage", record.stage.wireValue)
        .put("created_at_ms", record.createdAtMs)
        .put("updated_at_ms", record.updatedAtMs)
        .toString()

    fun decode(raw: String): WebChatConversationProjectMoveRecoveryRecord? {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return null
        if (root.optString("schema") != SCHEMA) return null
        val path = ChatGptWebConversationPath.normalize(root.optString("conversation_path"))
            ?: return null
        val sourceProjectId = root.opt("source_project_id")
            ?.takeUnless { it == JSONObject.NULL }
            ?.toString()
            ?.let(ChatGptWebConversationPath::canonicalProjectId)
        val destinationProjectId = ChatGptWebConversationPath.canonicalProjectId(
            root.optString("destination_project_id"),
        ) ?: return null
        if (sourceProjectId == destinationProjectId) return null
        val stage = WebChatConversationProjectMoveStage.entries.singleOrNull {
            it.wireValue == root.optString("stage")
        } ?: return null
        val createdAtMs = root.optLong("created_at_ms", -1L)
        val updatedAtMs = root.optLong("updated_at_ms", -1L)
        if (createdAtMs < 0L || updatedAtMs < createdAtMs) return null
        return WebChatConversationProjectMoveRecoveryRecord(
            conversationPath = path,
            sourceProjectId = sourceProjectId,
            destinationProjectId = destinationProjectId,
            stage = stage,
            createdAtMs = createdAtMs,
            updatedAtMs = updatedAtMs,
        )
    }
}
