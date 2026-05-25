package com.elon.app

import java.io.ByteArrayOutputStream
import java.net.Socket
import java.util.Locale

private const val MCP_MAX_BODY_BYTES = 256 * 1024
private const val MCP_MAX_HEADER_BYTES = 16 * 1024

internal data class HttpRequest(
    val method: String,
    val path: String,
    val headers: Map<String, String>,
    val body: String
)

internal fun readRequest(socket: Socket): HttpRequest? {
    val input = socket.getInputStream()
    val headerBytes = ByteArrayOutputStream()
    val window = java.util.ArrayDeque<Int>(4)
    while (headerBytes.size() <= MCP_MAX_HEADER_BYTES) {
        val byte = input.read()
        if (byte < 0) return null
        headerBytes.write(byte)
        window.addLast(byte)
        if (window.size > 4) window.removeFirst()
        if (window.size == 4 && window.toList() == listOf(13, 10, 13, 10)) break
    }
    val headerText = String(headerBytes.toByteArray(), Charsets.UTF_8)
    val lines = headerText.split("\r\n").filter { it.isNotBlank() }
    val requestLine = lines.firstOrNull() ?: return null
    val parts = requestLine.split(' ')
    if (parts.size < 2) return null
    val headers = mutableMapOf<String, String>()
    for (line in lines.drop(1)) {
        val separator = line.indexOf(':')
        if (separator > 0) {
            headers[line.substring(0, separator).trim().lowercase(Locale.ROOT)] =
                line.substring(separator + 1).trim()
        }
    }
    val length = headers["content-length"]?.toIntOrNull()?.coerceAtMost(MCP_MAX_BODY_BYTES) ?: 0
    val bodyBytes = ByteArray(length)
    var offset = 0
    while (offset < length) {
        val read = input.read(bodyBytes, offset, length - offset)
        if (read < 0) break
        offset += read
    }
    return HttpRequest(
        method = parts[0].uppercase(Locale.ROOT),
        path = parts[1].substringBefore('?'),
        headers = headers,
        body = String(bodyBytes.copyOf(offset), Charsets.UTF_8)
    )
}

internal fun writeHttpResponse(
    socket: Socket,
    status: Int,
    reason: String,
    body: String,
    protocolVersion: String,
    contentType: String = "application/json; charset=utf-8"
) {
    val bodyBytes = body.toByteArray(Charsets.UTF_8)
    val headers = buildString {
        append("HTTP/1.1 ").append(status).append(' ').append(reason).append("\r\n")
        append("Content-Type: ").append(contentType).append("\r\n")
        append("Content-Length: ").append(bodyBytes.size).append("\r\n")
        append("Connection: close\r\n")
        append("MCP-Protocol-Version: ").append(protocolVersion).append("\r\n")
        append("\r\n")
    }
    socket.getOutputStream().use { output ->
        output.write(headers.toByteArray(Charsets.UTF_8))
        if (bodyBytes.isNotEmpty()) output.write(bodyBytes)
        output.flush()
    }
}
