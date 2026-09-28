package com.elon.app.chatrecords

import org.json.JSONObject
import java.io.File
import java.security.MessageDigest

/** Network operations never hold the disk lock. Freshness is bounded by server validation time. */
internal class ChatRecordCache(private val root: File, scope: String, private val now: () -> Long = System::currentTimeMillis) {
    data class Download(val status: Int, val etag: String?, val bytes: ByteArray)
    class AccessDenied(val status: Int) : IllegalStateException(if (status == 401) "请先登录一龙" else "记录已撤回，或你已不在此群聊中")
    private data class Entry(val file: File, val meta: JSONObject)
    private val prefix = digest(scope.toByteArray()) + "_"
    fun clear() = synchronized(lock) { generation++; files(root).filter { it.name.startsWith(prefix) }.forEach { it.delete() }; Unit }
    fun bytes(): Long = synchronized(lock) { files(root).filter { it.name.startsWith(prefix) }.sumOf { it.length() } }
    fun fresh(url: String, maxAgeMs: Long = FRESH_MS): File? = synchronized(lock) {
        val entry = entry(url) ?: return@synchronized null
        val age = now() - entry.meta.optLong("verifiedAt", 0)
        if (age !in 0 until maxAgeMs || entry.meta.optLong("verifiedAt", 0) <= 0) return@synchronized null
        touch(entry.file, meta(url)); entry.file
    }
    fun read(url: String, fetch: (String?) -> Download): File {
        val (old, epoch) = synchronized(lock) { entry(url) to generation }
        var response = fetch(old?.meta?.optString("etag")?.takeIf { it.isNotBlank() })
        if (response.status == 304 && old?.file?.isFile != true) response = fetch(null)
        return synchronized(lock) {
            check(epoch == generation) { "缓存已清理，请重新打开记录" }
            if (response.status == 304 && old != null && old.file.isFile) {
                old.meta.put("verifiedAt", now()); atomicWrite(meta(url), old.meta.toString().toByteArray())
                touch(old.file, meta(url)); trim(root, now(), setOf(old.file, meta(url))); return@synchronized old.file
            }
            if (response.status !in 200..299) {
                if (response.status in listOf(401, 403, 404)) { clear(); throw AccessDenied(response.status) }
                error("聊天记录读取失败（${response.status}），请重试")
            }
            require(response.bytes.size <= 12 * 1024 * 1024) { "附件过大" }
            check(root.isDirectory || root.mkdirs()) { "无法创建附件缓存" }
            val hash = digest(response.bytes)
            val file = File(root, "${key(url)}_$hash.bin")
            atomicWrite(file, response.bytes)
            val info = JSONObject().put("hash", hash).put("size", file.length())
                .put("etag", response.etag.orEmpty()).put("verifiedAt", now())
            atomicWrite(meta(url), info.toString().toByteArray()); touch(file, meta(url))
            if (old != null && old.file != file) files(root).filter { it.name.startsWith(old.file.name) }.forEach { it.delete() }
            trim(root, now(), setOf(file, meta(url))); file
        }
    }
    private fun key(url: String) = prefix + digest(url.toByteArray())
    private fun meta(url: String) = File(root, "${key(url)}.json")
    private fun entry(url: String): Entry? = runCatching {
        val info = JSONObject(meta(url).readText()); val hash = info.getString("hash")
        require(hash.matches(Regex("[a-f0-9]{64}")))
        val file = File(root, "${key(url)}_$hash.bin")
        require(file.isFile && file.length() == info.getLong("size"))
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input -> val buffer = ByteArray(16384); while (true) { val n = input.read(buffer); if (n < 0) break; digest.update(buffer, 0, n) } }
        require(hex(digest.digest()) == hash)
        Entry(file, info)
    }.getOrNull()
    private fun touch(vararg files: File) { files.forEach { it.setLastModified(now()) } }
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
        const val FRESH_MS = 5L * 60 * 1000
        const val MAX_BYTES = 128L * 1024 * 1024
        const val RETAIN_MS = 7L * 86400000
        private val lock = Any()
        private var generation = 0L
        private fun files(root: File) = root.listFiles().orEmpty().filter { it.isFile && it.name.matches(Regex("[a-f0-9]{64}_[a-f0-9]{64}.*")) }
        fun clearAll(root: File) = synchronized(lock) { generation++; files(root).forEach { it.delete() }; Unit }
        fun storeDerived(source: File, write: () -> Unit) = synchronized(lock) {
            if (source.isFile) { write(); trim(source.parentFile, keep = setOf(source)) }
        }
        fun totalBytes(root: File): Long = synchronized(lock) { trim(root, System.currentTimeMillis()); files(root).sumOf { it.length() } }
        fun trim(root: File, now: Long = System.currentTimeMillis(), keep: Set<File> = emptySet()) = synchronized(lock) {
            val entries = files(root).groupBy { it.name.take(129) }.values.sortedBy { group -> group.maxOf { it.lastModified() } }
            var size = entries.sumOf { group -> group.sumOf { it.length() } }
            for (group in entries) {
                if (group.any { it in keep }) continue
                if (size > MAX_BYTES || group.maxOf { it.lastModified() } < now - RETAIN_MS) {
                    group.forEach { val bytes = it.length(); if (it.delete()) size -= bytes }
                }
            }
        }
        private fun hex(bytes: ByteArray) = bytes.joinToString("") { "%02x".format(it) }
        private fun digest(bytes: ByteArray) = hex(MessageDigest.getInstance("SHA-256").digest(bytes))
    }
}
