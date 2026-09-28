package com.elon.app.chatrecords

import org.json.JSONObject
import java.io.File
import java.security.MessageDigest

/** Private, bounded byte cache. Every reuse requires a successful server permission check. */
internal class ChatRecordCache(private val root: File, scope: String) {
    data class Download(val status: Int, val etag: String?, val bytes: ByteArray)
    private val prefix = digest(scope.toByteArray()) + "_"
    fun clear() = synchronized(lock) { root.listFiles()?.filter { it.name.startsWith(prefix) }?.forEach { it.delete() }; Unit }
    fun read(url: String, fetch: (String?) -> Download): File = synchronized(lock) {
        check(root.isDirectory || root.mkdirs()) { "无法创建附件缓存" }
        val key = prefix + digest(url.toByteArray())
        val meta = File(root, "$key.json")
        val old = runCatching { JSONObject(meta.readText()) }.getOrNull()
        val oldHash = old?.optString("hash").orEmpty().takeIf { it.matches(Regex("[a-f0-9]{64}")) }
        val cached = oldHash?.let { File(root, "${key}_$it.bin") }?.takeIf {
            it.isFile && it.length() == old?.optLong("size") && digest(it.readBytes()) == oldHash
        }
        var response = fetch(if (cached != null) old?.optString("etag")?.takeIf { it.isNotBlank() } else null)
        if (response.status == 304 && cached != null) {
            cached.setLastModified(System.currentTimeMillis()); return@synchronized cached
        }
        if (response.status == 304) response = fetch(null)
        if (response.status !in 200..299) {
            if (response.status in listOf(401, 403, 404)) clear()
            error(if (response.status in listOf(403, 404)) "记录已撤回，或你已不在此群聊中" else "聊天记录读取失败（${response.status}），请重试")
        }
        require(response.bytes.size <= 12 * 1024 * 1024) { "附件过大" }
        val hash = digest(response.bytes)
        val file = File(root, "${key}_$hash.bin")
        atomicWrite(file, response.bytes)
        atomicWrite(meta, JSONObject().put("hash", hash).put("size", file.length()).put("etag", response.etag.orEmpty()).toString().toByteArray())
        file.setLastModified(System.currentTimeMillis())
        if (cached != null && cached != file) cached.delete()
        var size = root.listFiles().orEmpty().sumOf { it.length() }
        root.listFiles().orEmpty().sortedBy { it.lastModified() }.forEach {
            if (it != file && it != meta && (size > 128L * 1024 * 1024 || it.lastModified() < System.currentTimeMillis() - 7L * 86400000)) {
                val length = it.length(); if (it.delete()) size -= length
            }
        }
        file
    }
    private fun atomicWrite(target: File, bytes: ByteArray) {
        val temporary = File(target.parentFile, target.name + ".tmp")
        temporary.writeBytes(bytes)
        try {
            java.nio.file.Files.move(temporary.toPath(), target.toPath(), java.nio.file.StandardCopyOption.ATOMIC_MOVE, java.nio.file.StandardCopyOption.REPLACE_EXISTING)
        } catch (_: java.nio.file.AtomicMoveNotSupportedException) {
            java.nio.file.Files.move(temporary.toPath(), target.toPath(), java.nio.file.StandardCopyOption.REPLACE_EXISTING)
        }
    }
    companion object {
        private val lock = Any()
        private fun digest(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
    }
}
