package com.elon.app.grid.create

import android.content.Context
import android.util.AtomicFile
import java.io.File

/** Crash recovery stays local and outside Android cloud/device backup. */
internal class BinanceCreateJournal(context: Context) {
    private val file = AtomicFile(File(context.noBackupFilesDir, "binance-create-attempt-v1.json"))
    fun read(): String? {
        if (!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists()) return null
        return file.openRead().use {
            val bytes = it.readBytes()
            require(bytes.size <= 2048)
            bytes.toString(Charsets.UTF_8)
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
