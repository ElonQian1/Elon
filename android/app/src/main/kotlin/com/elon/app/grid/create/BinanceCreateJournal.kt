package com.elon.app.grid.create

import android.content.Context
import android.util.AtomicFile
import java.io.File

/** Crash recovery stays local and outside Android cloud/device backup. */
internal class BinanceCreateJournal(context: Context, name: String = "binance-create-attempt-v1.json") {
    init { require(name in setOf("binance-create-attempt-v1.json", "binance-manage-attempt-v1.json")) }
    private val file = AtomicFile(File(context.noBackupFilesDir, name))
    fun read(): String? {
        if (!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists()) return null
        return file.openRead().use {
            val bytes = ByteArray(2049)
            var size = 0
            while (size < bytes.size) { val n = it.read(bytes,size,bytes.size-size); if(n < 0) break; size += n }
            require(size <= 2048)
            bytes.copyOf(size).toString(Charsets.UTF_8)
        }
    }
    fun save(raw: String?): Boolean = runCatching {
        if (raw == null) {
            file.delete(); require(!file.baseFile.exists()); return@runCatching true
        }
        val bytes = raw.toByteArray(Charsets.UTF_8); require(bytes.size <= 2048)
        val stream = file.startWrite()
        try { stream.write(bytes); file.finishWrite(stream) }
        catch (error: Exception) { file.failWrite(stream); throw error }
        true
    }.getOrDefault(false)
}
