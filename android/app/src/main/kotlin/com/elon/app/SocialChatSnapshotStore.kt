package com.elon.app

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest

/** Read cache only. Credentials and pending sends never enter this store. */
internal class SocialChatSnapshotStore(private val directory: File, private val now: () -> Long = System::currentTimeMillis) {
    fun read(key: String): JSONArray? = synchronized(lock) {
        runCatching {
            val file = file(key)
            if (!file.isFile || file.length() > MAX_ENTRY_BYTES) return@synchronized null
            val value = JSONObject(file.readText())
            if (value.optInt("schema") != 1 || now() - value.optLong("savedAt") !in 0..MAX_AGE_MS) return@synchronized null
            value.optJSONArray("rows")
        }.getOrNull()
    }

    fun write(key: String, rows: JSONArray, allowed: () -> Boolean = { true }): Boolean = synchronized(lock) {
        if (!allowed()) return@synchronized false
        runCatching {
            directory.mkdirs()
            val value = JSONObject().put("schema", 1).put("savedAt", now()).put("rows", rows).toString()
            if (value.toByteArray().size > MAX_ENTRY_BYTES) return@synchronized false
            val file = file(key)
            val temp = File(directory, file.name + ".tmp")
            temp.writeText(value)
            if (!temp.renameTo(file)) { file.writeText(value); temp.delete() }
            val entries = directory.listFiles().orEmpty().filter { it.extension == "json" }.sortedByDescending { it.lastModified() }
            var bytes = 0L
            entries.forEachIndexed { index, entry ->
                bytes += entry.length()
                if (index >= 60 || bytes > 12 * 1024 * 1024) entry.delete()
            }
            true
        }.getOrDefault(false)
    }

    fun remove(key: String) = synchronized(lock) { file(key).delete(); Unit }
    private fun file(key: String) = File(directory, hash(key) + ".json")

    companion object {
        private val lock = Any()
        const val MAX_ENTRY_BYTES = 2 * 1024 * 1024
        const val MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000L
        private const val ROOT = "social-chat-cache-v1"
        fun forAccount(context: Context, server: String, user: String) = SocialChatSnapshotStore(File(File(context.filesDir, ROOT), hash("${server.trimEnd('/')}|$user")))
        fun clear(context: Context) = synchronized(lock) { File(context.filesDir, ROOT).deleteRecursively(); Unit }
        private fun hash(value: String) = MessageDigest.getInstance("SHA-256").digest(value.toByteArray()).joinToString("") { "%02x".format(it) }
    }
}
