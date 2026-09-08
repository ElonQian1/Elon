package com.elon.app.grid.host

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.net.Uri
import android.os.Bundle

/** Typed local IPC. Each request authenticates the OS caller before inspecting its arguments. */
class BinanceHostProvider : ContentProvider() {
    override fun onCreate() = true
    override fun call(method: String, arg: String?, extras: Bundle?): Bundle {
        val owner = context ?: throw SecurityException("HOST_UNAVAILABLE")
        if (!BinanceHostCaller.ipc(owner)) throw SecurityException("CALLER_REJECTED")
        return runCatching {
            require(arg == null && extras != null)
            val keys = if (method == "detail") setOf("grant", "id") else setOf("grant")
            require(extras.keySet() == keys)
            val token = extras.getString("grant") ?: error("GRANT_MISSING")
            require(Regex("[0-9a-f]{64}").matches(token))
            BinanceHostRuntime.onMain(owner) { runtime ->
                when (method) {
                    "read" -> Bundle().apply { putString("result", runtime.read(token)) }
                    "detail" -> {
                        runtime.detail(token, extras.getString("id") ?: error("GRID_MISSING"))
                        Bundle().apply { putString("status", "pending") }
                    }
                    "revoke" -> { runtime.revoke(token); Bundle().apply { putString("status", "revoked") } }
                    else -> error("METHOD_UNSUPPORTED")
                }
            }
        }.getOrElse { Bundle().apply { putString("error", "HOST_READ_UNAVAILABLE") } }
    }
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = throw SecurityException("UNSUPPORTED")
    override fun insert(uri: Uri, values: ContentValues?): Uri? = throw SecurityException("UNSUPPORTED")
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = throw SecurityException("UNSUPPORTED")
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = throw SecurityException("UNSUPPORTED")
    override fun getType(uri: Uri): String? = null
}
