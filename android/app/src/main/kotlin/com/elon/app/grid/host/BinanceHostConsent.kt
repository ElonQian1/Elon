package com.elon.app.grid.host

import android.content.Context
import com.elon.app.privateaccess.StrictJson

/** Remembered permission, not a credential or a cached identity proof. */
internal class BinanceHostConsent(context: Context) {
    private val file = android.util.AtomicFile(java.io.File(context.noBackupFilesDir, "binance-read-consent-v2.json"))
    private fun read(): String? = runCatching {
        require(file.baseFile.length() in 1..2048)
        file.openRead().use { String(it.readBytes(), Charsets.UTF_8) }
    }.getOrNull()
    fun recorded() = read() != null
    fun permits(owner: String?, account: String?, kind: String): Boolean =
        matches(read(), owner, account, kind)
    fun approve(owner: String, account: String, kind: String) {
        val raw = encode(owner, account, kind)
        val output = file.startWrite()
        try { output.write(raw.toByteArray(Charsets.UTF_8)); file.finishWrite(output) }
        catch (error: Exception) { file.failWrite(output); throw error }
    }
    fun clear() { file.delete(); check(!file.baseFile.exists()) }

    companion object {
        fun encode(owner: String, account: String, kind: String): String {
            require(listOf(owner, account).all { Regex("[0-9a-f]{64}").matches(it) })
            require(kind in setOf("primary", "sub", "unknown"))
            return StrictJson.encode(mapOf("schema" to "binance.read.consent.v2", "owner" to owner,
                "account" to account, "kind" to kind, "consumer" to "com.elon.quant", "purpose" to "grid.read"))
        }
        fun matches(raw: String?, owner: String?, account: String?, kind: String): Boolean =
            if (raw == null || owner == null || account == null) false
            else runCatching { raw == encode(owner, account, kind) }.getOrDefault(false)
    }
}
