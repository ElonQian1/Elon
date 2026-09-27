package com.elon.app.chatrecords

import java.io.File
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.util.zip.ZipFile

internal data class RecordImport(val document: ChatRecordDocument, val files: Map<String, File>)
internal object WechatArchive {
    const val MAX_ARCHIVE = 64 * 1024 * 1024
    private const val MAX_ASSET = 12 * 1024 * 1024
    fun read(archive: File, output: File): RecordImport {
        require(archive.length() in 1..MAX_ARCHIVE.toLong()) { "ZIP 不能超过 64 MiB" }
        val created = mutableListOf<File>()
        try {
            ZipFile(archive).use { zip ->
                val entries = zip.entries().toList()
                require(entries.size <= 256) { "ZIP 内文件数量过多" }
                val names = mutableSetOf<String>()
                entries.forEach { e ->
                    val name = e.name.replace('\\', '/')
                    require(name.length <= 500 && !name.startsWith('/') && !name.contains(':') && !name.contains('\u0000') &&
                        name.split('/').none { it == ".." || it == "." } && names.add(name)) { "ZIP 包含不安全或重复的路径" }
                }
                val texts = entries.filter { !it.isDirectory && it.name.substringAfterLast('/') == "聊天记录.txt" }
                require(texts.size == 1) { "ZIP 必须包含唯一的聊天记录.txt" }
                var expanded = 0L
                fun bytes(e: java.util.zip.ZipEntry, limit: Int): ByteArray {
                    require(e.size <= limit) { "附件超过大小限制" }
                    require(e.compressedSize <= 0 || e.size <= maxOf(1024 * 1024, e.compressedSize * 250)) { "ZIP 压缩比例异常" }
                    return zip.getInputStream(e).use { input ->
                        val out = java.io.ByteArrayOutputStream()
                        val crc = java.util.zip.CRC32()
                        val buffer = ByteArray(16 * 1024)
                        while (true) {
                            val n = input.read(buffer); if (n < 0) break
                            expanded += n
                            require(out.size() + n <= limit && expanded <= MAX_ARCHIVE) { "ZIP 解压内容超过限制" }
                            out.write(buffer, 0, n); crc.update(buffer, 0, n)
                        }
                        require(e.crc < 0 || crc.value == e.crc) { "ZIP 文件校验失败，请重新从微信导出" }
                        out.toByteArray()
                    }
                }
                val raw = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes(texts.single(), 1024 * 1024))).toString()
                val parsed = WechatTextParser.parse(raw)
                val files = linkedMapOf<String, File>()
                val entryIds = mutableMapOf<String, String>()
                val warnings = parsed.warnings.toMutableList()
                val rows = parsed.messages.map { row ->
                    if (row.filename.isBlank()) return@map row
                    val matches = entries.filter { !it.isDirectory && it != texts.single() &&
                        (it.name == row.filename || it.name.replace('\\', '/').substringAfterLast('/') == row.filename) }
                    if (matches.size != 1) {
                        if (warnings.size < 100) warnings += "${row.filename.take(200)}：${if (matches.isEmpty()) "导出包未包含附件" else "存在同名附件，未自动匹配"}"
                        return@map row
                    }
                    val e = matches.single()
                    val id = entryIds.getOrPut(e.name) {
                        require(files.size < 64) { "每份记录最多包含 64 个附件" }
                        val id = "local_${files.size}"
                        val file = File(output, "record_$id")
                        require(file.canonicalFile.parentFile == output.canonicalFile && !file.exists())
                        created += file; file.writeBytes(bytes(e, MAX_ASSET)); files[id] = file; id
                    }
                    row.copy(assetId = id)
                }
                val document = parsed.copy(messages = rows, warnings = warnings)
                require(document.json().toString().toByteArray(Charsets.UTF_8).size <= 2 * 1024 * 1024) { "聊天记录过大，请减少所选消息后重新分享" }
                return RecordImport(document, files)
            }
        } catch (e: Exception) { created.forEach { it.delete() }; throw e }
    }
}
