package com.elon.app.exchange.okx

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.net.Uri
import android.os.Bundle
import com.elon.app.grid.host.BinanceHostCaller

class OkxReadProvider : ContentProvider() {
    override fun onCreate() = true
    override fun call(method: String, arg: String?, extras: Bundle?): Bundle {
        val context = context ?: throw SecurityException("HOST_UNAVAILABLE")
        if (!BinanceHostCaller.ipc(context)) throw SecurityException("CALLER_REJECTED")
        return try {
            require(arg == null)
            val data = extras ?: Bundle()
            val keys = when (method) {
                "capabilities_v1", "capabilities_v2", "capabilities_v3", "balance_capabilities_v1", "resume_v1" -> emptySet()
                "read_v1", "revoke_v1", "balance_v1" -> setOf("grant")
                "detail_v1" -> setOf("grant", "id")
                "history_v1" -> setOf("grant", "after")
                "records_v1" -> setOf("grant","kind","id","symbol","after")
                else -> throw IllegalArgumentException("METHOD_UNSUPPORTED")
            }
            require(data.keySet() == keys)
            val grant = if ("grant" in keys) data.getString("grant")?.also { require(Regex("[a-f0-9]{64}").matches(it)) } ?: error("GRANT_MISSING") else ""
            val host = OkxReadHost.get(context)
            Bundle().apply {
                putString("schema", OkxReadProtocol.SCHEMA)
                when (method) {
                    "balance_capabilities_v1" -> { putString("status", "supported"); putString("balance_schema", OkxBalance.SCHEMA) }
                    "balance_v1" -> putString("result", host.balance(grant))
                    "capabilities_v1" -> { putString("status", "supported"); putString("environment", "live") }
                    "capabilities_v2" -> { putString("status", "supported"); putString("environment", "live"); putString("history_schema", OkxHistoryPage.SCHEMA) }
                    "capabilities_v3" -> {putString("status","supported");putString("environment","live");putString("history_schema",OkxHistoryPage.SCHEMA);putString("records_schema",OkxRecordsPage.SCHEMA)}
                    "resume_v1" -> putString("grant", host.resume())
                    "revoke_v1" -> { host.revoke(grant); putString("status", "revoked") }
                    "history_v1" -> putString("result", host.history(grant, data.getString("after") ?: error("AFTER_MISSING")))
                    "records_v1" -> putString("result",host.records(grant,data.getString("kind") ?: error("KIND_MISSING"),data.getString("id") ?: error("ID_MISSING"),data.getString("symbol") ?: error("SYMBOL_MISSING"),data.getString("after") ?: error("AFTER_MISSING")))
                    else -> putString("result", host.read(grant, if (method == "detail_v1") data.getString("id") ?: error("ID_MISSING") else null))
                }
            }
        } catch (error: Exception) {
            Bundle().apply { putString("error", (error as? OkxReadException)?.reason?.name ?: "INVALID_RESPONSE") }
        }
    }
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = throw SecurityException("UNSUPPORTED")
    override fun insert(uri: Uri, values: ContentValues?): Uri? = throw SecurityException("UNSUPPORTED")
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = throw SecurityException("UNSUPPORTED")
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = throw SecurityException("UNSUPPORTED")
    override fun getType(uri: Uri): String? = null
}
