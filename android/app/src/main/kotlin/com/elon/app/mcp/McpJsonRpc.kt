package com.elon.app.mcp

import org.json.JSONArray
import org.json.JSONObject

internal fun rpcResult(id: Any, result: JSONObject): JSONObject {
    return JSONObject()
        .put("jsonrpc", "2.0")
        .put("id", id)
        .put("result", result)
}

internal fun rpcError(id: Any?, code: Int, message: String): JSONObject {
    return JSONObject()
        .put("jsonrpc", "2.0")
        .put("id", id ?: JSONObject.NULL)
        .put("error", JSONObject().put("code", code).put("message", message))
}

internal fun toolResult(message: String, structured: JSONObject, isError: Boolean = false): JSONObject {
    return JSONObject()
        .put(
            "content",
            JSONArray().put(JSONObject().put("type", "text").put("text", message))
        )
        .put("structuredContent", structured)
        .put("isError", isError)
}

internal fun jsonError(code: String, message: String): String {
    return JSONObject().put("error", code).put("message", message).toString()
}
