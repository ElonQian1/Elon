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
            if (method in com.elon.app.grid.create.BinanceCreateCommands.methods) {
                return@runCatching BinanceHostRuntime.onMain(owner) { runtime ->
                    com.elon.app.grid.create.BinanceCreateCommands.dispatch(owner, runtime, method, extras)
                }
            }
            if (method in setOf("resume_v2", "disconnect_v2")) {
                require(extras.isEmpty)
                return@runCatching BinanceHostRuntime.onMain(owner) { runtime ->
                    if (method == "resume_v2") runtime.resume()
                    else { runtime.disconnect(); Bundle().apply { putString("status", "revoked") } }
                }
            }
            val keys = when(method) { "detail" -> setOf("grant", "id"); "report_request_v1" -> setOf("grant", "query"); "report_read_v1" -> setOf("grant", "request"); else -> setOf("grant") }
            require(extras.keySet() == keys)
            val token = extras.getString("grant") ?: error("GRANT_MISSING")
            require(Regex("[0-9a-f]{64}").matches(token))
            BinanceHostRuntime.onMain(owner) { runtime ->
                when (method) {
                    "report_request_v1" -> { runtime.reportRequest(token, extras.getString("query") ?: error("QUERY_MISSING")); Bundle().apply { putString("status", "pending") } }
                    "report_read_v1" -> Bundle().apply { putString("result", runtime.reportRead(token, extras.getString("request") ?: error("REQUEST_MISSING"))) }
                    "read" -> Bundle().apply { putString("result", runtime.read(token)) }
                    "read_v2" -> Bundle().apply { putString("result", runtime.readContinuous(token)) }
                    "refresh" -> { runtime.refresh(token); Bundle().apply { putString("status", "pending") } }
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
