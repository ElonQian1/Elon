package com.elon.app

import android.content.Context
import org.json.JSONObject
import java.util.Locale
import java.util.UUID

internal fun mcpAuthorized(headers: Map<String, String>, args: JSONObject, expectedToken: String): Boolean {
    val bearer = headers["authorization"]
        ?.trim()
        ?.takeIf { it.lowercase(Locale.ROOT).startsWith("bearer ") }
        ?.substringAfter(' ')
    val customHeader = headers["x-elon-mcp-token"]?.trim()
    val argToken = args.optString("auth_token").takeIf { it.isNotBlank() }
    return listOfNotNull(bearer, customHeader, argToken).any { constantTimeEquals(it, expectedToken) }
}

internal fun mcpDebugToken(context: Context, prefName: String): String {
    val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
    return prefs.getString(prefName, null)
        ?: UUID.randomUUID().toString().replace("-", "").also {
            prefs.edit().putString(prefName, it).apply()
        }
}

private fun constantTimeEquals(left: String, right: String): Boolean {
    val a = left.toByteArray()
    val b = right.toByteArray()
    if (a.size != b.size) return false
    var diff = 0
    for (index in a.indices) diff = diff or (a[index].toInt() xor b[index].toInt())
    return diff == 0
}

internal fun redactLogLine(line: String): String {
    return line
        .replace(Regex("MCP debug token: [A-Za-z0-9._-]+"), "MCP debug token: <redacted>")
        .replace(Regex("(?i)(auth[_-]?token[\\\"'=:\\s]+)[A-Za-z0-9._-]+")) {
            it.groupValues[1] + "<redacted>"
        }
}
