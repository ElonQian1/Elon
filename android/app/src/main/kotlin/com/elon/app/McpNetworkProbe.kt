package com.elon.app

import android.content.Context
import android.os.SystemClock
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.InetSocketAddress
import java.net.Socket
import java.net.URL

internal fun mcpNetworkCheckJson(
    context: Context,
    args: JSONObject,
    defaultServerBaseUrl: String
): JSONObject {
    val urls = urlsFromArgs(args, defaultServerBaseUrl)
    val tcpHost = args.optString("tcp_host").takeIf { it.isNotBlank() } ?: "43.139.149.158"
    val tcpPort = args.optInt("tcp_port", 8080).takeIf { it in 1..65535 } ?: 8080
    return JSONObject()
        .put("network", networkCapabilitiesJson(context))
        .put("tcp_probe", tcpProbe(tcpHost, tcpPort))
        .put("http_probes", JSONArray().apply { urls.forEach { put(httpProbe(it)) } })
}

private fun urlsFromArgs(args: JSONObject, defaultServerBaseUrl: String): List<String> {
    val array = args.optJSONArray("urls")
    if (array != null && array.length() > 0) {
        return buildList {
            for (index in 0 until array.length()) {
                array.optString(index).takeIf { it.startsWith("http://") || it.startsWith("https://") }?.let { add(it) }
            }
        }.ifEmpty { defaultProbeUrls(defaultServerBaseUrl) }
    }
    return defaultProbeUrls(defaultServerBaseUrl)
}

private fun defaultProbeUrls(defaultServerBaseUrl: String) = listOf(
    "${defaultServerBaseUrl.trimEnd('/')}/health",
    "${defaultServerBaseUrl.trimEnd('/')}/app/version.json"
)

private fun tcpProbe(host: String, port: Int): JSONObject {
    val started = SystemClock.elapsedRealtime()
    return runCatching {
        Socket().use { socket ->
            socket.connect(InetSocketAddress(host, port), 5_000)
        }
        JSONObject()
            .put("host", host)
            .put("port", port)
            .put("ok", true)
            .put("duration_ms", SystemClock.elapsedRealtime() - started)
    }.getOrElse { error ->
        JSONObject()
            .put("host", host)
            .put("port", port)
            .put("ok", false)
            .put("duration_ms", SystemClock.elapsedRealtime() - started)
            .put("error", error.message ?: error.javaClass.simpleName)
    }
}

private fun httpProbe(url: String): JSONObject {
    val started = SystemClock.elapsedRealtime()
    return runCatching {
        val connection = (URL(url).openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            connectTimeout = 5_000
            readTimeout = 5_000
        }
        try {
            val code = connection.responseCode
            val stream = if (code in 200..299) connection.inputStream else connection.errorStream
            val preview = stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText().take(512) }
            JSONObject()
                .put("url", url)
                .put("ok", code in 200..299)
                .put("status_code", code)
                .put("duration_ms", SystemClock.elapsedRealtime() - started)
                .put("content_type", connection.contentType ?: JSONObject.NULL)
                .put("body_preview", preview ?: JSONObject.NULL)
        } finally {
            connection.disconnect()
        }
    }.getOrElse { error ->
        JSONObject()
            .put("url", url)
            .put("ok", false)
            .put("duration_ms", SystemClock.elapsedRealtime() - started)
            .put("error", error.message ?: error.javaClass.simpleName)
    }
}

internal fun fetchJson(url: String): JSONObject? {
    return runCatching {
        val connection = (URL(url).openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            connectTimeout = 5_000
            readTimeout = 5_000
        }
        try {
            if (connection.responseCode !in 200..299) return@runCatching null
            val body = connection.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
            JSONObject(body)
        } finally {
            connection.disconnect()
        }
    }.getOrNull()
}
